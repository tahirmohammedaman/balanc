currency ETB { scale = 2 }

account assets:cash { currency = ETB }
account expenses:rent { currency = ETB }
account expenses:utilities { currency = ETB }

txn "rent" {
  let m = credit(assets:cash, 100.00);
  let (a, b) = split(m, 250.00);
  debit(expenses:rent, a);
  debit(expenses:utilities, b);
}
