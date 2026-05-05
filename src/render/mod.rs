//! `LedgerState` -> report text. The only stage besides `main` allowed to produce
//! output for a human; `lex`/`parse`/`check`/`eval` stay pure (invariant 7).

use crate::amount::Amount;
use crate::eval::LedgerState;

pub fn render_trial_balance(ledger: &LedgerState) -> String {
    let mut out = String::from("Trial Balance\n");
    let mut total = Amount::ZERO;
    for (account, balance) in ledger {
        out.push_str(&format!("  {account:<24} {balance:>12}\n"));
        total = total + *balance;
    }
    out.push_str(&format!("  {:<24} {total:>12}\n", "Total"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_ledger_totals_to_zero() {
        let mut ledger = LedgerState::new();
        ledger.insert("expenses:coffee".to_string(), Amount::from_cents(4500));
        ledger.insert("assets:cash".to_string(), Amount::from_cents(-4500));
        let report = render_trial_balance(&ledger);
        assert!(report.contains("assets:cash"));
        assert!(report.contains("expenses:coffee"));
        assert!(report.trim_end().ends_with("0.00"));
    }
}
