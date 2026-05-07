currency ETB { scale = 2 }

account assets:cash { currency = ETB }
account expenses:a { currency = ETB }
account expenses:b { currency = ETB }

txn "coffee" {
  let m = credit(assets:cash, 45.00);
  debit(expenses:a, m);
  debit(expenses:b, m);
}
