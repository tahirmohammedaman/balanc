use std::process::ExitCode;

use balanc::span::SourceFile;

fn main() -> ExitCode {
    let path = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: balanc <file.bal>");
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
            print!("{report}");
            ExitCode::SUCCESS
        }
        Err(diags) => {
            eprint!("{diags}");
            ExitCode::FAILURE
        }
    }
}
