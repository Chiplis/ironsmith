#!/usr/bin/env bash
# Shared wasm-opt resolver for rebuild-wasm.sh and scripts/bench-cold-build.sh.
#
# The shipped optimizer is a pinned, checksum-verified Binaryen release that is
# installed natively for the host architecture. We deliberately do not reuse the
# binary that wasm-pack caches: wasm-pack 0.13 fetches Binaryen version_117 and
# only ships an x86_64 build for macOS, so on Apple Silicon it runs under
# Rosetta. That binary has a non-deterministic heap-corruption crash during
# teardown ("malloc: pointer being freed was not allocated" / Abort trap 6) that
# aborts release packaging even after it has already written valid output.
#
# Override with IRONSMITH_WASM_OPT=/path/to/wasm-opt (WASM_OPT is accepted as an
# alias for the bench script's historical flag). IRONSMITH_TOOLS_CACHE_DIR moves
# the install root.

IRONSMITH_BINARYEN_VERSION="version_132"

ironsmith_binaryen_sha256() {
  case "$1" in
    arm64-macos)   printf '%s\n' "98aad827847af7ef990ed7098d885725c8e5b5aae75073403635617ae4e259aa" ;;
    x86_64-macos)  printf '%s\n' "40c3de90bb3766bd0282a895e139a6f50253dba49b4f5bb89e66faca162d832e" ;;
    aarch64-linux) printf '%s\n' "c58562417836c5d0493d89bdefc434933bdc097db641b483df86bcfa557a107f" ;;
    x86_64-linux)  printf '%s\n' "195ddc94f9bc89f45abdabb0b9eea86023d727ba90eac8b35b80f2544fc30572" ;;
    *) return 1 ;;
  esac
}

ironsmith_binaryen_asset() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os:$arch" in
    Darwin:arm64)                 printf '%s\n' "arm64-macos" ;;
    Darwin:x86_64)                printf '%s\n' "x86_64-macos" ;;
    Linux:aarch64|Linux:arm64)    printf '%s\n' "aarch64-linux" ;;
    Linux:x86_64)                 printf '%s\n' "x86_64-linux" ;;
    *)
      echo "[ERROR] no pinned Binaryen $IRONSMITH_BINARYEN_VERSION build for $os/$arch" >&2
      return 1
      ;;
  esac
}

ironsmith_tools_cache_dir() {
  if [[ -n "${IRONSMITH_TOOLS_CACHE_DIR:-}" ]]; then
    printf '%s\n' "$IRONSMITH_TOOLS_CACHE_DIR"
  elif [[ "$(uname -s)" == "Darwin" ]]; then
    printf '%s\n' "$HOME/Library/Caches/ironsmith"
  else
    printf '%s\n' "${XDG_CACHE_HOME:-$HOME/.cache}/ironsmith"
  fi
}

ironsmith_sha256_file() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    echo "[ERROR] need shasum or sha256sum to verify the Binaryen download" >&2
    return 1
  fi
}

# Prints the path of the pinned wasm-opt, downloading and verifying it on first use.
ironsmith_install_pinned_wasm_opt() {
  local asset expected_sha install_root install_dir wasm_opt url tmp_dir tarball actual_sha
  asset="$(ironsmith_binaryen_asset)" || return 1
  expected_sha="$(ironsmith_binaryen_sha256 "$asset")" || {
    echo "[ERROR] no pinned checksum for Binaryen asset $asset" >&2
    return 1
  }
  install_root="$(ironsmith_tools_cache_dir)"
  install_dir="$install_root/binaryen-$IRONSMITH_BINARYEN_VERSION-$asset"
  wasm_opt="$install_dir/bin/wasm-opt"
  if [[ -x "$wasm_opt" ]]; then
    printf '%s\n' "$wasm_opt"
    return 0
  fi

  command -v curl >/dev/null 2>&1 || {
    echo "[ERROR] curl is required to download Binaryen $IRONSMITH_BINARYEN_VERSION; or set IRONSMITH_WASM_OPT" >&2
    return 1
  }
  url="https://github.com/WebAssembly/binaryen/releases/download/$IRONSMITH_BINARYEN_VERSION/binaryen-$IRONSMITH_BINARYEN_VERSION-$asset.tar.gz"
  mkdir -p "$install_root"
  tmp_dir="$(mktemp -d "$install_root/.binaryen-download.XXXXXX")"
  tarball="$tmp_dir/binaryen.tar.gz"
  echo "[INFO] downloading Binaryen $IRONSMITH_BINARYEN_VERSION ($asset) to $install_dir" >&2
  if ! curl -fsSL --retry 3 -o "$tarball" "$url"; then
    rm -rf -- "$tmp_dir"
    echo "[ERROR] failed to download $url; set IRONSMITH_WASM_OPT to a local wasm-opt to skip the download" >&2
    return 1
  fi
  actual_sha="$(ironsmith_sha256_file "$tarball")" || { rm -rf -- "$tmp_dir"; return 1; }
  if [[ "$actual_sha" != "$expected_sha" ]]; then
    rm -rf -- "$tmp_dir"
    echo "[ERROR] Binaryen checksum mismatch for $asset: expected $expected_sha, got $actual_sha" >&2
    return 1
  fi
  if ! tar -xzf "$tarball" -C "$tmp_dir"; then
    rm -rf -- "$tmp_dir"
    echo "[ERROR] failed to extract $tarball" >&2
    return 1
  fi
  if [[ ! -x "$tmp_dir/binaryen-$IRONSMITH_BINARYEN_VERSION/bin/wasm-opt" ]]; then
    rm -rf -- "$tmp_dir"
    echo "[ERROR] Binaryen archive did not contain bin/wasm-opt" >&2
    return 1
  fi
  rm -rf -- "$install_dir"
  mv "$tmp_dir/binaryen-$IRONSMITH_BINARYEN_VERSION" "$install_dir"
  rm -rf -- "$tmp_dir"
  printf '%s\n' "$wasm_opt"
}

# Resolves the wasm-opt binary to use: an explicit override, else the pinned install.
resolve_wasm_opt() {
  local override="${IRONSMITH_WASM_OPT:-${WASM_OPT:-}}"
  if [[ -n "$override" ]]; then
    if [[ ! -x "$override" ]]; then
      echo "[ERROR] IRONSMITH_WASM_OPT/WASM_OPT is set but not executable: $override" >&2
      return 1
    fi
    printf '%s\n' "$override"
    return 0
  fi
  ironsmith_install_pinned_wasm_opt
}

# Returns success when the given wasm-opt reports the pinned Binaryen version.
wasm_opt_matches_pin() {
  [[ "$("$1" --version 2>/dev/null)" == *"($IRONSMITH_BINARYEN_VERSION)"* ]]
}
