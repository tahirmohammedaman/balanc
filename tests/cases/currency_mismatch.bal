currency ETB { scale = 2 }
currency USD { scale = 2 }

account assets:usd_cash { currency = USD }
account expenses:coffee { currency = ETB }

txn "coffee" {
  let m = credit(assets:usd_cash, 45.00);
  debit(expenses:coffee, m);
}
