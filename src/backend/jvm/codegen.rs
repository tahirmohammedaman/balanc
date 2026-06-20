//! `typeck::TModule` -> `.class` bytes (Slice 7). Emits one class per module: a
//! `private final balanc.runtime.Ledger ledger` field (indexed by `AccountId`, so no
//! name lookup is needed at runtime — see `resolve::AccountInfo`), one public
//! no-arg-callable method per transaction (D-043), and a `run()` that calls them all
//! in declaration order and returns the trial balance as a `String`.
//!
//! Every generated method is straight-line bytecode — no branch, no jump, no
//! `StackMapTable` (see `super::code`'s doc comment for why that's sound: every place
//! this language's own semantics has a genuine data-dependent choice — `split_ratio`'s
//! largest-remainder tie-break (T-SplitRatio), `split`'s runtime bound check
//! (T-Split) — is a single `invokestatic` into `balanc.runtime.Ledger`, which carries
//! the branch instead, as ordinary `javac`-compiled Java. Money is fully erased to a
//! `long` of minor units by this point (linearity was already discharged by `typeck`,
//! D-001/D-020) — a `let`/`convert`/`split`/`split_ratio` binding becomes one `long`
//! local variable slot, keyed by its `SymbolId`.

use std::collections::{HashMap, HashSet};

use crate::amount::Amount;
use crate::parse::ast::NormalBalance;
use crate::resolve::{AccountInfo, AccountKind, CurrencyInfo, RateInfo, SymbolId};
use crate::typeck::{TModule, TMoneyExpr, TStmt, TTxnDecl};

use super::class::{ClassFile, ACC_FINAL, ACC_PRIVATE, ACC_PUBLIC, ACC_STATIC};
use super::code::{CodeBuilder, T_BOOLEAN, T_INT};
use super::pool::ConstantPool;

const LEDGER_CLASS: &str = "balanc/runtime/Ledger";
const LEDGER_DESC: &str = "Lbalanc/runtime/Ledger;";
const STRING_ARR: &str = "[Ljava/lang/String;";

/// Compiles `module` into a single `.class` file's bytes, naming the class
/// `class_name` (a valid Java identifier with no package — the caller picks it, e.g.
/// from the source file's stem).
pub fn compile(module: &TModule, class_name: &str) -> Vec<u8> {
    let mut class = ClassFile::new(class_name, "java/lang/Object");

    class.add_field(ACC_PRIVATE | ACC_FINAL, "ledger", LEDGER_DESC);
    class.add_field(ACC_PRIVATE | ACC_STATIC | ACC_FINAL, "PATHS", STRING_ARR);
    class.add_field(ACC_PRIVATE | ACC_STATIC | ACC_FINAL, "CURRENCY_INDEX", "[I");
    class.add_field(ACC_PRIVATE | ACC_STATIC | ACC_FINAL, "KIND_ORDER", "[I");
    class.add_field(ACC_PRIVATE | ACC_STATIC | ACC_FINAL, "IS_DEBIT_NORMAL", "[Z");
    class.add_field(ACC_PRIVATE | ACC_STATIC | ACC_FINAL, "CURRENCY_NAMES", STRING_ARR);
    class.add_field(ACC_PRIVATE | ACC_STATIC | ACC_FINAL, "CURRENCY_SCALES", "[I");

    let clinit = build_clinit(&mut class.pool, class_name, module);
    class.add_method(ACC_STATIC, "<clinit>", "()V", clinit);

    let init = build_init(&mut class.pool, class_name, module);
    class.add_method(ACC_PUBLIC, "<init>", "()V", init);

    let mut used_names = HashSet::new();
    let mut method_names = Vec::new();
    for (i, txn) in module.txns.iter().enumerate() {
        let name = sanitize_method_name(&txn.name, i, &mut used_names);
        let body =
            build_txn_method(&mut class.pool, class_name, txn, &module.accounts, &module.currencies, &module.rates);
        class.add_method(ACC_PUBLIC, &name, "()V", body);
        method_names.push(name);
    }

    let run = build_run(&mut class.pool, class_name, &method_names);
    class.add_method(ACC_PUBLIC, "run", "()Ljava/lang/String;", run);

    let main = build_main(&mut class.pool, class_name);
    class.add_method(ACC_PUBLIC | ACC_STATIC, "main", "([Ljava/lang/String;)V", main);

    class.to_bytes()
}

