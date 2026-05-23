use std::process::ExitCode;

use balanc::diag;
use balanc::span::SourceFile;

fn main() -> ExitCode {
    let mut json = false;
    let mut path = None;
    for arg in std::env::args().skip(1) {
        if arg == "--json" {
            json = true;
        } else if path.is_none() {
            path = Some(arg);
        } else {
            eprintln!("usage: balanc [--json] <file.bal>");
            return ExitCode::FAILURE;
        }
    }
    let path = match path {
        Some(path) => path,
        None => {
            eprintln!("usage: balanc [--json] <file.bal>");
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
    match balanc::run(&file) {
        Ok(report) => {
            if json {
                println!("{{\"ok\":true,\"report\":{}}}", diag::json_escape(&report));
            } else {
                print!("{report}");
            }
            ExitCode::SUCCESS
        }
        Err(diags) => {
            if json {
                print!("{}", diag::render_all_json(&diags, &file));
            } else {
                eprint!("{}", diag::render_all(&diags, &file));
            }
            ExitCode::FAILURE
        }
    }
}
