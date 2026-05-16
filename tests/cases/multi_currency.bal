currency ETB { scale = 2 }
currency USD { scale = 2 }

account assets:etb_cash { currency = ETB, kind = asset, normal = debit }
account expenses:coffee { currency = ETB, kind = expense, normal = debit }
account assets:usd_cash { currency = USD, kind = asset, normal = debit }
account expenses:software { currency = USD, kind = expense, normal = debit }

txn "coffee" {
  debit(expenses:coffee, credit(assets:etb_cash, 45.00));
}

txn "saas" {
  debit(expenses:software, credit(assets:usd_cash, 10.00));
}
