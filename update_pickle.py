#!/usr/bin/env python3
"""
update_pickle.py — CrimeBank Face Encodings Updater
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Reads target images from local temp files written by Rust, generates FaceNet
embeddings, and updates encodings.pickle on disk. No Storj access — Rust handles
the upload after this script exits.

Architecture:
    Rust (image bytes in memory)
        → writes /tmp/pickle_target_<uuid>.jpg   (temp, cleaned up after)
        → calls: python3 update_pickle.py '<json>'
    Python
        → reads each local_path from disk
        → generates 128-dim FaceNet embedding (DeepFace, no dlib)
        → loads ./encodings/encodings.pickle  (creates if missing)
        → appends new entries
        → saves ./encodings/encodings.pickle
        → prints OK:./encodings/encodings.pickle to stdout
    Rust
        → reads ./encodings/encodings.pickle
        → uploads to Storj as encodings/encodings.pickle

Usage:
    python3 update_pickle.py '<targets_json>'

targets_json — JSON array, each element:
    {
        "target_id":   "target_xxx",
        "evidence_id": "evidence_xxx",
        "name":        "EVD-2026-xxxxx",
        "local_path":  "/tmp/pickle_target_<uuid>.jpg"
    }

Stdout:
    OK:./encodings/encodings.pickle   — success
    ERROR:<message>                   — failure

Stderr:
    All human-readable logs (Rust prints these verbatim to its console)

Requirements:
    pip install deepface numpy opencv-python-headless tf-keras
"""

import sys
import os

# ── Stdout lockdown — must happen before ANY other import ────────────────────
# TensorFlow's C++ runtime prints directly to stdout via file descriptor 1,
# bypassing Python's sys.stdout entirely. env vars alone don't stop it.
# Solution: redirect fd 1 to /dev/null at the OS level immediately,
# then restore it only for the final OK:/ERROR: result line Rust parses.
_real_stdout_fd  = os.dup(1)                    # save real stdout fd
_devnull_fd      = os.open(os.devnull, os.O_WRONLY)
os.dup2(_devnull_fd, 1)                         # fd 1 → /dev/null
os.close(_devnull_fd)
_real_stdout     = sys.stdout                   # save Python stdout too
sys.stdout       = open(os.devnull, 'w')        # Python stdout → /dev/null

def _print_result(msg: str):
    """Restore fd 1 and print the final OK:/ERROR: line Rust reads."""
    sys.stdout = _real_stdout
    os.dup2(_real_stdout_fd, 1)
    os.close(_real_stdout_fd)
    print(msg, flush=True)

# TF env vars as backup (belt + suspenders)
os.environ["TF_CPP_MIN_LOG_LEVEL"]      = "3"
os.environ["TF_ENABLE_ONEDNN_OPTS"]     = "0"
os.environ["CUDA_VISIBLE_DEVICES"]      = ""
os.environ["TF_FORCE_GPU_ALLOW_GROWTH"] = "true"

import json
import pickle
import logging
import tempfile
import time
import datetime
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed

import numpy as np

