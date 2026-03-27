#!/bin/sh
set -e

REPO="dstanek/am"
BINARY="am"
INSTALL_DIR="${AM_INSTALL_DIR:-$HOME/.local/bin}"

# --- helpers -----------------------------------------------------------------

say() { printf '%s\n' "$*"; }
err() { printf 'error: %s\n' "$*" >&2; exit 1; }

need() {
    command -v "$1" >/dev/null 2>&1 || err "required command not found: $1"
}

# --- detect platform ---------------------------------------------------------

detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64)  echo "x86_64-unknown-linux-gnu" ;;
                aarch64) echo "aarch64-unknown-linux-gnu" ;;
                arm64)   echo "aarch64-unknown-linux-gnu" ;;
                *)       err "unsupported Linux architecture: $arch" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64) echo "x86_64-apple-darwin" ;;
                arm64)  echo "aarch64-apple-darwin" ;;
                *)      err "unsupported macOS architecture: $arch" ;;
            esac
            ;;
        *)
            err "unsupported operating system: $os (Windows users: download am-x86_64-pc-windows-msvc.zip from https://github.com/$REPO/releases/latest)"
            ;;
    esac
}

# --- download with verification ----------------------------------------------

download() {
    local url="$1" dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$dest" "$url"
    else
        err "curl or wget is required"
    fi
}

verify_checksum() {
    local file="$1" checksum_file="$2"
    local expected actual

    expected="$(cut -d' ' -f1 < "$checksum_file")"

    if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "$file" | cut -d' ' -f1)"
    elif command -v shasum >/dev/null 2>&1; then
        actual="$(shasum -a 256 "$file" | cut -d' ' -f1)"
    else
        say "warning: cannot verify checksum (sha256sum/shasum not found), skipping"
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        err "checksum mismatch for $file\n  expected: $expected\n  actual:   $actual"
    fi
}

# --- main --------------------------------------------------------------------

main() {
    local target archive_name archive_url checksum_url tmpdir

    target="$(detect_target)"
    archive_name="${BINARY}-${target}.tar.gz"
    archive_url="https://github.com/${REPO}/releases/latest/download/${archive_name}"
    checksum_url="${archive_url}.sha256"

    say "installing $BINARY for $target"
    say "  from: https://github.com/$REPO/releases/latest"
    say "  to:   $INSTALL_DIR"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    say ""
    say "downloading $archive_name ..."
    download "$archive_url" "$tmpdir/$archive_name"

    say "verifying checksum ..."
    download "$checksum_url" "$tmpdir/${archive_name}.sha256"
    verify_checksum "$tmpdir/$archive_name" "$tmpdir/${archive_name}.sha256"

    say "extracting ..."
    tar -xzf "$tmpdir/$archive_name" -C "$tmpdir"

    mkdir -p "$INSTALL_DIR"
    mv "$tmpdir/$BINARY" "$INSTALL_DIR/$BINARY"
    chmod +x "$INSTALL_DIR/$BINARY"

    say ""
    say "$BINARY installed to $INSTALL_DIR/$BINARY"

    # Warn if install dir is not in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            say ""
            say "warning: $INSTALL_DIR is not in your PATH"
            say "  add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            say "    export PATH=\"\$PATH:$INSTALL_DIR\""
            ;;
    esac
}

main "$@"
