//! Exercises the `balanc` binary itself (argument parsing, `--json`), as opposed to
//! `e2e.rs`, which drives `balanc::run` directly. No JSON parser here — this project
//! has no dependencies at all, and adding one just to check the shape of `--json`'s
//! own output would be backwards — so these checks confirm key substrings and
//! prefixes, not full structural parsing.

use std::path::Path;
use std::process::Command;

fn run(args: &[&str]) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_balanc"))
        .args(args)
        .output()
        .expect("failed to run the balanc binary");
    (output.status.success(), String::from_utf8(output.stdout).unwrap())
}

fn case(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cases")
        .join(format!("{name}.bal"))
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
fn json_flag_wraps_a_successful_report() {
    let (ok, stdout) = run(&["--json", &case("coffee")]);
    assert!(ok);
    assert!(stdout.starts_with("{\"ok\":true,\"report\":"));
    assert!(stdout.contains("Trial Balance"));
}

#[test]
fn json_flag_wraps_diagnostics_with_stable_codes_and_locations() {
    let (ok, stdout) = run(&["--json", &case("dropped")]);
    assert!(!ok);
    assert!(stdout.starts_with("{\"ok\":false,\"diagnostics\":["));
    assert!(stdout.contains("\"code\":\"E_DROPPED\""));
    assert!(stdout.contains("\"line\":"));
    assert!(stdout.contains("\"col\":"));
}

#[test]
fn plain_mode_still_prints_human_readable_diagnostics_to_stderr() {
    let output = Command::new(env!("CARGO_BIN_EXE_balanc"))
        .arg(case("dropped"))
        .output()
        .expect("failed to run the balanc binary");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("error[E_DROPPED]"));
    assert!(stderr.contains(" --> "));
}

#[test]
fn missing_file_argument_prints_usage_and_fails() {
    let output =
        Command::new(env!("CARGO_BIN_EXE_balanc")).output().expect("failed to run the balanc binary");
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().contains("usage:"));
}
