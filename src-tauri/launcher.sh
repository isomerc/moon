#!/bin/bash
# Launcher script for MOON app
# Forces X11 backend for webkit2gtk compatibility

export GDK_BACKEND=x11

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Execute the actual binary
exec "$SCRIPT_DIR/moon" "$@"
