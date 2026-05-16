currency ETB { scale = 2 }

account assets:cash { currency = ETB, kind = asset, normal = debit }
account expenses:rent { currency = ETB, kind = expense, normal = debit }
account expenses:utilities { currency = ETB, kind = expense, normal = debit }

txn "split rent three ways" {
  let m = credit(assets:cash, 300.00);
  let (a, remainder) = split(m, 100.00);
  let (b, c) = split_ratio(remainder, 1, 1);
  debit(expenses:rent, merge(a, b));
  debit(expenses:utilities, c);
}
