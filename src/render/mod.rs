//! `LedgerState` -> report text. The only stage besides `main` allowed to produce
//! output for a human; `lex`/`parse`/`check`/`eval` stay pure (invariant 7).
//!
//! One section per currency with at least one posting (Slice 2): accounts are grouped
//! by their declared currency, and each section's amounts are formatted at that
//! currency's own scale. A currency with no postings in this run gets no section — an
//! empty section would just be noise.
//!
//! Within a section (Slice 5), accounts are listed in `AccountKind` order (asset,
//! liability, equity, income, expense — balance-sheet then income-statement order),
//! and each displayed amount is the account's raw `LedgerState` balance flipped to its
//! `normal` sign: unchanged for a debit-normal account, negated for a credit-normal
//! one. This is a *display* convention only — `eval`'s raw arithmetic (debit adds,
//! credit subtracts, uniformly, regardless of any account's kind or normal balance)
//! is unchanged from Slice 0.
//!
//! The two subtotals ("Debit total", "Credit total") are the accounting equation, made
//! visible. For a module where every account's currency is closed under its own
//! transactions (nothing moves value across a currency boundary), the two are always
//! equal: writing `signed(x) = raw(x)` for a debit-normal account and `-raw(x)` for a
//! credit-normal one, `debit_total - credit_total = Σ_debit-normal signed(x) -
//! Σ_credit-normal signed(x) = Σ_debit-normal raw(x) + Σ_credit-normal raw(x) =
//! Σ_all-accounts-of-this-currency raw(x)`, which is `0` whenever nothing but
//! same-currency `debit`/`credit` pairs touched this currency (D-029's per-currency
//! netting). `convert` (Slice 3) is the one operation that doesn't preserve this: it
//! moves a `credit`-sourced value's *raw* posting into a different currency's ledger
//! entirely, so a currency a `convert` pulls value out of (or into) can legitimately
//! show unequal totals here — not a bug, the same already-documented consequence of
//! D-029 ("currency-labeled subtotals just stop being individually zero once money
//! legitimately crosses a currency boundary"). See `eval::tests::
//! convert_free_examples_satisfy_the_accounting_equation` for the property this
//! module doesn't (and, per D-029, can't generally) assert for itself.

use crate::amount::Amount;
use crate::eval::LedgerState;
use crate::parse::ast::NormalBalance;
use crate::resolve::AccountKind;
use crate::typeck::TModule;

const KIND_ORDER: [AccountKind; 5] = [
    AccountKind::Asset,
    AccountKind::Liability,
    AccountKind::Equity,
    AccountKind::Income,
    AccountKind::Expense,
];

