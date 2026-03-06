/**
 * face_service.js — Face Detection & pHash Sidecar
 * -------------------------------------------------
 * Runs as a long-lived HTTP server on localhost:3001.
 * Called by Rust (evidence_service.rs) after every target photo upload.
 *
 * POST /analyze
 *   Body : { image_base64: string, mime_type: string }
 *   Reply: FaceAnalysisResult (see bottom of file for shape)
 *
 * GET /health
 *   Reply: { status: "ok", models_loaded: true }
 *
 * Start with:  node face_service.js
 * Or via PM2:  pm2 start face_service.js --name face-service
 *
 * Dependencies (package.json supplied separately):
 *   @vladmandic/face-api   — face-api.js maintained fork (works in Node)
 *   @tensorflow/tfjs-node  — TF.js Node backend (uses native bindings)
 *   canvas                 — Node.js canvas for face-api image processing
 *   sharp                  — fast image decoding / resizing
 */

'use strict';

const http    = require('http');
const path    = require('path');
const fs      = require('fs');

// ─── Lazy-loaded after models are ready ──────────────────────────────────────
let faceapi = null;
let canvas  = null;
let tf      = null;

// ─── Config ──────────────────────────────────────────────────────────────────
const PORT          = parseInt(process.env.FACE_SERVICE_PORT  || '3001', 10);
const HOST          = process.env.FACE_SERVICE_HOST            || '127.0.0.1';
// Models directory — place the three weight folders here
const MODELS_PATH   = process.env.FACE_MODELS_PATH
                        || path.join(__dirname, 'models');
// Euclidean distance threshold for "same person"
const DEFAULT_THRESHOLD = parseFloat(process.env.FACE_THRESHOLD || '0.55');
// Maximum image dimension before resizing (keeps inference fast)
const MAX_DIM       = parseInt(process.env.FACE_MAX_DIM || '640', 10);

// ─── Startup state ───────────────────────────────────────────────────────────
let modelsLoaded    = false;
let startupError    = null;

