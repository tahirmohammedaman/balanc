#!/usr/bin/env bash
# Builds the balanc playground to WebAssembly and writes the JS bindings the
# docs site lazy-loads, into docs/src/playground/pkg/ (gitignored).
#
#   playground/build.sh            # from anywhere in the repo
#   npm run build:wasm             # the same, from docs/
#
# Needs the wasm32-unknown-unknown target (`rustup target add wasm32-unknown-unknown`)
# and wasm-bindgen-cli at exactly the wasm-bindgen version in Cargo.lock. If no
# matching `wasm-bindgen` is on PATH, the release binary for this machine is
# downloaded into target/ (or install it: `cargo install wasm-bindgen-cli --version <v> --locked`).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

version="$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/version = |"/, ""); print; exit }' Cargo.lock)"
if [[ -z "$version" ]]; then
  echo "build.sh: wasm-bindgen not found in Cargo.lock" >&2
  exit 1
fi

bindgen="wasm-bindgen"
if ! command -v wasm-bindgen >/dev/null || [[ "$(wasm-bindgen --version)" != "wasm-bindgen $version" ]]; then
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) triple="x86_64-unknown-linux-musl" ;;
    Linux-aarch64) triple="aarch64-unknown-linux-gnu" ;;
    Darwin-x86_64) triple="x86_64-apple-darwin" ;;
    Darwin-arm64) triple="aarch64-apple-darwin" ;;
    *)
      echo "build.sh: install wasm-bindgen-cli $version: cargo install wasm-bindgen-cli --version $version --locked" >&2
      exit 1
      ;;
  esac
  dir="target/wasm-bindgen-cli/wasm-bindgen-$version-$triple"
  if [[ ! -x "$dir/wasm-bindgen" ]]; then
    echo "build.sh: downloading wasm-bindgen-cli $version ($triple)" >&2
    mkdir -p target/wasm-bindgen-cli
    curl -sSfL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$version/wasm-bindgen-$version-$triple.tar.gz" \
      | tar -xz -C target/wasm-bindgen-cli
  fi
  bindgen="$dir/wasm-bindgen"
fi

cargo build -p balanc-playground --release --target wasm32-unknown-unknown
"$bindgen" --target web --no-typescript --out-dir docs/src/playground/pkg \
  target/wasm32-unknown-unknown/release/balanc_playground.wasm

ls -l docs/src/playground/pkg
