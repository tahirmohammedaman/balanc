currency ETB { scale = 2 }

account assets:cash { currency = ETB }
account expenses:rent { currency = ETB }
account expenses:utilities { currency = ETB }

txn "split rent three ways" {
  let m = credit(assets:cash, 300.00);
  let (a, remainder) = split(m, 100.00);
  let (b, c) = split_ratio(remainder, 1, 1);
  debit(expenses:rent, merge(a, b));
  debit(expenses:utilities, c);
}
