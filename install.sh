#!/bin/sh
# RustyLights installer
# Usage: curl -sSL https://raw.githubusercontent.com/appliedapp/rusty_lights/main/install.sh | sudo sh
set -e

REPO="appliedapp/rusty_lights"

# Detect architecture
ARCH=$(dpkg --print-architecture 2>/dev/null || uname -m)
case "$ARCH" in
    arm64|aarch64)  DEB_ARCH="arm64" ;;
    armhf|armv7l)   DEB_ARCH="armhf" ;;
    amd64|x86_64)   DEB_ARCH="amd64" ;;
    *)
        echo "Error: Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

echo "Detecting architecture: $DEB_ARCH"

# Get latest release tag
TAG=$(curl -sSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$TAG" ]; then
    echo "Error: Could not determine latest release"
    exit 1
fi

echo "Latest release: $TAG"

# Download and install .deb
DEB="rusty-lights_${TAG#v}_${DEB_ARCH}.deb"
URL="https://github.com/$REPO/releases/download/$TAG/$DEB"

echo "Downloading $URL ..."
TMP=$(mktemp -d)
curl -sSL -o "$TMP/$DEB" "$URL"

echo "Installing..."
dpkg -i "$TMP/$DEB"
rm -rf "$TMP"

echo ""
echo "Done! Edit /etc/rusty_lights.conf, then:"
echo "  sudo systemctl enable --now rusty_lights"
