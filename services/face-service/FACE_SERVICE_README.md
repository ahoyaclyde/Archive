# Face Service — Setup Guide

## Files in this delivery

| File | Purpose |
|---|---|
| `face_service.js` | Node.js HTTP sidecar (the main service) |
| `package.json` | Node.js dependencies |
| `download_models.js` | One-time model weight downloader (~6.4 MB) |
| `ecosystem.config.js` | PM2 production config |
| `face_client.rs` | Rust HTTP client + `evidence_service.rs` patch instructions |

---

## Step 1 — Install Node.js dependencies

```bash
# Place all files in a folder, e.g. /opt/face-service/
cd /opt/face-service

npm install
```

> Requires Node.js >= 18. Check with: `node --version`

---

## Step 2 — Download model weights (one time only)

```bash
node download_models.js
```

Expected output:
```
📥 Downloading face-api.js model weights...

   ↓ ssd_mobilenetv1_model-weights_manifest.json    100%
   ↓ ssd_mobilenetv1_model-shard1                   100%
   ...
✅ Done! 8 downloaded, 0 already present.
```

Downloads ~6.4 MB into `./models/`. Safe to re-run — skips already-downloaded files.

---

## Step 3 — Test the service manually

```bash
node face_service.js
```

In a second terminal:
```bash
# Health check
curl http://127.0.0.1:3001/health
# Expected: {"status":"ok","models_loaded":true}
```

---

## Step 4 — Install PM2 and start for production

```bash
npm install -g pm2

# Start
pm2 start ecosystem.config.js

# Verify running
pm2 status

# Save process list + enable autostart on reboot
pm2 save
pm2 startup
# → Follow the printed command (sudo env PATH=...)
```

---

## Step 5 — Wire Rust (face_client.rs)

1. Copy `face_client.rs` into `src/`
2. Add to `Cargo.toml`:
   ```toml
   reqwest = { version = "0.11", features = ["json"] }
   ```
3. Register in `main.rs` or `lib.rs`:
   ```rust
   mod face_client;
   pub use face_client::FaceClient;
   ```
4. Add to `EvidenceService` struct in `evidence_service.rs`:
   ```rust
   pub face_client: FaceClient,
   ```
5. Add to `EvidenceService::new()`:
   ```rust
   face_client: FaceClient::new(),
   ```
6. Replace the hash-check block in `upload_target_photos()` with the
   **PATCH** code found at the bottom of `face_client.rs`.

---

## Environment variables (all optional)

| Variable | Default | Description |
|---|---|---|
| `FACE_SERVICE_URL` | `http://127.0.0.1:3001` | Used by Rust client |
| `FACE_SERVICE_PORT` | `3001` | Port the Node service listens on |
| `FACE_SERVICE_HOST` | `127.0.0.1` | Bind address (localhost only — never expose publicly) |
| `FACE_MODELS_PATH` | `./models` | Path to model weight files |
| `FACE_THRESHOLD` | `0.55` | Euclidean distance threshold |
| `FACE_MAX_DIM` | `640` | Max image dimension before resize |

---

## Useful PM2 commands

```bash
pm2 logs face-service          # live logs
pm2 restart face-service       # restart after config change
pm2 stop face-service          # stop without deleting
pm2 delete face-service        # remove from PM2 list
pm2 monit                      # live dashboard
```

---

## How matching works (summary)

```
Upload target photo
    │
    ├─ Layer 0: SHA-256 exact hash match (Rust, existing)
    │
    └─ Layer 1: face-api.js (Node sidecar)
           │
           ├─ Detect all faces in image
           │      ├─ Face found → generate 128-dim descriptor
           │      │       └─ Search face_encodings table (Euclidean ≤ 0.55)
           │      │               ├─ Match → link_evidence_cases() + notify
           │      │               └─ No match → store new encoding
           │      │
           │      └─ No face → pHash fallback (Phase 4)
           │
           └─ Always store pHash alongside encoding
```
