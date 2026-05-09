currency JPY { scale = 0 }

account assets:cash { currency = JPY }
account expenses:coffee { currency = JPY }

txn "coffee" {
  debit(expenses:coffee, credit(assets:cash, 999999999999999999999999999999));
}