/// `render::KIND_ORDER`'s index of each kind — must match `Ledger.report`'s
/// `for (int kind = 0; kind < 5; kind++)` loop exactly.
fn kind_ordinal(kind: AccountKind) -> i32 {
    match kind {
        AccountKind::Asset => 0,
        AccountKind::Liability => 1,
        AccountKind::Equity => 2,
        AccountKind::Income => 3,
        AccountKind::Expense => 4,
    }
}

/// Builds the six static metadata arrays `Ledger::report` needs — all fully known at
/// compile time from `TModule::accounts`/`TModule::currencies` (only the numeric
/// balances and which accounts were ever posted to are genuinely runtime state).
fn build_clinit(pool: &mut ConstantPool, class_name: &str, module: &TModule) -> CodeBuilder {
    let mut c = CodeBuilder::new();
    let accounts = &module.accounts;

    build_array(&mut c, pool, class_name, "PATHS", STRING_ARR, accounts.len(), |c, pool, i| {
        c.push_string(&accounts[i].path, pool);
        StoreKind::Object
    });
    build_array(&mut c, pool, class_name, "CURRENCY_INDEX", "[I", accounts.len(), |c, pool, i| {
        c.push_int(accounts[i].currency.0 as i32, pool);
        StoreKind::Int
    });
    build_array(&mut c, pool, class_name, "KIND_ORDER", "[I", accounts.len(), |c, pool, i| {
        c.push_int(kind_ordinal(accounts[i].kind), pool);
        StoreKind::Int
    });
    build_array(&mut c, pool, class_name, "IS_DEBIT_NORMAL", "[Z", accounts.len(), |c, pool, i| {
        c.push_int(if accounts[i].normal == NormalBalance::Debit { 1 } else { 0 }, pool);
        StoreKind::Bool
    });

    let currencies = &module.currencies;
    build_array(&mut c, pool, class_name, "CURRENCY_NAMES", STRING_ARR, currencies.len(), |c, pool, i| {
        c.push_string(&currencies[i].name, pool);
        StoreKind::Object
    });
    build_array(&mut c, pool, class_name, "CURRENCY_SCALES", "[I", currencies.len(), |c, pool, i| {
        c.push_int(currencies[i].scale as i32, pool);
        StoreKind::Int
    });

    c.return_void();
    c
}

enum StoreKind {
    Object,
    Int,
    Bool,
}

/// `push length; (a)newarray; { dup; push index; <push_elem>; xastore }*; putstatic`.
fn build_array(
    c: &mut CodeBuilder,
    pool: &mut ConstantPool,
    class_name: &str,
    field_name: &str,
    field_desc: &str,
    len: usize,
    mut push_elem: impl FnMut(&mut CodeBuilder, &mut ConstantPool, usize) -> StoreKind,
) {
    c.push_int(len as i32, pool);
    match field_desc {
        STRING_ARR => c.anewarray("java/lang/String", pool),
        "[I" => c.newarray_primitive(T_INT),
        "[Z" => c.newarray_primitive(T_BOOLEAN),
        other => unreachable!("unexpected array descriptor {other}"),
    }
    for i in 0..len {
        c.dup();
        c.push_int(i as i32, pool);
        let kind = push_elem(c, pool, i);
        match kind {
            StoreKind::Object => c.aastore(),
            StoreKind::Int => c.iastore(),
            StoreKind::Bool => c.bastore(),
        }
    }
    c.putstatic(class_name, field_name, field_desc, pool);
}

