/**
 * frame-extractor.js  v3  ── UPLOAD-PAGE EDITION
 * ─────────────────────────────────────────────────────────────────────────────
 * PURPOSE
 *   Automatically extract the best person/face frames from any video file
 *   the user adds on the evidence upload page, BEFORE the evidence is submitted.
 *   Selected frames are uploaded as target_photo_N files alongside the evidence.
 *
 * DETECTION PRIORITY
 *   1. Face detected (face-api.js TinyFaceDetector) AND face is large enough
 *      to be recognisable (≥ 60 px wide at 480×270) → primary candidates.
 *   2. Person detected (COCO-SSD) but no usable face found → fallback,
 *      lower score so they appear after face frames in the grid.
 *   Frames with no person at all are discarded entirely.
 *
 * SCORING
 *   face_score  = detection_confidence × (face_width / canvas_width)^0.5 × 2
 *   person_score = coco_confidence × 0.35
 *   Frames within 2 s of a higher-scoring frame are de-duplicated.
 *
 * PUBLIC API (window.FrameExtractor)
 *   openForFile(file)       – open the modal for a specific File object
 *   getSelectedBlobs()      – returns Array<{blob, filename, ts}> for selected frames
 *   clearSelected()         – resets the selection (call after form submit)
 * ─────────────────────────────────────────────────────────────────────────────
 */

