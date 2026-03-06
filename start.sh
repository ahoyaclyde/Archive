#!/bin/bash
set -e

echo "━━━ CrimeBank Starting ━━━"
echo "Node:   $(node --version)"
echo "Python: $(python3 --version)"

# ── Ensure persistent disk directories exist ──────────────────────────────────
mkdir -p /app/data /app/data/encodings
echo "✅ Persistent disk directories ready"

# ── Start Node.js face sidecar via PM2 ───────────────────────────────────────
echo "▶ Starting face sidecar..."
cd /app/services/face-service
pm2 start ecosystem.config.js --no-daemon &
PM2_PID=$!
cd /app

# Wait up to 15s for sidecar to become healthy
echo "⏳ Waiting for face sidecar..."
for i in $(seq 1 15); do
    if curl -sf http://127.0.0.1:3001/health > /dev/null 2>&1; then
        echo "✅ Face sidecar ready (${i}s)"
        break
    fi
    sleep 1
    if [ "$i" -eq 15 ]; then
        echo "⚠️  Face sidecar not ready after 15s — continuing anyway"
        echo "   Uploads will work, face matching resumes when sidecar is healthy"
    fi
done

# ── Start Rust API (foreground — keeps container alive) ───────────────────────
echo "▶ Starting Rust API..."
exec /app/archive