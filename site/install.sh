#!/bin/sh
# genome installer — https://genome.nex-ovia.com/install.sh
#
# Downloads the latest genome release for your platform from GitHub, verifies
# its SHA-256, and installs the binary. Fully inspectable — read it first:
#   curl -fsSL https://genome.nex-ovia.com/install.sh
# then:
#   curl -fsSL https://genome.nex-ovia.com/install.sh | sh
#
# Overrides (env):
#   GENOME_VERSION       install a specific tag        (default: latest release)
#   GENOME_INSTALL_DIR   install location              (default: /usr/local/bin or ~/.local/bin)
set -eu

REPO="nex-ovia/genome"
BIN="genome"

say()  { printf '  %s\n' "$*"; }
err()  { printf 'error: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

# --- detect platform -------------------------------------------------------
os="$(uname -s)"; arch="$(uname -m)"
case "$os" in
  Linux)
    case "$arch" in
      x86_64|amd64) target="x86_64-unknown-linux-musl" ;;
      *) err "no prebuilt Linux binary for '$arch' (have: x86_64). Build from source: https://github.com/$REPO" ;;
    esac ;;
  Darwin)
    case "$arch" in
      arm64|aarch64) target="aarch64-apple-darwin" ;;
      x86_64)        target="x86_64-apple-darwin" ;;
      *) err "no prebuilt macOS binary for '$arch'" ;;
    esac ;;
  *) err "unsupported OS '$os' — on Windows use install.ps1, otherwise build from source: https://github.com/$REPO" ;;
esac

# --- downloader (curl or wget) ---------------------------------------------
if   have curl; then dl() { curl -fsSL "$1" -o "$2"; }; fetch() { curl -fsSL "$1"; }
elif have wget; then dl() { wget -qO "$2" "$1"; };      fetch() { wget -qO- "$1"; }
else err "need curl or wget"; fi

# --- resolve version -------------------------------------------------------
tag="${GENOME_VERSION:-}"
if [ -z "$tag" ]; then
  say "resolving latest release…"
  tag="$(fetch "https://api.github.com/repos/$REPO/releases" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"
  [ -n "$tag" ] || err "could not resolve the latest release tag"
fi

asset="$BIN-$tag-$target.tar.gz"
base="https://github.com/$REPO/releases/download/$tag"
say "installing $BIN $tag ($target)"

# --- download + verify -----------------------------------------------------
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
dl "$base/$asset" "$tmp/$asset" || err "download failed: $base/$asset"
if dl "$base/$asset.sha256" "$tmp/$asset.sha256" 2>/dev/null; then
  expected="$(awk '{print $1}' "$tmp/$asset.sha256")"
  if   have sha256sum; then actual="$(sha256sum "$tmp/$asset" | awk '{print $1}')"
  elif have shasum;    then actual="$(shasum -a 256 "$tmp/$asset" | awk '{print $1}')"
  else actual=""; say "no sha256 tool; skipping verification"; fi
  [ -z "$actual" ] || [ "$actual" = "$expected" ] || err "checksum mismatch — refusing to install"
  [ -z "$actual" ] || say "checksum verified"
else
  say "checksum file unavailable; skipping verification"
fi

# --- extract ---------------------------------------------------------------
tar -xzf "$tmp/$asset" -C "$tmp" || err "extraction failed"
binpath="$tmp/$BIN-$tag-$target/$BIN"
[ -f "$binpath" ] || binpath="$(find "$tmp" -type f -name "$BIN" 2>/dev/null | head -1)"
[ -n "${binpath:-}" ] && [ -f "$binpath" ] || err "binary not found in archive"
chmod +x "$binpath"

# --- install ---------------------------------------------------------------
dir="${GENOME_INSTALL_DIR:-}"
if [ -z "$dir" ]; then
  if [ -w /usr/local/bin ]; then dir="/usr/local/bin"; else dir="$HOME/.local/bin"; fi
fi
mkdir -p "$dir"
if mv "$binpath" "$dir/$BIN" 2>/dev/null; then :
elif [ "$dir" = "/usr/local/bin" ] && have sudo; then
  say "writing to $dir (sudo)…"; sudo mv "$binpath" "$dir/$BIN"
else
  err "cannot write to $dir — set GENOME_INSTALL_DIR to a writable directory"
fi

say "installed: $dir/$BIN"
case ":$PATH:" in
  *":$dir:"*) : ;;
  *) say "note: $dir is not on your PATH. Add it:"; say "  export PATH=\"$dir:\$PATH\"" ;;
esac
say "try:  $BIN render nexovia.toml > report.html"
