/**
 * download_models.js — One-time model weight downloader
 * -------------------------------------------------------
 * Run once before starting face_service.js:
 *   node download_models.js
 *
 * Downloads three face-api.js model families into ./models/
 * Total size: ~6.4 MB
 * Source: official vladmandic/face-api CDN (jsDelivr)
 */

'use strict';

const https = require('https');
const http  = require('http');
const fs    = require('fs');
const path  = require('path');

const MODELS_DIR = path.join(__dirname, 'models');
const BASE_URL   = 'https://cdn.jsdelivr.net/npm/@vladmandic/face-api/model';

// Every file we need — three model families, each with a manifest + weight shard(s)
const MODEL_FILES = [
    // ── SSD MobileNet v1 (face detection) ──────────────────────────────────
    'ssd_mobilenetv1_model-weights_manifest.json',
    'ssd_mobilenetv1_model-shard1',
    'ssd_mobilenetv1_model-shard2',

    // ── Face Landmark 68-point model ────────────────────────────────────────
    'face_landmark_68_model-weights_manifest.json',
    'face_landmark_68_model-shard1',

    // ── Face Recognition model (128-dim descriptor) ─────────────────────────
    'face_recognition_model-weights_manifest.json',
    'face_recognition_model-shard1',
    'face_recognition_model-shard2',
];

function download(url, destPath) {
    return new Promise((resolve, reject) => {
        const file    = fs.createWriteStream(destPath);
        const client  = url.startsWith('https') ? https : http;

        const req = client.get(url, (res) => {
            // Follow redirects (jsDelivr uses them)
            if (res.statusCode === 301 || res.statusCode === 302) {
                file.close();
                fs.unlinkSync(destPath);
                return download(res.headers.location, destPath)
                    .then(resolve).catch(reject);
            }

            if (res.statusCode !== 200) {
                file.close();
                fs.unlinkSync(destPath);
                return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
            }

            const total = parseInt(res.headers['content-length'] || '0', 10);
            let   received = 0;

            res.on('data', (chunk) => {
                received += chunk.length;
                if (total > 0) {
                    const pct = ((received / total) * 100).toFixed(0);
                    process.stdout.write(`\r   ${path.basename(destPath).padEnd(55)} ${pct}%`);
                }
            });

            res.pipe(file);
            file.on('finish', () => {
                file.close();
                process.stdout.write('\n');
                resolve();
            });
        });

        req.on('error', (err) => {
            file.close();
            if (fs.existsSync(destPath)) fs.unlinkSync(destPath);
            reject(err);
        });

        req.setTimeout(30_000, () => {
            req.destroy();
            reject(new Error(`Timeout downloading ${url}`));
        });
    });
}

async function main() {
    console.log('📥 Downloading face-api.js model weights...\n');

    if (!fs.existsSync(MODELS_DIR)) {
        fs.mkdirSync(MODELS_DIR, { recursive: true });
        console.log(`   Created: ${MODELS_DIR}\n`);
    }

    let downloaded = 0;
    let skipped    = 0;

    for (const filename of MODEL_FILES) {
        const destPath = path.join(MODELS_DIR, filename);

        if (fs.existsSync(destPath) && fs.statSync(destPath).size > 0) {
            console.log(`   ✓ Already exists: ${filename}`);
            skipped++;
            continue;
        }

        const url = `${BASE_URL}/${filename}`;
        console.log(`   ↓ Downloading: ${filename}`);

        try {
            await download(url, destPath);
            downloaded++;
        } catch (err) {
            console.error(`\n   ❌ Failed: ${filename} — ${err.message}`);
            process.exit(1);
        }
    }

    console.log(`\n✅ Done! ${downloaded} downloaded, ${skipped} already present.`);
    console.log(`   Models saved to: ${MODELS_DIR}`);
    console.log(`\n   You can now start the face service:`);
    console.log(`   node face_service.js\n`);
}

main().catch((err) => {
    console.error('Fatal:', err);
    process.exit(1);
});
