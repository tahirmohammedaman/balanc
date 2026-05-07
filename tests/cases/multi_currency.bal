currency ETB { scale = 2 }
currency USD { scale = 2 }

account assets:etb_cash { currency = ETB }
account expenses:coffee { currency = ETB }
account assets:usd_cash { currency = USD }
account expenses:software { currency = USD }

txn "coffee" {
  debit(expenses:coffee, credit(assets:etb_cash, 45.00));
}

txn "saas" {
  debit(expenses:software, credit(assets:usd_cash, 10.00));
}
