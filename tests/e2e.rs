//! End-to-end pipeline tests. Each case is a `.bal` file under `tests/cases/`; its
//! expected output (trial balance or diagnostics) is snapshotted under
//! `tests/snapshots/`. Regenerate with `BALANC_BLESS=1 cargo test` — never hand-edit a
//! snapshot to make a test pass.

use std::fs;
use std::path::Path;

use balanc::span::SourceFile;

fn run_case(name: &str) {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases");
    let snapshots_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");

    let src_path = cases_dir.join(format!("{name}.bal"));
    let text =
        fs::read_to_string(&src_path).unwrap_or_else(|e| panic!("reading {src_path:?}: {e}"));
    let file = SourceFile::new(format!("{name}.bal"), text);
    let output = match balanc::run(&file) {
        Ok(report) => report,
        Err(diags) => diags,
    };

    let snapshot_path = snapshots_dir.join(format!("{name}.txt"));
    if std::env::var("BALANC_BLESS").is_ok() {
        fs::write(&snapshot_path, &output).unwrap();
        return;
    }

    let expected = fs::read_to_string(&snapshot_path).unwrap_or_else(|e| {
        panic!("reading snapshot {snapshot_path:?}: {e} (run with BALANC_BLESS=1 to create it)")
    });
    assert_eq!(output, expected, "snapshot mismatch for '{name}' (run with BALANC_BLESS=1 to update)");
}

#[test]
fn coffee_prints_trial_balance() {
    run_case("coffee");
}

#[test]
fn dropped_value_reports_e_dropped() {
    run_case("dropped");
}

#[test]
fn reused_value_reports_e_reused() {
    run_case("reused");
}

#[test]
fn currency_mismatch_reports_e_currency_mismatch() {
    run_case("currency_mismatch");
}

#[test]
fn multi_currency_prints_one_section_per_currency() {
    run_case("multi_currency");
}

#[test]
fn undeclared_account_reports_e_undeclared_account() {
    run_case("undeclared_account");
}

#[test]
fn unknown_currency_reports_e_unknown_currency() {
    run_case("unknown_currency");
}

#[test]
fn too_many_fraction_digits_reports_the_currency_and_scale() {
    run_case("too_many_fraction_digits");
}

#[test]
fn convert_and_absorb_round_trip_cleanly() {
    run_case("convert_and_absorb");
}

#[test]
fn dropped_residue_reports_e_dropped() {
    run_case("dropped_residue");
}

#[test]
fn convert_currency_mismatch_reports_e_currency_mismatch() {
    run_case("convert_currency_mismatch");
}

#[test]
fn debiting_a_residue_reports_e_expected_money() {
    run_case("expected_money_found_residue");
}

#[test]
fn absorbing_money_reports_e_expected_residue() {
    run_case("expected_residue_found_money");
}
