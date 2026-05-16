currency ETB { scale = 2 }

account assets:cash { currency = ETB, kind = asset, normal = debit }

txn "coffee" {
  let m = credit(assets:cash, 45.00);
}
