//! The balanc compiler, compiled to WebAssembly for the docs-site playground.
//!
//! The core `balanc` crate stays dependency-free; this crate is the only place
//! `wasm-bindgen` appears. Every entry point mirrors what `src/main.rs` does for
//! one file, so the playground prints exactly what the CLI would.

use balanc::diag;
use balanc::span::SourceFile;
use wasm_bindgen::prelude::wasm_bindgen;

/// The name every playground snippet is compiled under; it appears in
/// diagnostics as `--> playground.bal:L:C` and in the JSON `"file"` field.
pub const PLAYGROUND_FILE: &str = "playground.bal";

/// What `balanc --json <file>` prints to stdout for `file`, byte for byte.
pub fn json_for(file: &SourceFile) -> String {
    let typed = match balanc::compile_typed(file) {
        Ok(typed) => typed,
        Err(diags) => return diag::render_all_json(&diags, file),
    };
    let (ledger, diags) = balanc::eval::eval(&typed);
    match ledger {
        Some(ledger) => {
            let report = balanc::render::render_trial_balance(&typed, &ledger);
            format!("{{\"ok\":true,\"report\":{}}}\n", diag::json_escape(&report))
        }
        None => diag::render_all_json(&diags, file),
    }
}

/// What `balanc <file>` prints for `file`: the trial balance (stdout) when the
/// program type-checks and evaluates, otherwise the rustc-style diagnostics
/// (stderr). Same pipeline as `balanc::run`.
pub fn pretty_for(file: &SourceFile) -> String {
    match balanc::run(file) {
        Ok(report) => report,
        Err(diags) => diag::render_all(&diags, file),
    }
}

fn playground_file(source: &str) -> SourceFile {
    SourceFile::new(PLAYGROUND_FILE.to_string(), source.to_string())
}

/// Type-checks (and, if typing succeeds, evaluates) `source` and returns the
/// JSON `balanc --json playground.bal` would print: `{"ok":false,"diagnostics":[…]}`
/// or `{"ok":true,"report":"…"}`.
#[wasm_bindgen]
pub fn check(source: &str) -> String {
    json_for(&playground_file(source))
}

/// The plain-text output of `balanc playground.bal`: rustc-style diagnostics,
/// or the interpreter's trial-balance report when the program is accepted.
#[wasm_bindgen]
pub fn check_pretty(source: &str) -> String {
    pretty_for(&playground_file(source))
}

