currency ETB { scale = 2 }
currency USD { scale = 2 }

account assets:usd_cash { currency = USD, kind = asset, normal = debit }
account expenses:coffee { currency = ETB, kind = expense, normal = debit }

txn "coffee" {
  let m = credit(assets:usd_cash, 45.00);
  debit(expenses:coffee, m);
}
