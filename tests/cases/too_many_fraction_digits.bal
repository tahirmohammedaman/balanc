currency JPY { scale = 0 }

account assets:cash { currency = JPY, kind = asset, normal = debit }
account expenses:coffee { currency = JPY, kind = expense, normal = debit }

txn "coffee" {
  debit(expenses:coffee, credit(assets:cash, 45.50));
}