(function () {
  'use strict';

  /* ═══════════════════════════════════════════════════════════════════════
     CONFIG
  ═══════════════════════════════════════════════════════════════════════ */
  const CFG = {
    interval:      2,          // seconds between sampled timestamps
    maxStamps:     120,        // max timestamps to scan per video
    captureW:      480,
    captureH:      270,
    minFaceW:      55,         // px – minimum face width to count as "recognisable"
    faceScoreMul:  2.0,        // face frames get this multiplier over person frames
    personScoreMul:0.35,       // fallback person-only frame score ceiling
    dedupWindow:   2.0,        // seconds – suppress near-duplicate frames
    maxResults:    40,         // maximum frames shown in grid
    jpegQuality:   0.88,

    // CDN URLs
    tfUrl:     'https://cdn.jsdelivr.net/npm/@tensorflow/tfjs@4.11.0/dist/tf.min.js',
    cocoUrl:   'https://cdn.jsdelivr.net/npm/@tensorflow-models/coco-ssd@2.2.3/dist/coco-ssd.min.js',
    faceApiUrl:'https://cdn.jsdelivr.net/npm/@vladmandic/face-api/dist/face-api.js',
    faceModelUrl: 'https://cdn.jsdelivr.net/npm/@vladmandic/face-api/model',
  };

  /* ═══════════════════════════════════════════════════════════════════════
     STATE
  ═══════════════════════════════════════════════════════════════════════ */
  let cocoModel  = null;   // COCO-SSD model instance
  let faceReady  = false;  // whether face-api TinyFaceDetector is loaded
  let frames     = [];     // { dataUrl, blob, ts, score, hasFace }
  let selected   = new Set();
  let busy       = false;
  let currentObjectUrl = null;

  /* ═══════════════════════════════════════════════════════════════════════
     MODAL INJECTION  (only once per page load)
  ═══════════════════════════════════════════════════════════════════════ */
  function ensureModal() {
    if (document.getElementById('fe-modal')) return;

    const modal = document.createElement('div');
    modal.id = 'fe-modal';
    modal.className = 'fixed inset-0 z-[9999] bg-black/85 flex items-center justify-center p-4 hidden';
    modal.innerHTML = /* html */`
<div class="relative bg-gray-950 rounded-2xl border border-purple-600/70 w-full max-w-5xl
            max-h-[92vh] flex flex-col overflow-hidden shadow-2xl shadow-purple-900/30">

  <!-- ── Header ─────────────────────────────────────────────────────── -->
  <div class="flex items-center justify-between px-5 py-4 border-b border-gray-800 shrink-0">
    <div class="flex items-center gap-3">
      <div class="w-9 h-9 rounded-xl bg-purple-600/20 border border-purple-600/40
                  flex items-center justify-center text-lg text-purple-400">🎯</div>
      <div>
        <h3 class="text-sm font-bold text-purple-300 tracking-wide">Target Person Extraction</h3>
        <p class="text-xs text-gray-500 mt-0.5">
          AI-powered face &amp; person detection — select frames to upload as targets
        </p>
      </div>
    </div>
    <div class="flex items-center gap-3">
      <span id="fe-ai-badge"
        class="inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full
               bg-yellow-500/10 border border-yellow-500/20 text-yellow-400">
        <span class="w-1.5 h-1.5 rounded-full bg-yellow-400 animate-pulse"></span>
        Loading AI…
      </span>
      <button onclick="FrameExtractor._close()"
        class="w-8 h-8 rounded-lg bg-gray-800 hover:bg-gray-700 flex items-center
               justify-center text-gray-400 hover:text-white transition-colors text-lg">✕</button>
    </div>
  </div>

  <!-- ── Diagnostic bar ─────────────────────────────────────────────── -->
  <div id="fe-diag"
    class="mx-5 mt-4 text-xs px-3 py-2 rounded-lg border
           bg-yellow-500/10 border-yellow-500/30 text-yellow-300 shrink-0">
    🔄 Initialising detection models…
  </div>

  <!-- ── Controls row ───────────────────────────────────────────────── -->
  <div class="flex flex-wrap items-center gap-3 px-5 pt-3 pb-2 shrink-0">
    <span id="fe-duration" class="text-xs text-gray-500"></span>

    <div class="flex items-center gap-2 ml-auto flex-wrap gap-y-2">
      <label class="text-xs text-gray-400 flex items-center gap-1.5">
        Scan every
        <select id="fe-interval"
          class="text-xs bg-gray-800 border border-gray-700 rounded px-2 py-1 text-white
                 focus:outline-none focus:border-purple-500">
          <option value="1">1 sec</option>
          <option value="2" selected>2 sec</option>
          <option value="3">3 sec</option>
          <option value="5">5 sec</option>
        </select>
      </label>
      <button id="fe-extract-btn" disabled
        onclick="FrameExtractor.startExtraction()"
        class="inline-flex items-center gap-1.5 px-4 py-1.5 rounded-lg text-xs font-bold
               bg-purple-600 hover:bg-purple-700 text-white transition-colors
               disabled:opacity-40 disabled:cursor-not-allowed">
        ▶ Scan for People
      </button>
    </div>
  </div>

  <!-- ── Progress ───────────────────────────────────────────────────── -->
  <div id="fe-progress-wrap" class="hidden px-5 pb-3 shrink-0">
    <div class="flex justify-between text-xs text-gray-400 mb-1.5">
      <span id="fe-progress-label">Scanning…</span>
      <span id="fe-progress-pct">0%</span>
    </div>
    <div class="w-full bg-gray-800 rounded-full h-2 overflow-hidden">
      <div id="fe-progress-bar"
        class="h-2 rounded-full bg-gradient-to-r from-purple-600 to-pink-500 transition-all duration-150"
        style="width:0%"></div>
    </div>
  </div>

  <!-- ── Frame grid ─────────────────────────────────────────────────── -->
  <div class="flex-1 overflow-y-auto px-5 pb-4 min-h-0">

    <!-- Empty / hint state -->
    <div id="fe-empty-hint" class="py-12 text-center text-gray-600 text-sm">
      <div class="text-4xl mb-3 opacity-30">👁</div>
      <p>Hit <strong class="text-gray-500">Scan for People</strong> to analyse your video.</p>
      <p class="text-xs mt-1 text-gray-700">
        Faces are prioritised — fallback to best person matches when no face is visible.
      </p>
    </div>

    <!-- Detection type legend -->
    <div id="fe-legend" class="hidden flex items-center gap-4 mb-3 text-xs text-gray-500">
      <span class="flex items-center gap-1">
        <span class="w-2 h-2 rounded-full bg-purple-500 inline-block"></span> Face detected
      </span>
      <span class="flex items-center gap-1">
        <span class="w-2 h-2 rounded-full bg-blue-500 inline-block"></span> Person (no face)
      </span>
    </div>

    <div id="fe-grid"
      class="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 gap-2">
    </div>
  </div>

  <!-- ── Action bar ─────────────────────────────────────────────────── -->
  <div id="fe-action-bar"
    class="hidden shrink-0 px-5 py-3 border-t border-gray-800
           flex flex-wrap items-center justify-between gap-3 bg-gray-950">
    <div class="flex items-center gap-3 text-xs">
      <span id="fe-frame-count" class="text-gray-400"></span>
      <button onclick="FrameExtractor._selectAll()"
        class="text-purple-400 hover:text-purple-300 underline underline-offset-2">
        Select all faces
      </button>
      <button onclick="FrameExtractor._clearSel()"
        class="text-gray-500 hover:text-gray-300 underline underline-offset-2">
        Clear
      </button>
    </div>
    <div class="flex items-center gap-3">
      <span class="text-xs text-gray-500">
        Selected frames will be uploaded as target photos
      </span>
      <button id="fe-use-btn" disabled onclick="FrameExtractor._confirm()"
        class="inline-flex items-center gap-2 px-5 py-2 rounded-lg text-sm font-bold
               bg-pink-600 hover:bg-pink-700 text-white transition-colors
               disabled:opacity-40 disabled:cursor-not-allowed">
        ✓ Use <span id="fe-use-count">0</span> Frame(s)
      </button>
    </div>
  </div>

</div>`;

    document.body.appendChild(modal);
    log('Modal injected');
  }

  /* ═══════════════════════════════════════════════════════════════════════
     AI MODEL LOADING
  ═══════════════════════════════════════════════════════════════════════ */
  let modelsLoading = false;
  let modelsLoaded  = false;

  async function loadModels() {
    if (modelsLoaded || modelsLoading) return;
    modelsLoading = true;

    log('Loading AI models…');
    setBadge('loading');

    // ── face-api.js ──────────────────────────────────────────────────────
    try {
      await loadScript(CFG.faceApiUrl);
      await window.faceapi.nets.tinyFaceDetector.loadFromUri(CFG.faceModelUrl);
      faceReady = true;
      log('✅ face-api TinyFaceDetector ready');
    } catch (e) {
      log('⚠️  face-api failed:', e.message, '— will use person-only detection');
      faceReady = false;
    }

    // ── TensorFlow + COCO-SSD ───────────────────────────────────────────
    try {
      await loadScript(CFG.tfUrl);
      await loadScript(CFG.cocoUrl);
      cocoModel = await window.cocoSsd.load();
      log('✅ COCO-SSD ready');
    } catch (e) {
      log('⚠️  COCO-SSD failed:', e.message);
      cocoModel = null;
    }

    modelsLoaded = true;
    modelsLoading = false;

    if (faceReady || cocoModel) {
      setBadge('ready');
    } else {
      setBadge('offline');
      setDiag('⚠️ AI models unavailable — cannot extract frames', 'red');
    }
  }

  function loadScript(src) {
    return new Promise((ok, fail) => {
      if (document.querySelector(`script[src="${src}"]`)) return ok();
      const s = document.createElement('script');
      s.src = src; s.onload = ok; s.onerror = () => fail(new Error(`Failed: ${src}`));
      document.head.appendChild(s);
    });
  }

  /* ═══════════════════════════════════════════════════════════════════════
     OPEN MODAL FOR A FILE
  ═══════════════════════════════════════════════════════════════════════ */
  async function openForFile(file) {
    if (!file || !file.type.startsWith('video/')) {
      log('openForFile: not a video file');
      return;
    }

    ensureModal();

    // Reset state
    frames = [];
    selected = new Set();
    busy = false;

    const grid = document.getElementById('fe-grid');
    if (grid) grid.innerHTML = '';
    document.getElementById('fe-empty-hint')?.classList.remove('hidden');
    document.getElementById('fe-legend')?.classList.add('hidden');
    document.getElementById('fe-action-bar')?.classList.add('hidden');
    document.getElementById('fe-progress-wrap')?.classList.add('hidden');
    document.getElementById('fe-extract-btn').disabled = true;

    // Revoke any previous object URL
    if (currentObjectUrl) URL.revokeObjectURL(currentObjectUrl);
    currentObjectUrl = URL.createObjectURL(file);

    // Show modal
    document.getElementById('fe-modal').classList.remove('hidden');
    setDiag('🔄 Loading video…', 'yellow');

    // Start loading models in parallel
    loadModels();

    // Load video metadata
    await loadVideoMeta(currentObjectUrl);
  }

  /* ─── load video duration ──────────────────────────────────────────── */
  function loadVideoMeta(src) {
    return new Promise(resolve => {
      const vid = getOrCreateHiddenVideo();
      vid.src = src;

      const onMeta = () => {
        const dur = vid.duration;
        log('Video duration:', dur);
        const el = document.getElementById('fe-duration');
        if (el) el.textContent = `Duration: ${fmt(dur)}`;

        if (modelsLoaded && (faceReady || cocoModel)) {
          setDiag('✅ Ready — click Scan for People', 'green');
          document.getElementById('fe-extract-btn').disabled = false;
        } else {
          setDiag('⏳ AI models still loading… please wait', 'yellow');
          waitForModels();
        }
        resolve(dur);
      };

      if (vid.readyState >= 1) { onMeta(); }
      else { vid.addEventListener('loadedmetadata', onMeta, { once: true }); }
    });
  }

  function waitForModels() {
    const poll = setInterval(() => {
      if (modelsLoaded) {
        clearInterval(poll);
        if (faceReady || cocoModel) {
          setDiag('✅ Ready — click Scan for People', 'green');
          document.getElementById('fe-extract-btn').disabled = false;
        } else {
          setDiag('❌ AI models failed to load', 'red');
        }
      }
    }, 400);
  }

  /* ─── hidden video element ─────────────────────────────────────────── */
  function getOrCreateHiddenVideo() {
    let v = document.getElementById('fe-hidden-video');
    if (!v) {
      v = document.createElement('video');
      v.id = 'fe-hidden-video';
      v.style.cssText = 'position:absolute;width:1px;height:1px;left:-9999px;visibility:hidden;';
      v.muted = true;
      v.playsInline = true;
      document.body.appendChild(v);
    }
    return v;
  }

  /* ═══════════════════════════════════════════════════════════════════════
     EXTRACTION
  ═══════════════════════════════════════════════════════════════════════ */
  async function startExtraction() {
    if (busy) return;
    busy = true;
    document.getElementById('fe-extract-btn').disabled = true;

    const vid = getOrCreateHiddenVideo();
    if (!vid.src) { busy = false; return; }

    const duration = vid.duration;
    if (!duration || isNaN(duration)) {
      setDiag('❌ Video duration unavailable', 'red');
      busy = false; return;
    }

    const interval = parseInt(document.getElementById('fe-interval')?.value || '2');

    // Reset grid
    frames = []; selected = new Set();
    const grid = document.getElementById('fe-grid');
    if (grid) grid.innerHTML = '';
    document.getElementById('fe-empty-hint')?.classList.add('hidden');
    document.getElementById('fe-legend')?.classList.add('hidden');
    document.getElementById('fe-action-bar')?.classList.add('hidden');
    document.getElementById('fe-progress-wrap')?.classList.remove('hidden');

    // Build timestamp list
    const stamps = [];
    for (let t = 0; t < duration && stamps.length < CFG.maxStamps; t += interval) {
      stamps.push(parseFloat(t.toFixed(2)));
    }
    log(`Scanning ${stamps.length} timestamps @ interval=${interval}s`);

    const canvas = document.createElement('canvas');
    canvas.width  = CFG.captureW;
    canvas.height = CFG.captureH;
    const ctx = canvas.getContext('2d');

    const rawResults = [];

    for (let i = 0; i < stamps.length; i++) {
      setProgress(i, stamps.length, `Analysing frame ${i + 1} / ${stamps.length}…`);

      const dataUrl = await grabFrame(vid, stamps[i], canvas, ctx);
      if (!dataUrl) continue;

      const analysis = await analyseFrame(dataUrl, canvas);
      if (!analysis) continue; // no person found

      rawResults.push({ dataUrl, ts: stamps[i], ...analysis });
    }

    // ── De-duplicate: suppress near-duplicate timestamps ────────────────
    rawResults.sort((a, b) => b.score - a.score); // best first
    const kept = [];
    for (const r of rawResults) {
      const tooClose = kept.some(k => Math.abs(k.ts - r.ts) < CFG.dedupWindow);
      if (!tooClose) kept.push(r);
      if (kept.length >= CFG.maxResults) break;
    }

    // Sort by timestamp for display
    kept.sort((a, b) => a.ts - b.ts);

    for (const r of kept) {
      frames.push({
        dataUrl: r.dataUrl,
        blob:    b64toBlob(r.dataUrl),
        ts:      r.ts,
        score:   r.score,
        hasFace: r.hasFace,
      });
      appendCard(frames.length - 1, grid);
    }

    const found = frames.length;
    setProgress(stamps.length, stamps.length,
      found > 0
        ? `✅ ${found} person frame(s) found`
        : '⚠️ No persons detected — try a shorter interval');

    setDiag(
      found > 0
        ? `✅ ${found} frame(s) — click to select, then "Use Frames"`
        : '⚠️ No persons detected. Try a shorter scan interval.',
      found > 0 ? 'green' : 'yellow');

    if (found > 0) {
      document.getElementById('fe-legend')?.classList.remove('hidden');
      document.getElementById('fe-action-bar')?.classList.remove('hidden');
      refreshActionBar();
      // Auto-select face frames
      frames.forEach((f, i) => { if (f.hasFace) toggleSelect(i); });
    } else {
      document.getElementById('fe-empty-hint')?.classList.remove('hidden');
      document.getElementById('fe-empty-hint').textContent =
        'No persons detected — try a shorter scan interval.';
    }

    busy = false;
    document.getElementById('fe-extract-btn').disabled = false;
    log(`Extraction done — ${found} frames kept`);
  }

  /* ─── grab one frame ───────────────────────────────────────────────── */
  function grabFrame(video, time, canvas, ctx) {
    return new Promise(resolve => {
      const onSeeked = () => {
        video.removeEventListener('seeked', onSeeked);
        try {
          ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
          resolve(canvas.toDataURL('image/jpeg', CFG.jpegQuality));
        } catch (e) {
          resolve(null);
        }
      };
      video.addEventListener('seeked', onSeeked);
      video.currentTime = time;
    });
  }

  /* ─── analyse one frame for faces/persons ──────────────────────────── */
  async function analyseFrame(dataUrl, canvas) {
    // Build HTMLImageElement from dataUrl
    const img = await loadImage(dataUrl);

    // ── Step 1: face detection ──────────────────────────────────────────
    if (faceReady && window.faceapi) {
      try {
        const detections = await window.faceapi.detectAllFaces(
          img,
          new window.faceapi.TinyFaceDetectorOptions({ scoreThreshold: 0.45 })
        );

        // Filter by minimum face width for recognisability
        const usable = detections.filter(d => d.box.width >= CFG.minFaceW);

        if (usable.length > 0) {
          // Score = best face confidence × sqrt(face_area_ratio) × multiplier
          const best = usable.reduce((a, b) => a.score > b.score ? a : b);
          const areaRatio = (best.box.width * best.box.height) /
                            (canvas.width * canvas.height);
          const score = best.score * Math.sqrt(areaRatio) * CFG.faceScoreMul;
          log(`  Face detected @ score=${score.toFixed(3)} size=${Math.round(best.box.width)}px`);
          return { hasFace: true, score };
        }
      } catch (e) {
        log('  face-api error:', e.message);
      }
    }

    // ── Step 2: COCO-SSD person detection (fallback) ────────────────────
    if (cocoModel) {
      try {
        const preds = await cocoModel.detect(img);
        const persons = preds.filter(p => p.class === 'person' && p.score >= 0.40);

        if (persons.length > 0) {
          const best = persons.reduce((a, b) => a.score > b.score ? a : b);
          const score = best.score * CFG.personScoreMul;
          log(`  Person (no face) @ score=${score.toFixed(3)}`);
          return { hasFace: false, score };
        }
      } catch (e) {
        log('  COCO-SSD error:', e.message);
      }
    }

    return null; // no person
  }

  function loadImage(dataUrl) {
    return new Promise(resolve => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.src = dataUrl;
    });
  }

  /* ═══════════════════════════════════════════════════════════════════════
     SELECTION
  ═══════════════════════════════════════════════════════════════════════ */
  function appendCard(idx, grid) {
    if (!grid) return;
    const f = frames[idx];
    const borderColor = f.hasFace ? 'border-purple-700/60' : 'border-blue-900/60';
    const dot         = f.hasFace
      ? '<span class="w-1.5 h-1.5 rounded-full bg-purple-400 inline-block"></span>'
      : '<span class="w-1.5 h-1.5 rounded-full bg-blue-500 inline-block"></span>';

    const card = document.createElement('div');
    card.id        = `fe-card-${idx}`;
    card.className = `relative cursor-pointer rounded-lg overflow-hidden border-2
                      ${borderColor} transition-all hover:border-purple-400/80 select-none`;
    card.onclick   = () => toggleSelect(idx);
    card.innerHTML = `
      <img src="${f.dataUrl}" class="w-full aspect-video object-cover block"
           alt="${fmt(f.ts)}" draggable="false">
      <div class="absolute bottom-0 left-0 right-0 bg-black/70 px-1 py-0.5
                  flex items-center justify-between">
        <span class="text-[8px] text-white">${fmt(f.ts)}</span>
        ${dot}
      </div>
      <div id="fe-chk-${idx}"
        class="hidden absolute top-0.5 right-0.5 w-4 h-4 rounded-full
               bg-pink-600 border border-white flex items-center justify-center pointer-events-none">
        <svg width="7" height="7" viewBox="0 0 12 12" fill="none" stroke="white" stroke-width="3">
          <path d="M2 6l3 3 5-5"/>
        </svg>
      </div>`;
    grid.appendChild(card);
  }

  function toggleSelect(idx) {
    const card = document.getElementById(`fe-card-${idx}`);
    const chk  = document.getElementById(`fe-chk-${idx}`);
    if (!card) return;
    if (selected.has(idx)) {
      selected.delete(idx);
      card.classList.remove('border-pink-500', 'ring-1', 'ring-pink-500/50');
      chk?.classList.add('hidden');
    } else {
      selected.add(idx);
      card.classList.add('border-pink-500', 'ring-1', 'ring-pink-500/50');
      chk?.classList.remove('hidden');
    }
    refreshActionBar();
  }

  function _selectAll() {
    // Prefer face frames; if none selected, select all
    const faceIdxs = frames.map((f, i) => f.hasFace ? i : -1).filter(i => i >= 0);
    const targets = faceIdxs.length > 0 ? faceIdxs : frames.map((_, i) => i);
    targets.forEach(i => { if (!selected.has(i)) toggleSelect(i); });
  }

  function _clearSel() {
    [...selected].forEach(i => toggleSelect(i));
  }

  function refreshActionBar() {
    const cnt = document.getElementById('fe-frame-count');
    const uc  = document.getElementById('fe-use-count');
    const ub  = document.getElementById('fe-use-btn');
    const faceCount = frames.filter(f => f.hasFace).length;
    if (cnt) cnt.textContent =
      `${frames.length} frame(s) · ${faceCount} with face · ${selected.size} selected`;
    if (uc)  uc.textContent  = selected.size;
    if (ub)  ub.disabled     = selected.size === 0;
  }

  /* ═══════════════════════════════════════════════════════════════════════
     CONFIRM → close modal, expose blobs
  ═══════════════════════════════════════════════════════════════════════ */
  function _confirm() {
    if (!selected.size) return;

    const blobs = [...selected].map(i => ({
      blob:     frames[i].blob,
      filename: `target_person_${fmt(frames[i].ts).replace(':', 's')}_${i}.jpg`,
      ts:       frames[i].ts,
      hasFace:  frames[i].hasFace,
    }));

    // Store for consumption by evidence_upload.js
    window._feSelectedBlobs = blobs;

    // Fire a custom event so upload page can react
    window.dispatchEvent(new CustomEvent('fe:confirmed', { detail: { blobs } }));

    log(`${blobs.length} frames confirmed as targets`);
    _close();
  }

  function _close() {
    const modal = document.getElementById('fe-modal');
    if (modal) modal.classList.add('hidden');
  }

  /* ═══════════════════════════════════════════════════════════════════════
     UI HELPERS
  ═══════════════════════════════════════════════════════════════════════ */
  const DIAG = {
    green:  'bg-green-500/10 border-green-500/30 text-green-300',
    yellow: 'bg-yellow-500/10 border-yellow-500/30 text-yellow-300',
    red:    'bg-red-500/10 border-red-500/30 text-red-300',
  };

  function setDiag(msg, color) {
    const el = document.getElementById('fe-diag');
    if (!el) return;
    el.className = `mx-5 mt-4 text-xs px-3 py-2 rounded-lg border ${DIAG[color] || DIAG.yellow}`;
    el.textContent = msg;
  }

  function setProgress(done, total, label) {
    const pct = total > 0 ? Math.round(done / total * 100) : 0;
    const b = document.getElementById('fe-progress-bar');
    const p = document.getElementById('fe-progress-pct');
    const l = document.getElementById('fe-progress-label');
    if (b) b.style.width = `${pct}%`;
    if (p) p.textContent = `${pct}%`;
    if (l) l.textContent = label;
  }

  function setBadge(state) {
    const el = document.getElementById('fe-ai-badge');
    if (!el) return;
    const map = {
      loading: ['bg-yellow-500/10 border border-yellow-500/20 text-yellow-400',
                '<span class="w-1.5 h-1.5 rounded-full bg-yellow-400 animate-pulse"></span>Loading AI…'],
      ready:   ['bg-green-500/10 border border-green-500/20 text-green-400',
                '<span class="w-1.5 h-1.5 rounded-full bg-green-400"></span>✅ AI ready'],
      offline: ['bg-gray-700/50 border border-gray-600/30 text-gray-500',
                '<span class="w-1.5 h-1.5 rounded-full bg-gray-500"></span>⚠ No AI'],
    };
    const [cls, html] = map[state] || map.loading;
    el.className = `inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full ${cls}`;
    el.innerHTML = html;
  }

  /* ─── utils ────────────────────────────────────────────────────────── */
  function fmt(s) {
    if (!s && s !== 0) return '00:00';
    return `${Math.floor(s / 60).toString().padStart(2, '0')}:${Math.floor(s % 60).toString().padStart(2, '0')}`;
  }

  function b64toBlob(dataUrl) {
    const [meta, data] = dataUrl.split(',');
    const mime  = meta.match(/:(.*?);/)[1];
    const bytes = atob(data);
    const buf   = new Uint8Array(bytes.length);
    for (let i = 0; i < bytes.length; i++) buf[i] = bytes.charCodeAt(i);
    return new Blob([buf], { type: mime });
  }

  function log(...a) { console.log('[FrameExtractor v3]', ...a); }

  /* ═══════════════════════════════════════════════════════════════════════
     PUBLIC API
  ═══════════════════════════════════════════════════════════════════════ */
  window.FrameExtractor = {
    openForFile,
    startExtraction,
    getSelectedBlobs: () => window._feSelectedBlobs || [],
    clearSelected:    () => { window._feSelectedBlobs = []; selected.clear(); },

    // Internal (called from injected HTML)
    _close,
    _confirm,
    _selectAll,
    _clearSel,
  };

  // Pre-inject modal and start loading models as soon as DOM is ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => { ensureModal(); loadModels(); });
  } else {
    ensureModal();
    loadModels();
  }

  log('Loaded — waiting for openForFile() call');

})();