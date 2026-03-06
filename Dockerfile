# ─────────────────────────────────────────────────────────────────────────────
# Stage 1 — Rust builder
# Compiles the release binary. Nothing from this stage bleeds into production.
# ─────────────────────────────────────────────────────────────────────────────
FROM rustlang/rust:nightly AS builder

WORKDIR /app
COPY . .
RUN cargo build --release


# ─────────────────────────────────────────────────────────────────────────────
# Stage 2 — Runtime
# Installs Node.js + Python + face deps + pre-warms models.
# ─────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

WORKDIR /app

# ── System packages ───────────────────────────────────────────────────────────
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Rust binary runtime
    ca-certificates \
    libssl3 \
    # Python
    python3 \
    python3-pip \
    python3-dev \
    # Node.js (via NodeSource — bookworm = Node 20 LTS)
    curl \
    gnupg \
    # OpenCV headless needs these
    libglib2.0-0 \
    libsm6 \
    libxext6 \
    libxrender1 \
    libgomp1 \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g pm2 \
    && rm -rf /var/lib/apt/lists/*

# ── Python dependencies ───────────────────────────────────────────────────────
RUN pip3 install --no-cache-dir --break-system-packages \
    deepface \
    numpy \
    opencv-python-headless \
    tf-keras

# ── Pre-warm FaceNet model weights (~92MB) ────────────────────────────────────
# Downloads once at build time → zero cold-start delay at runtime.
# Cached in /root/.deepface/weights/ inside the image layer.
RUN python3 - << 'PYEOF'
import os
os.environ["TF_CPP_MIN_LOG_LEVEL"] = "3"
os.environ["CUDA_VISIBLE_DEVICES"] = ""
try:
    from deepface import DeepFace
    DeepFace.build_model("Facenet")
    print("✅ FaceNet weights pre-downloaded")
except Exception as e:
    print(f"⚠️  Pre-warm failed (will download on first use): {e}")
PYEOF

# ── Copy Rust binary ──────────────────────────────────────────────────────────
COPY --from=builder /app/target/release/archive /app/archive

# ── Copy application files ────────────────────────────────────────────────────
COPY --from=builder /app/static        /app/static
COPY --from=builder /app/update_pickle.py /app/update_pickle.py
COPY --from=builder /app/start.sh      /app/start.sh

# ── Copy face sidecar ─────────────────────────────────────────────────────────
COPY --from=builder /app/services/face-service /app/services/face-service

# ── Install face sidecar Node.js dependencies + download face-api.js models ──
RUN cd /app/services/face-service \
    && npm install --omit=dev \
    && node download_models.js \
    && echo "✅ face-api.js models downloaded"

# ── Runtime directories ───────────────────────────────────────────────────────
# /app/data  → mounted as Render persistent disk (SQLite + encodings pickle)
# /app/encodings → symlink to /app/data/encodings so ./encodings works in code
RUN mkdir -p /app/data /app/data/encodings \
    && ln -s /app/data/encodings /app/encodings

# ── Pickle path env var ───────────────────────────────────────────────────────
# update_pickle.py reads ENCODINGS_DIR — points at persistent disk
ENV ENCODINGS_DIR=/app/data/encodings

# ── Permissions ───────────────────────────────────────────────────────────────
RUN chmod +x /app/start.sh /app/archive

EXPOSE 8080

ENTRYPOINT ["/app/start.sh"]