#!/bin/bash

echo "🚀 Building archive FORENSICS AI..."


# Export environment variables for wallets
# Ethereum/BASE/Avalanche support
# Set defaults if not set
export STORJ_ACCESS_KEY=${STORJ_ACCESS_KEY:-"jvxhlw3ggcwuqxeau2fgakuts2wq"}
export STORJ_SECRET_KEY=${STORJ_SECRET_KEY:-"j2obtp5yz2mikbyzx52okrr44vimmrdwdf5vhowhk5jnaq5a6km6c"}
export STORJ_ENDPOINT=${STORJ_ENDPOINT:-"https://gateway.storjshare.io"}
export RUST_LOG=${RUST_LOG:-"debug"}

# Optional: Blockchain variables
# export LISK_NODE_URL="https://testnet.lisk.com"
# export INFURA_API_KEY="your-infura-key"
# export ALCHEMY_API_KEY="your-alchemy-key"


# Install dependencies if needed
# cargo install --locked --path .

# Build the application
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    ls -la target/release/
    echo "🚀 Starting application..."
    ./target/release/archive
else
    echo "❌ Build failed!"
    exit 1
fi