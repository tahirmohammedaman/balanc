currency ETB { scale = 2 }

account assets:cash { currency = ETB, kind = asset, normal = debit }
account expenses:coffee { currency = ETB, kind = expense, normal = debit }
account expenses:pastry { currency = ETB, kind = expense, normal = debit }

txn "coffee and a pastry, paid once" {
  let m = credit(assets:cash, 45.00);
  let (coffee, pastry) = split(m, 30.00);
  debit(expenses:coffee, coffee);
  debit(expenses:pastry, pastry);
}
