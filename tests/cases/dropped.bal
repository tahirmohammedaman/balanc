currency ETB { scale = 2 }

account assets:cash { currency = ETB }

txn "coffee" {
  let m = credit(assets:cash, 45.00);
}
