account assets:cash { currency = USD }

txn "coffee" {
  debit(expenses:coffee, credit(assets:cash, 45.00));
}