fn build_init(pool: &mut ConstantPool, class_name: &str, module: &TModule) -> CodeBuilder {
    let mut c = CodeBuilder::new();
    c.aload_this();
    c.invokespecial("java/lang/Object", "<init>", "()V", pool);
    c.aload_this();
    c.new_(LEDGER_CLASS, pool);
    c.dup();
    c.push_int(module.accounts.len() as i32, pool);
    c.invokespecial(LEDGER_CLASS, "<init>", "(I)V", pool);
    c.putfield(class_name, "ledger", LEDGER_DESC, pool);
    c.return_void();
    c
}

fn build_run(pool: &mut ConstantPool, class_name: &str, method_names: &[String]) -> CodeBuilder {
    let mut c = CodeBuilder::new();
    for name in method_names {
        c.aload_this();
        c.invokevirtual(class_name, name, "()V", pool);
    }
    c.aload_this();
    c.getfield(class_name, "ledger", LEDGER_DESC, pool);
    c.getstatic(class_name, "PATHS", STRING_ARR, pool);
    c.getstatic(class_name, "CURRENCY_INDEX", "[I", pool);
    c.getstatic(class_name, "KIND_ORDER", "[I", pool);
    c.getstatic(class_name, "IS_DEBIT_NORMAL", "[Z", pool);
    c.getstatic(class_name, "CURRENCY_NAMES", STRING_ARR, pool);
    c.getstatic(class_name, "CURRENCY_SCALES", "[I", pool);
    c.invokevirtual(
        LEDGER_CLASS,
        "report",
        "([Ljava/lang/String;[I[I[Z[Ljava/lang/String;[I)Ljava/lang/String;",
        pool,
    );
    c.areturn();
    c
}

/// `public static void main(String[] args)` — prints `run()`'s report to stdout, so
/// the packaged class is directly runnable (`java -cp ... ClassName`) as well as
/// callable as a library from another Java program (D-043).
fn build_main(pool: &mut ConstantPool, class_name: &str) -> CodeBuilder {
    let mut c = CodeBuilder::new();
    c.new_(class_name, pool);
    c.dup();
    c.invokespecial(class_name, "<init>", "()V", pool);
    let instance_slot = 1; // slot 0 is `args`
    c.astore(instance_slot);
    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;", pool);
    c.aload(instance_slot);
    c.invokevirtual(class_name, "run", "()Ljava/lang/String;", pool);
    c.invokevirtual("java/io/PrintStream", "print", "(Ljava/lang/String;)V", pool);
    c.return_void();
    c
}

/// Turns a `txn "name"` into a valid, class-unique Java method identifier. Doesn't
/// avoid Java reserved words (no txn name in the corpus collides with one); a
/// from-scratch bytecode writer already can't run `javac`-level identifier
/// validation, so this stays a best-effort transliteration, not a full check.
fn sanitize_method_name(txn_name: &str, index: usize, used: &mut HashSet<String>) -> String {
    let mut ident: String =
        txn_name.chars().map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' }).collect();
    if ident.is_empty() || ident.chars().next().unwrap().is_ascii_digit() {
        ident = format!("txn{index}_{ident}");
    }
    ident.push_str("Txn");
    let mut candidate = ident.clone();
    let mut suffix = 1;
    while used.contains(&candidate) {
        candidate = format!("{ident}{suffix}");
        suffix += 1;
    }
    used.insert(candidate.clone());
    candidate
}

/// Per-transaction codegen state: `SymbolId` -> local variable slot (every bound
/// value is a `long`, so slots always advance by 2 — JVMS §2.6.1), plus which
/// `extra_scale` (D-026) each live `Residue` binding carries, needed again once its
/// `absorb` statement is reached.
struct TxnCodegen<'a> {
    pool: &'a mut ConstantPool,
    class_name: &'a str,
    code: CodeBuilder,
    slots: HashMap<SymbolId, u16>,
    next_slot: u16,
    residue_extra_scale: HashMap<SymbolId, u32>,
}

impl<'a> TxnCodegen<'a> {
    fn slot_for(&mut self, symbol: SymbolId) -> u16 {
        *self.slots.entry(symbol).or_insert_with(|| {
            let slot = self.next_slot;
            self.next_slot += 2;
            slot
        })
    }

