#!/bin/bash

# Helper script to run coddog-db commands through Docker
# This script forwards all arguments to the coddog-db running in a container

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

# Run the DB command with all passed arguments
echo "Running: coddog-db $*"
docker compose run --rm db ./coddog-db "$@"