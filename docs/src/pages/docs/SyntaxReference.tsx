import { DocPage } from "../../components/DocPage";
import { C, DocTable } from "../../components/Prose";
import { CodeBlock } from "../../components/code/CodeBlock";

const GRAMMAR = `Module        := Decl* Eof
Decl          := CurrencyDecl | AccountDecl | RateDecl | TxnDecl
CurrencyDecl  := "currency" Ident "{" "scale" "=" WholeNumber "}"
AccountDecl   := "account" AccountPath "{" "currency" "=" Ident ","
                   "kind" "=" Ident "," "normal" "=" ("debit" | "credit") "}"
RateDecl      := "rate" Ident "from" Ident "to" Ident "=" Decimal "round" "down" ";"
TxnDecl       := "txn" String "{" Stmt* "}"
Stmt          := LetStmt | PairStmt | DebitStmt | AbsorbStmt
LetStmt       := "let" Ident "=" MoneyExpr ";"
PairStmt      := "let" "(" Ident "," Ident ")" "=" (ConvertCall | SplitCall | SplitRatioCall) ";"
ConvertCall   := "convert" "(" MoneyExpr "," Ident ")"
SplitCall     := "split" "(" MoneyExpr "," Decimal ")"
SplitRatioCall:= "split_ratio" "(" MoneyExpr "," WholeNumber "," WholeNumber ")"
DebitStmt     := "debit" "(" AccountPath "," MoneyExpr ")" ";"
AbsorbStmt    := "absorb" "(" Ident "," AccountPath ")" ";"
MoneyExpr     := CreditExpr | MergeExpr | Var
CreditExpr    := "credit" "(" AccountPath "," Decimal ")"
MergeExpr     := "merge" "(" MoneyExpr "," MoneyExpr ")"
Var           := Ident
AccountPath   := Ident (":" Ident)*`;

export function SyntaxReference() {
  return (
    <DocPage
      title="Syntax Reference"
      lede="The complete grammar, hand-written as a recursive-descent parser (no parser generator). Declarations may appear in any order at module scope — nothing requires a currency to be declared before the accounts that use it."
    >
      <CodeBlock lang="ebnf" code={GRAMMAR} />

      <p>
        Two disambiguation rules worth knowing: a <C>let</C> immediately followed by <C>(</C> is always a{" "}
        <C>PairStmt</C> (an ordinary <C>let</C>'s left-hand side is always a single identifier), and the keyword
        right after <C>=</C> decides which of <C>convert</C>/<C>split</C>/<C>split_ratio</C> it is. Neither case
        needs backtracking. <C>merge</C> is the only place a <C>MoneyExpr</C> nests inside another, so a{" "}
        <C>merge</C> chain is capped at 256 levels deep (<C>E_EXPR_TOO_DEEP</C>) to stay well inside a safe
        recursion budget.
      </p>

      <h2>Lexical elements</h2>
      <DocTable
        columns={[{ header: "Element" }, { header: "Notes" }]}
        rows={[
          [
            <span className="code-col">Keywords</span>,
            <C>
              txn let debit credit currency account scale kind normal rate from to round down convert absorb
              split split_ratio merge
            </C>,
          ],
          [
            <span className="code-col">Decimal</span>,
            "Digits with an optional . and at least one fractional digit if present. Kept as raw text through lexing — lowering into a scaled integer happens in typeck, once the value's currency (and hence its scale) is known.",
          ],
          [
            <span className="code-col">String</span>,
            <>
              Double-quoted; used only for a transaction's name, e.g. <C>{`txn "coffee"`}</C>.
            </>,
          ],
          [<span className="code-col">Comments</span>, <><C>{"//"}</C> to end of line. No block comments.</>],
          [
            <span className="code-col">Negative numbers</span>,
            <>
              Not representable — there is no <C>-</C> token in numeric-literal position. Every literal amount
              is non-negative by construction.
            </>,
          ],
        ]}
      />
    </DocPage>
  );
}