    fn fresh_long_slot(&mut self) -> u16 {
        let slot = self.next_slot;
        self.next_slot += 2;
        slot
    }

    fn fresh_ref_slot(&mut self) -> u16 {
        let slot = self.next_slot;
        self.next_slot += 1;
        slot
    }

    /// Emits `expr`'s bytecode, leaving its `long` value on top of the operand stack
    /// — mirrors `eval::eval_money_expr` exactly, including `Credit`'s immediate post
    /// to its source account.
    fn emit_money_expr(&mut self, expr: &TMoneyExpr, accounts: &[AccountInfo]) {
        match expr {
            TMoneyExpr::Credit { account, amount } => {
                self.emit_post(accounts, account.0, -amount.cents());
                self.code.push_long(amount.cents(), self.pool);
            }
            TMoneyExpr::Var { symbol } => {
                let slot = self.slot_for(*symbol);
                self.code.lload(slot);
            }
            TMoneyExpr::Merge { a, b } => {
                self.emit_money_expr(a, accounts);
                self.emit_money_expr(b, accounts);
                self.code.ladd();
            }
        }
    }

    /// `this.ledger.post(accountIndex, delta)` — `delta` is already on the caller's
    /// mind as a compile-time-known `i64`; account index is `AccountId::0` directly
    /// (`resolve::AccountInfo` is stored in declaration order, so no lookup needed).
    fn emit_post(&mut self, _accounts: &[AccountInfo], account_index: u32, delta: i64) {
        self.code.aload_this();
        self.code.getfield(self.class_name, "ledger", LEDGER_DESC, self.pool);
        self.code.push_int(account_index as i32, self.pool);
        self.code.push_long(delta, self.pool);
        self.code.invokevirtual(LEDGER_CLASS, "post", "(IJ)V", self.pool);
    }

    fn emit_stmt(&mut self, stmt: &TStmt, accounts: &[AccountInfo], currencies: &[CurrencyInfo], rates: &[RateInfo]) {
        match stmt {
            TStmt::Let { symbol, value, .. } => {
                self.emit_money_expr(value, accounts);
                let slot = self.slot_for(*symbol);
                self.code.lstore(slot);
            }
            TStmt::Convert { primary, residual, money, rate, .. } => {
                let amt_slot = self.fresh_long_slot();
                self.emit_money_expr(money, accounts);
                self.code.lstore(amt_slot);

                let rate_info = &rates[rate.0 as usize];
                let from_scale = currencies[rate_info.from.0 as usize].scale;
                let to_scale = currencies[rate_info.to.0 as usize].scale;
                let extra_scale = from_scale + rate_info.scale;
                // Mirrors `eval::convert`: numerator = amt * rate.numerator * 10^to_scale,
                // denominator = 10^extra_scale — folding the two compile-time-known
                // factors into one `multiplier` constant since neither depends on `amt`.
                let multiplier = rate_info
                    .numerator
                    .checked_mul(10i64.checked_pow(to_scale).expect("rate scale too large for i64"))
                    .expect("rate multiplier overflows i64");
                let denom = 10i64.checked_pow(extra_scale).expect("rate extra_scale too large for i64");

                self.code.lload(amt_slot);
                self.code.push_long(multiplier, self.pool);
                self.code.lmul();
                self.code.push_long(denom, self.pool);
                self.code.ldiv();
                let primary_slot = self.slot_for(*primary);
                self.code.lstore(primary_slot);

                self.code.lload(amt_slot);
                self.code.push_long(multiplier, self.pool);
                self.code.lmul();
                self.code.push_long(denom, self.pool);
                self.code.lrem();
                let residual_slot = self.slot_for(*residual);
                self.code.lstore(residual_slot);

                self.residue_extra_scale.insert(*residual, extra_scale);
            }
            TStmt::Debit { account, value, .. } => {
                self.code.aload_this();
                self.code.getfield(self.class_name, "ledger", LEDGER_DESC, self.pool);
                self.code.push_int(account.0 as i32, self.pool);
                self.emit_money_expr(value, accounts);
                self.code.invokevirtual(LEDGER_CLASS, "post", "(IJ)V", self.pool);
            }
            TStmt::Absorb { residual, account, .. } => {
                let extra_scale = *self
                    .residue_extra_scale
                    .get(residual)
                    .expect("internal error: typeck guaranteed this residue is live");
                let slot = self.slot_for(*residual);
                self.code.aload_this();
                self.code.getfield(self.class_name, "ledger", LEDGER_DESC, self.pool);
                self.code.push_int(account.0 as i32, self.pool);
                self.code.lload(slot);
                if extra_scale != 0 {
                    let divisor = 10i64.checked_pow(extra_scale).expect("extra_scale too large for i64");
                    self.code.push_long(divisor, self.pool);
                    self.code.ldiv();
                }
                self.code.invokevirtual(LEDGER_CLASS, "post", "(IJ)V", self.pool);
            }
            TStmt::Split { a, b, money, amount, .. } => {
                self.emit_split(*a, *b, money, *amount, accounts);
            }
            TStmt::SplitRatio { a, b, money, a_weight, b_weight, .. } => {
                self.emit_split_ratio(*a, *b, money, *a_weight, *b_weight, accounts);
            }
        }
    }

