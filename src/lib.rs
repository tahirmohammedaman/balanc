//! balanc pipeline: source text -> lex -> parse -> resolve -> typeck -> eval -> render.
//!
//! [`run`] is the single entry point both the `main` binary and the end-to-end tests
//! drive, so the two can never disagree about what a given source file produces.

pub mod amount;
pub mod backend;
pub mod diag;
pub mod eval;
pub mod lex;
pub mod parse;
pub mod render;
pub mod resolve;
pub mod span;
pub mod typeck;

use diag::{sort_by_span, Diagnostic};
use span::SourceFile;

/// Runs the pipeline through `typeck` only — the shared front end both `run` (the
/// interpreter) and the JVM backend (`backend::jvm::compile`, driven from `main`)
/// build on; the two diverge only in what they do with the resulting `TModule`.
pub fn compile_typed(file: &SourceFile) -> Result<typeck::TModule, Vec<Diagnostic>> {
    let (tokens, diags) = lex::lex(&file.text);
    let tokens = match tokens {
        Some(tokens) => tokens,
        None => return Err(sorted(diags)),
    };

    let (module, diags) = parse::parse(&tokens);
    let module = match module {
        Some(module) => module,
        None => return Err(sorted(diags)),
    };

    let (resolved, diags) = resolve::resolve(module);
    let resolved = match resolved {
        Some(resolved) => resolved,
        None => return Err(sorted(diags)),
    };

    let (typed, diags) = typeck::typeck(resolved);
    match typed {
        Some(typed) => Ok(typed),
        None => Err(sorted(diags)),
    }
}

/// Runs the full pipeline over one source file. `Ok` is the rendered trial balance;
/// `Err` is the diagnostics that stopped the pipeline, sorted by span — callers render
/// them as plain text (`diag::render_all`) or JSON (`diag::render_all_json`).
pub fn run(file: &SourceFile) -> Result<String, Vec<Diagnostic>> {
    let typed = compile_typed(file)?;
    let (ledger, diags) = eval::eval(&typed);
    let ledger = match ledger {
        Some(ledger) => ledger,
        None => return Err(sorted(diags)),
    };

    Ok(render::render_trial_balance(&typed, &ledger))
}

fn sorted(mut diags: Vec<Diagnostic>) -> Vec<Diagnostic> {
    sort_by_span(&mut diags);
    diags
}
