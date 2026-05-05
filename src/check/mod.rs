//! Balance checking only, for Slice 0. This is a placeholder for `typeck`: once Slice 1
//! introduces the linear context Γ, this module's one rule (T-Txn's balance premise)
//! moves there and this module goes away.

use crate::amount::Amount;
use crate::diag::{Code, Diagnostic};
use crate::parse::ast::{LegKind, Module};

/// (T-Txn, balance premise) — a transaction checks only if, summed over its legs,
/// total debits equal total credits.
pub fn check_balance(module: &Module) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for txn in &module.txns {
        let mut debits = Amount::ZERO;
        let mut credits = Amount::ZERO;
        for leg in &txn.legs {
            match leg.kind {
                LegKind::Debit => debits = debits + leg.amount,
                LegKind::Credit => credits = credits + leg.amount,
            }
        }
        if debits != credits {
            diags.push(Diagnostic::new(
                Code::Unbalanced,
                format!(
                    "transaction \"{}\" is unbalanced: debits {debits} != credits {credits}",
                    txn.name
                ),
                txn.span,
            ));
        }
    }
    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex;
    use crate::parse::parse;

    fn check_src(src: &str) -> Vec<Diagnostic> {
        let (tokens, _) = lex(src);
        let (module, _) = parse(&tokens.unwrap());
        check_balance(&module.unwrap())
    }

    #[test]
    fn balanced_transaction_has_no_diagnostics() {
        let diags = check_src(
            r#"txn "coffee" { debit(expenses:coffee, 45.00); credit(assets:cash, 45.00); }"#,
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn unbalanced_transaction_is_reported_at_txn_span() {
        let diags = check_src(
            r#"txn "coffee" { debit(expenses:coffee, 45.00); credit(assets:cash, 40.00); }"#,
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Code::Unbalanced);
    }
}
