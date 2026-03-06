#!/bin/bash
# ── LOCAL DEVELOPMENT RUN ─────────────────────────────────────────────────────
# Not used by Render — Render uses start.sh via Dockerfile ENTRYPOINT.
# Run this locally after `bash build.sh`.
set -e

# Load .env if present
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
    echo "✅ Loaded .env"
fi

echo "🚀 Starting CrimeBank locally..."
./target/release/archive