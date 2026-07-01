#!/bin/sh
# Installs the latest light-gen-subZ release for Linux.
# Usage: curl -fsSL https://raw.githubusercontent.com/sindus/light-gen-subZ/main/install.sh | sh
set -e

REPO="sindus/light-gen-subZ"
API_URL="https://api.github.com/repos/$REPO/releases/latest"

os="$(uname -s)"
arch="$(uname -m)"

if [ "$os" = "Darwin" ]; then
  echo "On macOS, install light-gen-subZ via Homebrew instead:"
  echo
  echo "  brew tap sindus/light-gen-subz"
  echo "  brew install --cask light-gen-subz"
  exit 1
fi

if [ "$os" != "Linux" ]; then
  echo "Unsupported OS: $os" >&2
  exit 1
fi

command -v curl >/dev/null 2>&1 || {
  echo "curl is required to install light-gen-subZ." >&2
  exit 1
}

echo "Fetching latest release info from $REPO..."
release_json="$(curl -fsSL "$API_URL")"

pick_asset_url() {
  pattern="$1"
  printf '%s' "$release_json" |
    grep -io "\"browser_download_url\": *\"[^\"]*${pattern}[^\"]*\"" |
    head -n1 |
    sed -E 's/.*"(https[^"]+)"/\1/'
}

case "$arch" in
  x86_64 | amd64) arch_tag="amd64" ;;
  aarch64 | arm64) arch_tag="arm64" ;;
  *)
    echo "Unsupported architecture: $arch" >&2
    exit 1
    ;;
esac

ensure_ffmpeg() {
  if command -v ffmpeg >/dev/null 2>&1; then
    return
  fi
  if command -v apt-get >/dev/null 2>&1; then
    echo "ffmpeg not found, installing it via apt (sudo required)..."
    sudo apt-get update -qq
    sudo apt-get install -y ffmpeg
  else
    echo "Warning: ffmpeg was not found and could not be installed automatically." >&2
    echo "light-gen-subZ needs it on your PATH to extract audio from video files." >&2
    echo "Install it with your distro's package manager (e.g. 'dnf install ffmpeg', 'pacman -S ffmpeg')." >&2
  fi
}

if command -v dpkg >/dev/null 2>&1; then
  url="$(pick_asset_url "${arch_tag}\\.deb")"
  if [ -z "$url" ]; then
    echo "Could not find a .deb asset for arch $arch in the latest release." >&2
    exit 1
  fi
  tmp="$(mktemp --suffix=.deb)"
  echo "Downloading $url"
  curl -fsSL "$url" -o "$tmp"
  echo "Installing (sudo required)..."
  sudo apt-get install -y "$tmp" || sudo dpkg -i "$tmp"
  rm -f "$tmp"
  ensure_ffmpeg
  echo "Installed. Launch it from your app menu, or run: light-gen-subz"
else
  url="$(pick_asset_url "${arch_tag}\\.AppImage")"
  if [ -z "$url" ]; then
    echo "Could not find an AppImage asset for arch $arch in the latest release." >&2
    exit 1
  fi
  mkdir -p "$HOME/.local/bin"
  dest="$HOME/.local/bin/light-gen-subz.AppImage"
  echo "Downloading $url"
  curl -fsSL "$url" -o "$dest"
  chmod +x "$dest"
  ensure_ffmpeg
  echo "Installed to $dest"
  echo "Make sure $HOME/.local/bin is on your PATH, then run: light-gen-subz.AppImage"
fi
