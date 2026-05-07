//! balanc pipeline: source text -> lex -> parse -> resolve -> typeck -> eval -> render.
//!
//! [`run`] is the single entry point both the `main` binary and the end-to-end tests
//! drive, so the two can never disagree about what a given source file produces.

pub mod amount;
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

/// Runs the full pipeline over one source file. `Ok` is the rendered trial balance;
/// `Err` is the rendered diagnostics that stopped the pipeline, sorted by span.
pub fn run(file: &SourceFile) -> Result<String, String> {
    let (tokens, diags) = lex::lex(&file.text);
    let tokens = match tokens {
        Some(tokens) => tokens,
        None => return Err(render_diags(diags, file)),
    };

    let (module, diags) = parse::parse(&tokens);
    let module = match module {
        Some(module) => module,
        None => return Err(render_diags(diags, file)),
    };

    let (resolved, diags) = resolve::resolve(module);
    let resolved = match resolved {
        Some(resolved) => resolved,
        None => return Err(render_diags(diags, file)),
    };

    let (typed, diags) = typeck::typeck(resolved);
    let typed = match typed {
        Some(typed) => typed,
        None => return Err(render_diags(diags, file)),
    };

    let ledger = eval::eval(&typed);
    Ok(render::render_trial_balance(&typed, &ledger))
}

fn render_diags(mut diags: Vec<Diagnostic>, file: &SourceFile) -> String {
    sort_by_span(&mut diags);
    let mut out = diags.iter().map(|d| d.render(file)).collect::<Vec<_>>().join("\n");
    out.push('\n');
    out
}
