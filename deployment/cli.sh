#!/bin/bash

# Helper script to run coddog-cli commands through Docker
# This script forwards all arguments to the coddog-cli running in a container

set -e

# Change to the deployment directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Check if .env file exists and source it
if [ -f .env ]; then
    set -a
    source .env
    set +a
fi

# Ensure the CLI service is built
echo "Building CLI service if needed..."
docker compose build cli

# Run the CLI command with all passed arguments
echo "Running: coddog-cli $*"
docker compose run --rm cli ./coddog-cli "$@"