account assets:cash { currency = USD, kind = asset, normal = debit }

txn "coffee" {
  debit(expenses:coffee, credit(assets:cash, 45.00));
}
