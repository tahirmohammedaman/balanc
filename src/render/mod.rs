//! `LedgerState` -> report text. The only stage besides `main` allowed to produce
//! output for a human; `lex`/`parse`/`check`/`eval` stay pure (invariant 7).
//!
//! One section per currency with at least one posting (Slice 2): accounts are grouped
//! by their declared currency, in declaration order, and each section's amounts are
//! formatted at that currency's own scale. A currency with no postings in this run
//! gets no section — an empty section would just be noise.

use crate::amount::Amount;
use crate::eval::LedgerState;
use crate::typeck::TModule;

pub fn render_trial_balance(module: &TModule, ledger: &LedgerState) -> String {
    let mut out = String::new();
    for (idx, currency) in module.currencies.iter().enumerate() {
        let mut lines = Vec::new();
        let mut total = Amount::ZERO;
        for account in &module.accounts {
            if account.currency.0 as usize != idx {
                continue;
            }
            if let Some(&balance) = ledger.get(&account.path) {
                lines.push((&account.path, balance));
                total = total + balance;
            }
        }
        if lines.is_empty() {
            continue;
        }
        out.push_str(&format!("Trial Balance ({})\n", currency.name));
        for (path, balance) in lines {
            out.push_str(&format!("  {path:<24} {:>12}\n", balance.format(currency.scale)));
        }
        out.push_str(&format!("  {:<24} {:>12}\n", "Total", total.format(currency.scale)));
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
               account assets:cash { currency = ETB }
               account expenses:coffee { currency = ETB }
               txn "coffee" { debit(expenses:coffee, credit(assets:cash, 45.00)); }"#,
        );
        assert!(report.contains("Trial Balance (ETB)"));
        assert!(report.contains("assets:cash"));
        assert!(report.contains("expenses:coffee"));
        assert!(report.trim_end().ends_with("0.00"));
    }

    #[test]
    fn multi_currency_ledger_prints_one_section_per_currency() {
        let report = render_src(
            r#"currency ETB { scale = 2 }
               currency USD { scale = 2 }
               account assets:etb_cash { currency = ETB }
               account expenses:coffee { currency = ETB }
               account assets:usd_cash { currency = USD }
               account expenses:software { currency = USD }
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
               account assets:cash { currency = JPY }
               account expenses:coffee { currency = JPY }
               txn "coffee" { debit(expenses:coffee, credit(assets:cash, 500)); }"#,
        );
        assert!(report.contains("500"));
        assert!(!report.contains("500.0"));
    }
}
