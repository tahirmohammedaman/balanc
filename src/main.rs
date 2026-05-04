use std::process::ExitCode;

fn main() -> ExitCode {
    let path = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: balanc <file.bal>");
            return ExitCode::FAILURE;
        }
    };

    // The pipeline (lex -> parse -> check -> eval -> print) lands in slice 0.
    eprintln!("balanc: nothing to run yet: {path}");
    ExitCode::FAILURE
}
