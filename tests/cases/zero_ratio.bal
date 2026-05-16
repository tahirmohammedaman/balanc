currency ETB { scale = 2 }

account assets:cash { currency = ETB, kind = asset, normal = debit }
account expenses:rent { currency = ETB, kind = expense, normal = debit }
account expenses:utilities { currency = ETB, kind = expense, normal = debit }

txn "rent" {
  let m = credit(assets:cash, 100.00);
  let (a, b) = split_ratio(m, 0, 0);
  debit(expenses:rent, a);
  debit(expenses:utilities, b);
}
