txn "coffee" {
  let m = credit(assets:cash, 45.00);
  debit(expenses:coffee, m);
}
