currency ETB { scale = 2 }

account assets:cash { currency = ETB }
account expenses:coffee { currency = ETB }

txn "coffee" {
  let m = credit(assets:cash, 45.00);
  debit(expenses:coffee, merge(m, m));
}
