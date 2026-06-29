use std::path::Path;
use std::process::ExitCode;

use balanc::diag;
use balanc::span::SourceFile;

fn main() -> ExitCode {
    let mut json = false;
    let mut emit_jvm = None;
    let mut path = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--version" || arg == "-V" {
            println!("balanc {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        } else if arg == "--json" {
            json = true;
        } else if arg == "--emit-jvm" {
            emit_jvm = match args.next() {
                Some(out) => Some(out),
                None => {
                    eprintln!("usage: balanc [--json] [--emit-jvm <out.class>] <file.bal>");
                    return ExitCode::FAILURE;
                }
            };
        } else if path.is_none() {
            path = Some(arg);
        } else {
            eprintln!("usage: balanc [--json] [--emit-jvm <out.class>] <file.bal>");
            return ExitCode::FAILURE;
        }
    }
    let path = match path {
        Some(path) => path,
        None => {
            eprintln!("usage: balanc [--json] [--emit-jvm <out.class>] <file.bal>");
            return ExitCode::FAILURE;
        }
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("balanc: cannot read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let file = SourceFile::new(path, text);
    let typed = match balanc::compile_typed(&file) {
        Ok(typed) => typed,
        Err(diags) => {
            if json {
                print!("{}", diag::render_all_json(&diags, &file));
            } else {
                eprint!("{}", diag::render_all(&diags, &file));
            }
            return ExitCode::FAILURE;
        }
    };

    if let Some(out_path) = &emit_jvm {
        let class_name = class_name_from_path(&file.name);
        let class_bytes = balanc::backend::jvm::compile(&typed, &class_name);
        if let Err(e) = std::fs::write(out_path, &class_bytes) {
            eprintln!("balanc: cannot write {out_path}: {e}");
            return ExitCode::FAILURE;
        }
    }

    let (ledger, diags) = balanc::eval::eval(&typed);
    match ledger {
        Some(ledger) => {
            let report = balanc::render::render_trial_balance(&typed, &ledger);
            if json {
                println!("{{\"ok\":true,\"report\":{}}}", diag::json_escape(&report));
            } else {
                print!("{report}");
            }
            ExitCode::SUCCESS
        }
        None => {
            if json {
                print!("{}", diag::render_all_json(&diags, &file));
            } else {
                eprint!("{}", diag::render_all(&diags, &file));
            }
            ExitCode::FAILURE
        }
    }
}

/// Derives a valid, PascalCase Java class name from a source path's stem, e.g.
/// `"examples/split_and_merge.bal"` -> `"SplitAndMerge"`.
fn class_name_from_path(path: &str) -> String {
    let stem = Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("Module");
    let mut name = String::new();
    let mut capitalize_next = true;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize_next {
                name.extend(ch.to_uppercase());
            } else {
                name.push(ch);
            }
            capitalize_next = false;
        } else {
            capitalize_next = true;
        }
    }
    if name.is_empty() || name.chars().next().unwrap().is_ascii_digit() {
        name = format!("Module{name}");
    }
    name
}
