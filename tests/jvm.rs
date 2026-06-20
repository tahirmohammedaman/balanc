//! Slice 7 proof tests for the JVM backend (PLAN.md): differential testing against
//! the interpreter over the corpus's successful (diagnostic-free) cases, `java
//! -Xverify:all` acceptance, a `javap -c` legibility smoke check, and a packaged
//! `.jar` callable from an external Java program with no special classpath tricks.
//! Shells out to `javac`/`java`/`jar` directly — this crate stays zero-dependency
//! (D-039/D-041), and the confirmed toolchain (PLAN.md OQ-8) puts all three on `PATH`
//! wherever `cargo test` itself runs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use balanc::backend::jvm;
use balanc::lex::lex;
use balanc::parse::parse;
use balanc::resolve::resolve;
use balanc::span::SourceFile;
use balanc::typeck::{typeck, TModule};

fn typed_module(src: &str) -> TModule {
    let (tokens, diags) = lex(src);
    assert!(diags.is_empty(), "lex failed: {diags:?}");
    let (module, diags) = parse(&tokens.unwrap());
    assert!(diags.is_empty(), "parse failed: {diags:?}");
    let (resolved, diags) = resolve(module.unwrap());
    assert!(diags.is_empty(), "resolve failed: {diags:?}");
    let (typed, diags) = typeck(resolved.unwrap());
    assert!(diags.is_empty(), "typeck failed: {diags:?}");
    typed.unwrap()
}

fn interpreter_report(name: &str, src: &str) -> String {
    let file = SourceFile::new(format!("{name}.bal"), src.to_string());
    balanc::run(&file).unwrap_or_else(|diags| panic!("interpreter run failed for {name}: {diags:?}"))
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A fresh scratch directory this test owns exclusively — no `tempfile` crate
/// (zero-dependency project, D-039/D-041), so this rolls its own from `std::env::
/// temp_dir()` plus a pid+counter suffix, and removes any pre-existing junk with the
/// same name defensively before use.
fn scratch_dir(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("balanc-jvm-test-{tag}-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_ok(cmd: &mut Command) -> String {
    let out = cmd.output().unwrap_or_else(|e| panic!("failed to spawn {cmd:?}: {e}"));
    if !out.status.success() {
        panic!(
            "{cmd:?} failed ({}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8(out.stdout).expect("non-utf8 stdout")
}

fn runtime_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/balanc/runtime/Ledger.java")
}

fn case_source(case_name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases").join(format!("{case_name}.bal"));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {path:?}: {e}"))
}

/// Compiles `case_name` with both the interpreter and the JVM backend, runs the
/// generated class's `main` under `java -Xverify:all`, and asserts the two outputs
/// are byte-for-byte identical — Slice 7's core proof test. Returns the scratch
/// directory (with the class files and the built `.jar` in it) so callers that need
/// to check more (javap, external-jar-caller) don't have to redo the setup.
fn build_and_run(case_name: &str, class_name: &str) -> PathBuf {
    let src = case_source(case_name);
    let expected = interpreter_report(case_name, &src);
    let typed = typed_module(&src);
    let class_bytes = jvm::compile(&typed, class_name);

    let dir = scratch_dir(case_name);
    fs::write(dir.join(format!("{class_name}.class")), &class_bytes).unwrap();
    run_ok(Command::new("javac").arg("-d").arg(&dir).arg(runtime_source()));

    let actual = run_ok(Command::new("java").arg("-Xverify:all").arg("-cp").arg(&dir).arg(class_name));
    assert_eq!(actual, expected, "JVM backend output differs from the interpreter for '{case_name}'");

    run_ok(Command::new("jar")
        .current_dir(&dir)
        .arg("cf")
        .arg("out.jar")
        .arg(format!("{class_name}.class"))
        .arg("balanc"));

    dir
}

#[test]
fn coffee_matches_the_interpreter() {
    build_and_run("coffee", "Coffee");
}

#[test]
fn multi_currency_matches_the_interpreter() {
    build_and_run("multi_currency", "MultiCurrency");
}

#[test]
fn convert_and_absorb_matches_the_interpreter() {
    build_and_run("convert_and_absorb", "ConvertAndAbsorb");
}

#[test]
fn split_and_merge_matches_the_interpreter() {
    build_and_run("split_and_merge", "SplitAndMerge");
}

/// `javap -c` disassembly is legible and shows the expected shape of opcodes for a
/// hand-traced example — an independent check against the real JDK, not just this
/// writer's own internal consistency (PLAN.md Slice 7 proof test).
#[test]
fn javap_disassembly_is_legible_for_coffee() {
    let dir = build_and_run("coffee", "Coffee");
    let out = run_ok(Command::new("javap").arg("-c").arg("-p").arg("-cp").arg(&dir).arg("Coffee"));
    assert!(out.contains("Code:"), "javap output missing Code: sections:\n{out}");
    assert!(out.contains("public Coffee()"), "javap output missing constructor:\n{out}");
    assert!(out.contains("coffeeTxn"), "javap output missing the coffee txn method:\n{out}");
    assert!(out.contains("invokevirtual"), "javap output missing invokevirtual calls:\n{out}");
    assert!(out.contains("getfield"), "javap output missing getfield (the ledger field):\n{out}");
}

/// The packaged `.jar`'s public API is callable from a small external Java program
/// with no special classpath/reflection tricks — the concrete proof behind D-043's
/// "usable from any Java/Spring Boot app" claim (PLAN.md Slice 7 proof test).
#[test]
fn packaged_jar_is_callable_from_an_external_java_program() {
    let case_name = "coffee";
    let class_name = "Coffee";
    let dir = build_and_run(case_name, class_name);
    let expected = interpreter_report(case_name, &case_source(case_name));

    let ext_dir = dir.join("external_caller");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(
        ext_dir.join("Main.java"),
        format!(
            r#"public class Main {{
    public static void main(String[] args) {{
        {class_name} module = new {class_name}();
        System.out.print(module.run());
    }}
}}
"#
        ),
    )
    .unwrap();

    let jar_path = dir.join("out.jar");
    run_ok(Command::new("javac")
        .arg("-cp")
        .arg(&jar_path)
        .arg("-d")
        .arg(&ext_dir)
        .arg(ext_dir.join("Main.java")));

    let classpath = format!("{}:{}", jar_path.display(), ext_dir.display());
    let actual = run_ok(Command::new("java").arg("-cp").arg(&classpath).arg("Main"));
    assert_eq!(actual, expected, "external caller's output differs from the interpreter");
}
