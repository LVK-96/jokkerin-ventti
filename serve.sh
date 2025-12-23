#!/bin/bash
# Simple local development server for the timer website

PORT="${1:-8000}"

echo "Starting local server at http://localhost:${PORT}"
echo "Press Ctrl+C to stop"

python3 -m http.server "$PORT"
