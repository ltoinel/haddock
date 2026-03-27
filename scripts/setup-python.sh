#!/bin/bash
# Setup script to download Python embeddable for Windows and install Sherlock
# Run this from WSL or Linux before cross-compiling, or use the .ps1 on Windows

set -e

PYTHON_VERSION="3.12.9"
PYTHON_ZIP="python-${PYTHON_VERSION}-embed-amd64.zip"
PYTHON_URL="https://www.python.org/ftp/python/${PYTHON_VERSION}/${PYTHON_ZIP}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEST="${SCRIPT_DIR}/../src-tauri/python-embed"

echo "=== Haddock: Setting up embedded Python + Sherlock ==="

# Clean previous install
if [ -d "$DEST" ]; then
    echo "Removing previous embedded Python..."
    rm -rf "$DEST"
fi

mkdir -p "$DEST"

# Download Python embeddable
if [ ! -f "/tmp/${PYTHON_ZIP}" ]; then
    echo "Downloading Python ${PYTHON_VERSION} embeddable..."
    curl -Lo "/tmp/${PYTHON_ZIP}" "$PYTHON_URL"
fi

# Extract
echo "Extracting Python..."
unzip -o "/tmp/${PYTHON_ZIP}" -d "$DEST"

# Enable import site
PTH_FILE=$(ls "$DEST"/python*._pth 2>/dev/null | head -1)
if [ -n "$PTH_FILE" ]; then
    echo "Enabling site-packages in $(basename "$PTH_FILE")..."
    sed -i 's/^#import site/import site/' "$PTH_FILE"
    echo "Lib\\site-packages" >> "$PTH_FILE"
fi

echo ""
echo "=== Python extracted ==="
echo "NOTE: To complete setup, run the following on Windows:"
echo "  cd src-tauri/python-embed"
echo "  python.exe get-pip.py"
echo "  python.exe -m pip install sherlock-project"
echo ""
echo "Or use scripts/setup-python.ps1 directly on Windows."