// ─────────────────────────────────────────────────────────────────────────────
// 1. LOAD MODELS
// ─────────────────────────────────────────────────────────────────────────────
async function loadModels() {
    console.log('🔄 Loading TensorFlow.js and face-api.js...');

    // These require() calls are deferred so the HTTP server can start
    // (and return 503 health checks) while models are loading.
    tf      = require('@tensorflow/tfjs-node');
    canvas  = require('canvas');
    faceapi = require('@vladmandic/face-api/dist/face-api.node.js');

    // Patch face-api to use the Node canvas implementation
    const { Canvas, Image, ImageData } = canvas;
    faceapi.env.monkeyPatch({ Canvas, Image, ImageData });

    // Verify the models directory exists
    if (!fs.existsSync(MODELS_PATH)) {
        throw new Error(
            `Models directory not found: ${MODELS_PATH}\n` +
            `Run: node download_models.js   to fetch them.`
        );
    }

    console.log(`📂 Loading models from: ${MODELS_PATH}`);

    // We need three models:
    //  1. SsdMobilenetv1       — detects face bounding boxes
    //  2. FaceLandmark68Net    — 68 facial landmark points
    //  3. FaceRecognitionNet   — 128-dim descriptor (the "encoding")
    await Promise.all([
        faceapi.nets.ssdMobilenetv1.loadFromDisk(MODELS_PATH),
        faceapi.nets.faceLandmark68Net.loadFromDisk(MODELS_PATH),
        faceapi.nets.faceRecognitionNet.loadFromDisk(MODELS_PATH),
    ]);

    modelsLoaded = true;
    console.log('✅ Face-api.js models loaded and ready');
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. pHASH  (perceptual hash — 64-bit DCT hash, 16-char hex)
// ─────────────────────────────────────────────────────────────────────────────
// Pure JS implementation — no extra dependency.
// Two images are "similar" if their Hamming distance is ≤ 10 bits.

async function computePHash(imageBuffer) {
    // We use sharp (loaded lazily) to resize to 32×32 greyscale
    const sharp = require('sharp');

    const SIZE = 32;
    const DCT_SIZE = 8; // use top-left 8×8 DCT coefficients

    // Resize to 32×32 greyscale
    const raw = await sharp(imageBuffer)
        .resize(SIZE, SIZE, { fit: 'fill' })
        .grayscale()
        .raw()
        .toBuffer();

    // Build SIZE×SIZE pixel grid (values 0–255)
    const pixels = new Float64Array(SIZE * SIZE);
    for (let i = 0; i < pixels.length; i++) {
        pixels[i] = raw[i];
    }

    // 2-D DCT (separable — row then column pass)
    const dct = dct2d(pixels, SIZE);

    // Extract top-left 8×8 block (exclude DC component at [0,0])
    const vals = [];
    for (let y = 0; y < DCT_SIZE; y++) {
        for (let x = 0; x < DCT_SIZE; x++) {
            if (x === 0 && y === 0) continue;
            vals.push(dct[y * SIZE + x]);
        }
    }

    // Compute median
    const sorted = [...vals].sort((a, b) => a - b);
    const median = sorted[Math.floor(sorted.length / 2)];

    // Build 64-bit hash: 1 if value > median, else 0
    let hash = BigInt(0);
    for (let i = 0; i < 64; i++) {
        if (vals[i] > median) {
            hash |= BigInt(1) << BigInt(63 - i);
        }
    }

    // Return as 16-char hex string (zero-padded)
    return hash.toString(16).padStart(16, '0');
}

/** Compute 2-D DCT of a flat SIZE×SIZE Float64Array in-place. */
function dct2d(pixels, size) {
    const out = new Float64Array(size * size);

    // Row DCT
    for (let y = 0; y < size; y++) {
        const row = pixels.slice(y * size, (y + 1) * size);
        const d   = dct1d(row);
        for (let x = 0; x < size; x++) out[y * size + x] = d[x];
    }

    // Column DCT
    for (let x = 0; x < size; x++) {
        const col = new Float64Array(size);
        for (let y = 0; y < size; y++) col[y] = out[y * size + x];
        const d = dct1d(col);
        for (let y = 0; y < size; y++) out[y * size + x] = d[y];
    }

    return out;
}

/** Naive 1-D DCT-II of length N. O(N²) — fine for N=32. */
function dct1d(signal) {
    const N   = signal.length;
    const out = new Float64Array(N);
    for (let k = 0; k < N; k++) {
        let sum = 0;
        for (let n = 0; n < N; n++) {
            sum += signal[n] * Math.cos((Math.PI / N) * (n + 0.5) * k);
        }
        out[k] = sum;
    }
    return out;
}

/** Hamming distance between two 16-char hex pHash strings. */
function hammingDistance(a, b) {
    const ai = BigInt('0x' + a);
    const bi = BigInt('0x' + b);
    let xor  = ai ^ bi;
    let dist = 0;
    while (xor > BigInt(0)) {
        dist += Number(xor & BigInt(1));
        xor >>= BigInt(1);
    }
    return dist;
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. FACE ANALYSIS  (the main workhorse)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Analyse a single image buffer.
 *
 * Returns FaceAnalysisResult:
 * {
 *   faces: FaceResult[],    // one per detected face, sorted largest→smallest
 *   phash: string,          // 16-char hex pHash of the full image
 *   face_count: number,
 *   processing_ms: number,
 *   image_width: number,
 *   image_height: number,
 * }
 *
 * FaceResult:
 * {
 *   face_index: number,         // 0 = largest face
 *   descriptor: number[],       // 128 floats  (the "encoding")
 *   detection_score: number,    // 0.0–1.0
 *   box: { x, y, width, height },
 *   is_largest: boolean,
 * }
 */
async function analyzeImage(imageBuffer) {
    const t0 = Date.now();

    // Resize if needed (keeps inference under ~100 ms on most hardware)
    const sharp = require('sharp');
    const meta  = await sharp(imageBuffer).metadata();
    let buf = imageBuffer;

    if ((meta.width || 0) > MAX_DIM || (meta.height || 0) > MAX_DIM) {
        buf = await sharp(imageBuffer)
            .resize(MAX_DIM, MAX_DIM, { fit: 'inside', withoutEnlargement: true })
            .toBuffer();
    }

    // Compute pHash on the (possibly resized) buffer — always, even if no face
    const phash = await computePHash(buf);

    // Load image into canvas
    const img = await canvas.loadImage(buf);

    // Detect all faces with landmarks + descriptors
    // minConfidence 0.5 avoids very weak detections
    const detections = await faceapi
        .detectAllFaces(img, new faceapi.SsdMobilenetv1Options({ minConfidence: 0.5 }))
        .withFaceLandmarks()
        .withFaceDescriptors();

    // Sort by box area descending (largest face = most likely the target)
    const sorted = [...detections].sort((a, b) => {
        const areaA = a.detection.box.width * a.detection.box.height;
        const areaB = b.detection.box.width * b.detection.box.height;
        return areaB - areaA;
    });

    const faces = sorted.map((det, idx) => ({
        face_index:      idx,
        descriptor:      Array.from(det.descriptor), // Float32Array → plain array
        detection_score: det.detection.score,
        box: {
            x:      Math.round(det.detection.box.x),
            y:      Math.round(det.detection.box.y),
            width:  Math.round(det.detection.box.width),
            height: Math.round(det.detection.box.height),
        },
        is_largest: idx === 0,
    }));

    return {
        faces,
        phash,
        face_count:     faces.length,
        processing_ms:  Date.now() - t0,
        image_width:    img.width,
        image_height:   img.height,
    };
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. HTTP SERVER
// ─────────────────────────────────────────────────────────────────────────────

function readBody(req) {
    return new Promise((resolve, reject) => {
        const chunks = [];
        req.on('data', chunk => chunks.push(chunk));
        req.on('end',  ()    => resolve(Buffer.concat(chunks)));
        req.on('error', reject);
    });
}

function sendJSON(res, statusCode, body) {
    const payload = JSON.stringify(body);
    res.writeHead(statusCode, {
        'Content-Type':   'application/json',
        'Content-Length': Buffer.byteLength(payload),
    });
    res.end(payload);
}

const server = http.createServer(async (req, res) => {
    // ── GET /health ──────────────────────────────────────────────────────────
    if (req.method === 'GET' && req.url === '/health') {
        if (startupError) {
            return sendJSON(res, 503, {
                status:       'error',
                models_loaded: false,
                error:        startupError.message,
            });
        }
        return sendJSON(res, modelsLoaded ? 200 : 503, {
            status:        modelsLoaded ? 'ok' : 'loading',
            models_loaded: modelsLoaded,
        });
    }

    // ── POST /analyze ────────────────────────────────────────────────────────
    if (req.method === 'POST' && req.url === '/analyze') {
        if (!modelsLoaded) {
            return sendJSON(res, 503, {
                success: false,
                error:   startupError
                    ? `Model load failed: ${startupError.message}`
                    : 'Models not ready yet — retry in a moment',
            });
        }

        let body;
        try {
            const raw  = await readBody(req);
            body       = JSON.parse(raw.toString('utf8'));
        } catch (e) {
            return sendJSON(res, 400, { success: false, error: 'Invalid JSON body' });
        }

        const { image_base64, mime_type } = body;

        if (!image_base64 || typeof image_base64 !== 'string') {
            return sendJSON(res, 400, { success: false, error: 'Missing image_base64' });
        }

        let imageBuffer;
        try {
            imageBuffer = Buffer.from(image_base64, 'base64');
        } catch (e) {
            return sendJSON(res, 400, { success: false, error: 'Invalid base64 data' });
        }

        if (imageBuffer.length < 100) {
            return sendJSON(res, 400, { success: false, error: 'Image data too small' });
        }

        try {
            const result = await analyzeImage(imageBuffer);
            console.log(
                `📸 Analyzed image: ${result.face_count} face(s) found` +
                ` | pHash: ${result.phash}` +
                ` | ${result.processing_ms}ms`
            );
            return sendJSON(res, 200, { success: true, ...result });
        } catch (e) {
            console.error('❌ Analysis error:', e);
            return sendJSON(res, 500, {
                success: false,
                error:   `Analysis failed: ${e.message}`,
            });
        }
    }

    // ── 404 ──────────────────────────────────────────────────────────────────
    sendJSON(res, 404, { error: 'Not found' });
});

// ─────────────────────────────────────────────────────────────────────────────
// 5. STARTUP
// ─────────────────────────────────────────────────────────────────────────────
server.listen(PORT, HOST, () => {
    console.log(`🚀 Face service listening on http://${HOST}:${PORT}`);
    console.log(`   Models path : ${MODELS_PATH}`);
    console.log(`   Threshold   : ${DEFAULT_THRESHOLD}`);
    console.log(`   Max image dim: ${MAX_DIM}px`);
});

// Load models asynchronously — server accepts /health while this runs
loadModels().catch(err => {
    console.error('❌ Failed to load face-api models:', err.message);
    startupError = err;
});

// Graceful shutdown
process.on('SIGTERM', () => {
    console.log('🛑 SIGTERM received — shutting down gracefully');
    server.close(() => process.exit(0));
});

process.on('SIGINT', () => {
    console.log('🛑 SIGINT received — shutting down');
    server.close(() => process.exit(0));
});
