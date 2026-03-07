# ─────────────────────────────────────────────────────────────────────────────
# Stage 1 — Rust builder
# Uses nightly on Debian trixie (GLIBC 2.39) so the binary links against the
# same GLIBC version that the runtime image ships with. No musl needed.
# ─────────────────────────────────────────────────────────────────────────────
FROM rustlang/rust:nightly AS builder

WORKDIR /app

# OpenSSL dev headers needed by openssl-sys crate
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release


# ─────────────────────────────────────────────────────────────────────────────
# Stage 2 — Runtime (trixie = GLIBC 2.39 — matches nightly compiler output)
# ─────────────────────────────────────────────────────────────────────────────
FROM debian:trixie-slim

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
    # Node.js (via NodeSource — trixie compatible)
    curl \
    gnupg \
    # OpenCV headless runtime deps
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
# Downloaded once at build time — zero cold-start delay at runtime.
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
COPY --from=builder /app/static           /app/static
COPY --from=builder /app/update_pickle.py /app/update_pickle.py
COPY --from=builder /app/start.sh         /app/start.sh

# ── Copy and install face sidecar ─────────────────────────────────────────────
COPY --from=builder /app/services/face-service /app/services/face-service
RUN cd /app/services/face-service \
    && npm install --omit=dev \
    && node download_models.js \
    && echo "✅ face-api.js models ready"

# ── Persistent disk symlink ───────────────────────────────────────────────────
# /app/data is mounted as Render persistent disk
# /app/encodings symlinks there so ./encodings relative path works in code
RUN mkdir -p /app/data /app/data/encodings \
    && ln -s /app/data/encodings /app/encodings

ENV ENCODINGS_DIR=/app/data/encodings

# ── Permissions ───────────────────────────────────────────────────────────────
RUN chmod +x /app/start.sh /app/archive

EXPOSE 8080

ENTRYPOINT ["/app/start.sh"]