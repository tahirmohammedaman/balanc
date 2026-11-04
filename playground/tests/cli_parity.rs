//! The playground must print exactly what the CLI prints. These tests build the
//! real `balanc` binary and compare its stdout with the WASM entry points'
//! native builds, byte for byte.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use balanc::span::SourceFile;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// Builds the `balanc` CLI into a private target dir (so the nested cargo never
/// waits on the lock held by the running `cargo test`) and returns its path.
fn cli() -> &'static Path {
    static CLI: OnceLock<PathBuf> = OnceLock::new();
    CLI.get_or_init(|| {
        let target = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli");
        let status = Command::new(env!("CARGO"))
            .args(["build", "--quiet", "-p", "balanc", "--bin", "balanc", "--target-dir"])
            .arg(&target)
            .current_dir(repo_root())
            .status()
            .expect("run cargo build for the balanc CLI");
        assert!(status.success(), "building the balanc CLI failed");
        target.join("debug").join(format!("balanc{}", std::env::consts::EXE_SUFFIX))
    })
}

fn cli_stdout(args: &[&str], cwd: &Path) -> String {
    let out = Command::new(cli()).args(args).current_dir(cwd).output().expect("run balanc");
    String::from_utf8(out.stdout).expect("balanc prints UTF-8")
}

fn cli_stderr(args: &[&str], cwd: &Path) -> String {
    let out = Command::new(cli()).args(args).current_dir(cwd).output().expect("run balanc");
    String::from_utf8(out.stderr).expect("balanc prints UTF-8")
}

/// `check()` names its input `playground.bal`, so run the CLI on a copy with
/// that name: the two outputs must then be identical, `"file"` field included.
#[test]
fn check_matches_cli_json_for_coffee_twice() {
    let src = std::fs::read_to_string(repo_root().join("examples/coffee_twice.bal")).unwrap();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("parity-coffee-twice");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("playground.bal"), &src).unwrap();

    let cli = cli_stdout(&["--json", "playground.bal"], &dir);
    let wasm = balanc_playground::check(&src);
    assert_eq!(wasm.as_bytes(), cli.as_bytes());
    assert!(wasm.contains("\"code\":\"E_REUSED\""), "expected E_REUSED in {wasm}");

    // And under the example's own path, the output is the CLI's for the repo file.
    let file = SourceFile::new("examples/coffee_twice.bal".to_string(), src);
    let cli = cli_stdout(&["--json", "examples/coffee_twice.bal"], &repo_root());
    assert_eq!(balanc_playground::json_for(&file).as_bytes(), cli.as_bytes());
}

fn bal_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            bal_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "bal") {
            out.push(path);
        }
    }
}

/// Every example, accepted or rejected, at every stage (lex through eval):
/// the JSON matches `--json` and the text matches stdout + stderr.
#[test]
fn every_example_matches_the_cli() {
    let root = repo_root();
    let mut files = Vec::new();
    bal_files(&root.join("examples"), &mut files);
    bal_files(&root.join("tests/cases"), &mut files);
    files.sort();
    assert!(files.len() > 10, "found only {} programs", files.len());

    for path in files {
        let rel = path.strip_prefix(&root).unwrap().to_str().unwrap().replace('\\', "/");
        let file = SourceFile::new(rel.clone(), std::fs::read_to_string(&path).unwrap());

        let json = cli_stdout(&["--json", &rel], &root);
        assert_eq!(balanc_playground::json_for(&file), json, "--json differs for {rel}");

        let text = cli_stdout(&[&rel], &root) + &cli_stderr(&[&rel], &root);
        assert_eq!(balanc_playground::pretty_for(&file), text, "text differs for {rel}");
    }
}
