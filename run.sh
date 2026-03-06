

#!/bin/bash

# Export environment variables for wallets
# Ethereum/BASE/Avalanche support
# Set defaults if not set
export STORJ_ACCESS_KEY=${STORJ_ACCESS_KEY:-"jv7kqbaoch5tgxvqw6kzid6r5ttq"}
export STORJ_SECRET_KEY=${STORJ_SECRET_KEY:-"j3agyyqmddqwik2wgh7judvgbnau2cb2dpnphd4eyf6wggo2acwwk"}
export STORJ_ENDPOINT=${STORJ_ENDPOINT:-"https://gateway.storjshare.io"}
export RUST_LOG=${RUST_LOG:-"debug"}

# Optional: Blockchain variables
# export LISK_NODE_URL="https://testnet.lisk.com"
# export INFURA_API_KEY="your-infura-key"
# export ALCHEMY_API_KEY="your-alchemy-key"

echo "Starting FLUG Evidence server..."
cargo run --release