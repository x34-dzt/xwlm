#!/bin/sh
set -eu

REPO="x34-dzt/xwlm"
BIN="xwlm"

main() {
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64) target="x86_64-unknown-linux-gnu" ;;
        aarch64|arm64) target="aarch64-unknown-linux-gnu" ;;
        *) echo "error: unsupported architecture: $arch" >&2; exit 1 ;;
    esac

    if [ "$(uname -s)" != "Linux" ]; then
        echo "error: xwlm only supports Linux (Wayland)" >&2
        exit 1
    fi

    if [ -n "${XWLM_VERSION:-}" ]; then
        tag="$XWLM_VERSION"
    else
        tag=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
            | grep '"tag_name"' | cut -d'"' -f4)
    fi

    if [ -z "$tag" ]; then
        echo "error: could not determine latest version" >&2
        exit 1
    fi

    url="https://github.com/${REPO}/releases/download/${tag}/${BIN}-${target}.tar.gz"

    install_dir="${XWLM_INSTALL_DIR:-${HOME}/.local/bin}"
    mkdir -p "$install_dir"

    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    echo "downloading ${BIN} ${tag} for ${target}..."
    curl -fsSL "$url" -o "${tmpdir}/${BIN}.tar.gz"
    tar xzf "${tmpdir}/${BIN}.tar.gz" -C "$tmpdir"
    install -m 755 "${tmpdir}/${BIN}" "${install_dir}/${BIN}"

    echo "${BIN} ${tag} installed to ${install_dir}/${BIN}"

    case ":${PATH}:" in
        *":${install_dir}:"*) ;;
        *) echo "note: add ${install_dir} to your PATH if it's not already" ;;
    esac
}

main
