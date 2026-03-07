# ─────────────────────────────────────────────────────────────────────────────
# Stage 1 — Rust builder (musl static binary — no GLIBC dependency)
# ─────────────────────────────────────────────────────────────────────────────
FROM rustlang/rust:nightly AS builder

WORKDIR /app

# Install musl toolchain for fully static compilation
RUN apt-get update && apt-get install -y musl-tools && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl

COPY . .

# Build fully static binary — runs on any Linux regardless of GLIBC version
RUN cargo build --release --target x86_64-unknown-linux-musl


# ─────────────────────────────────────────────────────────────────────────────
# Stage 2 — Runtime
# ─────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

WORKDIR /app

# ── System packages ───────────────────────────────────────────────────────────
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    python3 \
    python3-pip \
    python3-dev \
    curl \
    gnupg \
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

# ── Copy static Rust binary (no GLIBC needed) ─────────────────────────────────
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/archive /app/archive

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

# ── Persistent disk directories ───────────────────────────────────────────────
# /app/data is mounted as Render persistent disk
# ./encodings symlinks there so update_pickle.py default path works too
RUN mkdir -p /app/data /app/data/encodings \
    && ln -s /app/data/encodings /app/encodings

ENV ENCODINGS_DIR=/app/data/encodings

# ── Permissions ───────────────────────────────────────────────────────────────
RUN chmod +x /app/start.sh /app/archive

EXPOSE 8080

ENTRYPOINT ["/app/start.sh"]