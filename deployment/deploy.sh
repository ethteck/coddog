#!/bin/bash

# Deployment script for cod.dog
# This script ensures the git hash is passed to the Docker build

set -e

# Get the current git hash
GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

echo "Building cod.dog with git hash: $GIT_HASH"

# Change to deployment directory
cd "$(dirname "$0")"

# Export environment variables for docker-compose
export GIT_HASH="$GIT_HASH"
export API_BASE_URL="${API_BASE_URL:-/api}"

# Build and start the services
docker-compose down
docker-compose build --no-cache
docker-compose up -d

echo "Deployment complete!"
echo "Git hash: $GIT_HASH"
echo "API base URL: $API_BASE_URL"