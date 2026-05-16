currency ETB { scale = 2 }

account assets:cash { currency = ETB, kind = asset, normal = debit }
account expenses:rent { currency = ETB, kind = expense, normal = debit }

txn "rent" {
  let m = credit(assets:cash, 100.00);
  let (a, b) = split(m, 40.00);
  debit(expenses:rent, a);
}