# ── Logging — stderr only, stdout is reserved for machine-readable result ─────
logging.basicConfig(
    stream=sys.stderr,
    level=logging.INFO,
    format="🥒 [PICKLE] %(asctime)s %(levelname)s — %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger("pickle_updater")

# ── Configuration ─────────────────────────────────────────────────────────────
PICKLE_DIR     = "./encodings"
PICKLE_PATH    = "./encodings/encodings.pickle"
MODEL_NAME     = "Facenet"    # 128-dim, no dlib, pure pip
DETECTOR       = "opencv"     # fastest detector, no extra deps
MAX_WORKERS    = 4            # parallel encoding threads

# ── Model cache — built once per process, reused across all thread calls ──────
_model_cache = None

def get_model():
    global _model_cache
    if _model_cache is None:
        from deepface import DeepFace
        log.info("Loading FaceNet model into memory (one-time cost)...")
        t0 = time.time()
        _model_cache = DeepFace.build_model(MODEL_NAME)
        log.info("FaceNet model ready in %.2fs", time.time() - t0)
    return _model_cache


# ── Pickle structure ──────────────────────────────────────────────────────────
def empty_store():
    """Returns a fresh empty encoding store with all expected keys."""
    return {
        "encodings":    [],   # List[np.ndarray] — 128-dim FaceNet embedding
        "target_ids":   [],   # str — dedup key, also links to DB target record
        "evidence_ids": [],   # str — links encoding back to evidence record
        "names":        [],   # str — EVD-2026-xxxxx human label for classifier
        "timestamps":   [],   # str — ISO8601 UTC when encoding was added
        "model":        MODEL_NAME,
        "version":      2,
    }


# ── Pickle I/O ────────────────────────────────────────────────────────────────
def load_pickle() -> dict:
    """
    Load the pickle from disk. If it doesn't exist yet (first run or fresh
    deploy) returns an empty store — Rust will have seeded it from Storj on
    startup if the persistent disk had it previously.
    """
    if not Path(PICKLE_PATH).exists():
        log.info("No local pickle found at %s — starting fresh store", PICKLE_PATH)
        return empty_store()

    try:
        t0 = time.time()
        with open(PICKLE_PATH, "rb") as fh:
            store = pickle.load(fh)

        # Backfill keys added in newer versions so old pickles stay compatible
        for key, default in empty_store().items():
            if key not in store:
                store[key] = default

        log.info(
            "Loaded local pickle — %d encoding(s) already present (%.2fs)",
            len(store["encodings"]), time.time() - t0
        )
        return store

    except Exception as exc:
        log.warning("Corrupt or unreadable pickle (%s) — starting fresh", exc)
        return empty_store()


def save_pickle(store: dict) -> str:
    """Ensure encodings/ dir exists, write pickle, return path."""
    Path(PICKLE_DIR).mkdir(parents=True, exist_ok=True)
    with open(PICKLE_PATH, "wb") as fh:
        pickle.dump(store, fh, protocol=pickle.HIGHEST_PROTOCOL)
    size_kb = Path(PICKLE_PATH).stat().st_size / 1024
    log.info(
        "Pickle saved → %s  (%.1f KB, %d total encodings)",
        PICKLE_PATH, size_kb, len(store["encodings"])
    )
    return PICKLE_PATH


# ── Face encoding ─────────────────────────────────────────────────────────────
def encode_local_file(local_path: str, target_id: str) -> np.ndarray | None:
    """
    Read image from local_path, run DeepFace.represent(), return 128-dim array.
    Cleans up the temp file after reading regardless of outcome.
    Runs safely inside a ThreadPoolExecutor — DeepFace is thread-safe with opencv.
    """
    from deepface import DeepFace

    if not Path(local_path).exists():
        log.error("  ❌ Temp file not found: %s", local_path)
        return None

    # Redirect stdout during inference to prevent TF noise corrupting our protocol
    _old_stdout = sys.stdout
    sys.stdout = open(os.devnull, 'w')
    try:
        t0 = time.time()
        result = DeepFace.represent(
            img_path=local_path,
            model_name=MODEL_NAME,
            detector_backend=DETECTOR,
            enforce_detection=True,
            align=True,
        )

        if not result:
            log.warning("  ⚠️  No face representation returned for %s", target_id[:24])
            return None

        embedding = np.array(result[0]["embedding"], dtype=np.float32)
        log.info(
            "  🧬 Encoded %s → %d-dim in %.2fs",
            target_id[:24], len(embedding), time.time() - t0
        )
        return embedding

    except ValueError as exc:
        # DeepFace raises ValueError when enforce_detection=True and no face found
        log.warning("  ⚠️  No face detected in image for %s: %s", target_id[:24], exc)
        return None
    except Exception as exc:
        log.error("  ❌ Encoding error for %s: %s", target_id[:24], exc)
        return None
    finally:
        # Always clean up the temp file Rust wrote — win or fail
        try:
            os.unlink(local_path)
            log.info("  🗑️  Cleaned up temp file: %s", local_path)
        except OSError:
            pass


# ── Main ──────────────────────────────────────────────────────────────────────
def run(targets: list) -> str:
    t_start = time.time()
    log.info("━━━ CrimeBank Pickle Updater — %d target(s) received ━━━", len(targets))

    # Load local pickle and filter out targets already encoded (dedup)
    store       = load_pickle()
    existing    = set(store["target_ids"])
    new_targets = [t for t in targets if t["target_id"] not in existing]
    skipped     = len(targets) - len(new_targets)

    if skipped:
        log.info("Skipping %d already-encoded target(s)", skipped)
    if not new_targets:
        log.info("Nothing new — pickle unchanged")
        return save_pickle(store)

    log.info("Encoding %d new target(s) from local temp files...", len(new_targets))

    # Warm model in main thread before spawning workers — avoids each worker
    # paying the model load cost independently
    get_model()

    encoded, failed = 0, 0

    with ThreadPoolExecutor(max_workers=MAX_WORKERS) as pool:
        futures = {
            pool.submit(encode_local_file, t["local_path"], t["target_id"]): t
            for t in new_targets
        }
        for future in as_completed(futures):
            meta = futures[future]
            try:
                embedding = future.result()
            except Exception as exc:
                log.error("  ❌ Future error for %s: %s", meta["target_id"][:24], exc)
                embedding = None

            if embedding is not None and len(embedding) == 128:
                store["encodings"].append(embedding)
                store["target_ids"].append(meta["target_id"])
                store["evidence_ids"].append(meta.get("evidence_id", ""))
                store["names"].append(meta.get("name", ""))
                store["timestamps"].append(datetime.datetime.utcnow().isoformat())
                encoded += 1
                log.info(
                    "  ✅ Added — %s (%s)",
                    meta.get("name", "?"), meta["target_id"][:24]
                )
            else:
                failed += 1

    log.info(
        "━━━ Done — encoded=%d  failed=%d  total_in_store=%d  elapsed=%.2fs ━━━",
        encoded, failed, len(store["encodings"]), time.time() - t_start
    )

    return save_pickle(store)


# ── Entry point ───────────────────────────────────────────────────────────────
if __name__ == "__main__":
    if len(sys.argv) < 2:
        _print_result("ERROR:Usage: update_pickle.py '<targets_json>'")
        sys.exit(1)

    try:
        targets = json.loads(sys.argv[1])

        if not isinstance(targets, list):
            raise ValueError("targets_json must be a JSON array")

        log.info("Parsed %d target(s) from args", len(targets))

        # Validate each target has required fields
        for t in targets:
            for field in ("target_id", "evidence_id", "name", "local_path"):
                if field not in t:
                    raise ValueError(f"Missing required field '{field}' in target {t}")

    except Exception as exc:
        _print_result(f"ERROR:Invalid arguments — {exc}")
        sys.exit(1)

    try:
        result_path = run(targets)
        _print_result(f"OK:{result_path}")
    except Exception as exc:
        log.exception("Fatal error")
        _print_result(f"ERROR:{exc}")
        sys.exit(1)