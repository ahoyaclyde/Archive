#!/bin/bash
# ── LOCAL DEVELOPMENT BUILD ───────────────────────────────────────────────────
# Not used by Render — Render uses the Dockerfile directly.
# Run this locally to test a release build before pushing.
set -e

echo "🔨 Building CrimeBank (local release build)..."
cargo build --release

echo "✅ Build complete → ./target/release/archive"
echo "Run with: bash run.sh"