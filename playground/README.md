# balanc-playground

The balanc compiler compiled to WebAssembly, for the docs site's `/playground`
route. The core `balanc` crate stays dependency-free: this is a separate
workspace member that depends on it by path and adds only `wasm-bindgen`.

## API

| export | returns |
| --- | --- |
| `check(source)` | exactly what `balanc --json playground.bal` prints: `{"ok":false,"diagnostics":[…]}` or `{"ok":true,"report":"…"}` |
| `check_pretty(source)` | exactly what `balanc playground.bal` prints: rustc-style diagnostics, or the interpreter's trial balance |

Both run the CLI's pipeline (`compile_typed`, then `eval` and
`render_trial_balance`), so evaluation-time codes such as `E_UNBALANCED` appear
too. The snippet is always named `playground.bal`.

## Build

The toolchain is plain cargo plus `wasm-bindgen-cli` (no wasm-pack):

```
rustup target add wasm32-unknown-unknown
playground/build.sh
```

`build.sh` builds `--release --target wasm32-unknown-unknown`, then runs
`wasm-bindgen --target web` into `docs/src/playground/pkg/`. The CLI must match
the `wasm-bindgen` version in `Cargo.lock` (pinned with `=` in `Cargo.toml`);
if no matching `wasm-bindgen` is on `PATH`, the script downloads that release's
binary into `target/`.

## Test

```
cargo test -p balanc-playground
```

builds the real CLI and checks, byte for byte, that `check()` on
`examples/coffee_twice.bal` equals `balanc --json` on the same file, and that
both entry points match the CLI for every program under `examples/` and
`tests/cases/`.
