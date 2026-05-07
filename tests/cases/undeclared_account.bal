currency ETB { scale = 2 }

txn "coffee" {
  debit(expenses:coffee, credit(assets:cash, 45.00));
}