    /// T-Split (D-034): `b = Ledger.checkedSplitRemainder(total, amount)`, `a =
    /// amount` — `amount` is already a compile-time constant (typeck lowered the
    /// literal), so it's simply emitted twice rather than duplicated on the stack.
    fn emit_split(&mut self, a: SymbolId, b: SymbolId, money: &TMoneyExpr, amount: Amount, accounts: &[AccountInfo]) {
        self.emit_money_expr(money, accounts);
        self.code.push_long(amount.cents(), self.pool);
        self.code.invokestatic(LEDGER_CLASS, "checkedSplitRemainder", "(JJ)J", self.pool);
        let b_slot = self.slot_for(b);
        self.code.lstore(b_slot);

        self.code.push_long(amount.cents(), self.pool);
        let a_slot = self.slot_for(a);
        self.code.lstore(a_slot);
    }

    /// T-SplitRatio (D-015/D-035): `{a, b} = Ledger.splitRatio(total, aWeight,
    /// bWeight)` — the tie-break's data-dependent branch lives entirely in that
    /// runtime call (see this module's doc comment); this just unpacks the returned
    /// `long[2]`.
    fn emit_split_ratio(
        &mut self,
        a: SymbolId,
        b: SymbolId,
        money: &TMoneyExpr,
        a_weight: u32,
        b_weight: u32,
        accounts: &[AccountInfo],
    ) {
        self.emit_money_expr(money, accounts);
        self.code.push_long(a_weight as i64, self.pool);
        self.code.push_long(b_weight as i64, self.pool);
        self.code.invokestatic(LEDGER_CLASS, "splitRatio", "(JJJ)[J", self.pool);
        let arr_slot = self.fresh_ref_slot();
        self.code.astore(arr_slot);

        self.code.aload(arr_slot);
        self.code.push_int(0, self.pool);
        self.code.laload();
        let a_slot = self.slot_for(a);
        self.code.lstore(a_slot);

        self.code.aload(arr_slot);
        self.code.push_int(1, self.pool);
        self.code.laload();
        let b_slot = self.slot_for(b);
        self.code.lstore(b_slot);
    }
}

fn build_txn_method(
    pool: &mut ConstantPool,
    class_name: &str,
    txn: &TTxnDecl,
    accounts: &[AccountInfo],
    currencies: &[CurrencyInfo],
    rates: &[RateInfo],
) -> CodeBuilder {
    let mut ctx = TxnCodegen {
        pool,
        class_name,
        code: CodeBuilder::new(),
        slots: HashMap::new(),
        next_slot: 1,
        residue_extra_scale: HashMap::new(),
    };
    for stmt in &txn.stmts {
        ctx.emit_stmt(stmt, accounts, currencies, rates);
    }
    ctx.code.return_void();
    ctx.code
}
