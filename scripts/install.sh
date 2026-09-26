#!/bin/sh
# seeds one-line installer (seeds-d54c step 3, seeds-9e2e).
#
# Installs a prebuilt `seeds` binary from the GitHub Releases into
# ~/.local/bin (override with SEEDS_INSTALL_DIR). Version defaults to
# the latest release; pin with an argument or SEEDS_VERSION (0.3.0 or
# v0.3.0 both accepted).
#
#   curl -fsSL https://github.com/denkhaus/seeds/releases/latest/download/install.sh | sh
#
# Contract: target detection via uname (Linux/macOS, x86_64/aarch64 -
# the closed release catalog), SHASUMS.txt verification on top of TLS,
# clear errors for unsupported targets and offline. POSIX sh only.

set -eu

REPO='denkhaus/seeds'
VERSION="${SEEDS_VERSION:-${1:-latest}}"
PREFIX="${SEEDS_INSTALL_DIR:-$HOME/.local/bin}"

fail() {
    echo "error: $*" >&2
    exit 1
}

# --- target detection ----------------------------------------------------

os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
    Linux)  os='unknown-linux-musl' ;;
    Darwin) os='apple-darwin' ;;
    *)      fail "unsupported OS '$os' (supported: Linux, macOS)" ;;
esac
case "$arch" in
    x86_64 | amd64)  arch='x86_64' ;;
    aarch64 | arm64) arch='aarch64' ;;
    *)               fail "unsupported architecture '$arch' (supported: x86_64, aarch64)" ;;
esac
target="$arch-$os"

# --- resolve version -> tag ----------------------------------------------

if [ "$VERSION" = 'latest' ]; then
    # Follow the /releases/latest redirect; its final URL ends in the
    # tag name - no JSON parsing, no jq dependency.
    redirect="$(curl -fsSL -o /dev/null -w '%{url_effective}' \
        "https://github.com/$REPO/releases/latest")" \
        || fail 'cannot resolve the latest release (offline?)'
    tag="${redirect##*/tag/}"
    [ -n "$tag" ] && [ "$tag" != "$redirect" ] \
        || fail "cannot resolve the latest release tag (got: $redirect)"
else
    tag="v${VERSION#v}"
fi
version="${tag#v}"
asset="seeds-$version-$target.tar.gz"

echo "Installing seeds $version for $target into $PREFIX"

# --- download ------------------------------------------------------------

tmp="$(mktemp -d)" || fail 'mktemp failed'
trap 'rm -rf "$tmp"' EXIT INT TERM

base="https://github.com/$REPO/releases/download/$tag"
curl -fsSL -o "$tmp/$asset" "$base/$asset" \
    || fail "download failed: $asset (version $version; offline or missing asset - check https://github.com/$REPO/releases)"
curl -fsSL -o "$tmp/SHASUMS.txt" "$base/SHASUMS.txt" \
    || fail 'download failed: SHASUMS.txt'

# --- verify (on top of TLS) ----------------------------------------------

expected="$(grep "  $asset" "$tmp/SHASUMS.txt" | awk '{print $1}')"
[ -n "$expected" ] || fail "$asset is not listed in SHASUMS.txt"
if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$tmp/$asset" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$tmp/$asset" | awk '{print $1}')"
else
    fail 'no sha256 tool found (need sha256sum or shasum)'
fi
[ "$actual" = "$expected" ] || fail "checksum mismatch for $asset (expected $expected, got $actual)"

# --- install -------------------------------------------------------------

tar -xzf "$tmp/$asset" -C "$tmp" || fail 'tar extraction failed'
[ -f "$tmp/seeds" ] || fail 'archive did not contain a seeds binary at its root'
mkdir -p "$PREFIX" || fail "cannot create $PREFIX"
mv -f "$tmp/seeds" "$PREFIX/seeds" || fail "cannot install into $PREFIX/seeds"
chmod +x "$PREFIX/seeds"

# --- report --------------------------------------------------------------

echo "Installed: $PREFIX/seeds"
case ":$PATH:" in
    *":$PREFIX:"*) ;;
    *) echo "note: $PREFIX is not on your PATH - add it to use 'seeds'." ;;
esac
"$PREFIX/seeds" --version