pub fn render_trial_balance(module: &TModule, ledger: &LedgerState) -> String {
    let mut out = String::new();
    for (idx, currency) in module.currencies.iter().enumerate() {
        let mut debit_lines = Vec::new();
        let mut credit_lines = Vec::new();
        let mut debit_total = Amount::ZERO;
        let mut credit_total = Amount::ZERO;

        for &kind in &KIND_ORDER {
            for account in &module.accounts {
                if account.currency.0 as usize != idx || account.kind != kind {
                    continue;
                }
                let Some(&raw) = ledger.get(&account.path) else { continue };
                let displayed = match account.normal {
                    NormalBalance::Debit => raw,
                    NormalBalance::Credit => -raw,
                };
                match account.normal {
                    NormalBalance::Debit => {
                        debit_lines.push((&account.path, displayed));
                        debit_total = debit_total + displayed;
                    }
                    NormalBalance::Credit => {
                        credit_lines.push((&account.path, displayed));
                        credit_total = credit_total + displayed;
                    }
                }
            }
        }
        if debit_lines.is_empty() && credit_lines.is_empty() {
            continue;
        }

        out.push_str(&format!("Trial Balance ({})\n", currency.name));
        for (path, balance) in debit_lines.into_iter().chain(credit_lines) {
            out.push_str(&format!("  {path:<24} {:>12}\n", balance.format(currency.scale)));
        }
        out.push_str(&format!("  {:<24} {:>12}\n", "Debit total", debit_total.format(currency.scale)));
        out.push_str(&format!("  {:<24} {:>12}\n", "Credit total", credit_total.format(currency.scale)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex;
    use crate::parse::parse;
    use crate::resolve::resolve;
    use crate::typeck::typeck;

    fn render_src(src: &str) -> String {
        let (tokens, _) = lex(src);
        let (module, _) = parse(&tokens.unwrap());
        let (resolved, _) = resolve(module.unwrap());
        let (typed, diags) = typeck(resolved.unwrap());
        assert!(diags.is_empty(), "typeck failed: {diags:?}");
        let typed = typed.unwrap();
        let (ledger, diags) = crate::eval::eval(&typed);
        assert!(diags.is_empty(), "eval failed: {diags:?}");
        render_trial_balance(&typed, &ledger.unwrap())
    }

    #[test]
    fn balanced_ledger_totals_to_zero() {
        let report = render_src(
            r#"currency ETB { scale = 2 }
               account assets:cash { currency = ETB, kind = asset, normal = debit }
               account expenses:coffee { currency = ETB, kind = expense, normal = debit }
               txn "coffee" { debit(expenses:coffee, credit(assets:cash, 45.00)); }"#,
        );
        assert!(report.contains("Trial Balance (ETB)"));
        assert!(report.contains("assets:cash"));
        assert!(report.contains("expenses:coffee"));
        assert_line_words(&report, "Debit total", &["Debit", "total", "0.00"]);
        assert_line_words(&report, "Credit total", &["Credit", "total", "0.00"]);
    }

    #[test]
    fn multi_currency_ledger_prints_one_section_per_currency() {
        let report = render_src(
            r#"currency ETB { scale = 2 }
               currency USD { scale = 2 }
               account assets:etb_cash { currency = ETB, kind = asset, normal = debit }
               account expenses:coffee { currency = ETB, kind = expense, normal = debit }
               account assets:usd_cash { currency = USD, kind = asset, normal = debit }
               account expenses:software { currency = USD, kind = expense, normal = debit }
               txn "coffee" { debit(expenses:coffee, credit(assets:etb_cash, 45.00)); }
               txn "saas" { debit(expenses:software, credit(assets:usd_cash, 10.00)); }"#,
        );
        assert!(report.contains("Trial Balance (ETB)"));
        assert!(report.contains("Trial Balance (USD)"));
        let etb_section = report.split("Trial Balance (USD)").next().unwrap();
        assert!(etb_section.contains("assets:etb_cash"));
        assert!(!etb_section.contains("usd_cash"));
    }

    #[test]
    fn scale_zero_currency_prints_without_a_decimal_point() {
        let report = render_src(
            r#"currency JPY { scale = 0 }
               account assets:cash { currency = JPY, kind = asset, normal = debit }
               account expenses:coffee { currency = JPY, kind = expense, normal = debit }
               txn "coffee" { debit(expenses:coffee, credit(assets:cash, 500)); }"#,
        );
        assert!(report.contains("500"));
        assert!(!report.contains("500.0"));
    }

    #[test]
    fn credit_normal_accounts_display_flipped_and_grouped_after_debit_normal_ones() {
        // Owner contributes capital: crediting equity (T-Credit's "money exits this
        // account", the only way to introduce Money, D-005) produces the cash that
        // gets debited into assets. Raw balances: equity:owner_capital = -1000.00,
        // assets:cash = +1000.00. equity is credit-normal, so its *displayed* balance
        // flips to +1000.00 -- both sides now read as the conventional "increased by
        // 1000" a real trial balance would show, and both land in the "Debit total" /
        // "Credit total" subtotals respectively (the accounting equation, D-036-
        // adjacent design in this module's doc comment).
        let report = render_src(
            r#"currency ETB { scale = 2 }
               account assets:cash { currency = ETB, kind = asset, normal = debit }
               account equity:owner_capital { currency = ETB, kind = equity, normal = credit }
               txn "capital" {
                   let m = credit(equity:owner_capital, 1000.00);
                   debit(assets:cash, m);
               }"#,
        );
        let cash_line =
            report.lines().find(|l| l.contains("assets:cash")).expect("cash line present");
        assert!(cash_line.trim_end().ends_with("1000.00"), "{cash_line}");
        let equity_line = report
            .lines()
            .find(|l| l.contains("equity:owner_capital"))
            .expect("equity line present");
        assert!(equity_line.trim_end().ends_with("1000.00"), "{equity_line}");
        // Asset (debit-normal) is listed before equity (credit-normal).
        assert!(report.find("assets:cash").unwrap() < report.find("equity:owner_capital").unwrap());
        assert_line_words(&report, "Debit total", &["Debit", "total", "1000.00"]);
        assert_line_words(&report, "Credit total", &["Credit", "total", "1000.00"]);
    }

    /// Finds the (unique) line starting with `label` and asserts its whitespace-split
    /// words equal `expected` — avoids hardcoding this format's column widths in tests.
    fn assert_line_words(report: &str, label: &str, expected: &[&str]) {
        let line = report
            .lines()
            .find(|l| l.trim_start().starts_with(label))
            .unwrap_or_else(|| panic!("no line starting with {label:?} in:\n{report}"));
        let words: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(words, expected, "line: {line:?}");
    }
}
