currency ETB { scale = 2 }

account assets:cash { currency = ETB, kind = asset, normal = debit }
account expenses:a { currency = ETB, kind = expense, normal = debit }
account expenses:b { currency = ETB, kind = expense, normal = debit }

txn "coffee" {
  let m = credit(assets:cash, 45.00);
  debit(expenses:a, m);
  debit(expenses:b, m);
}
