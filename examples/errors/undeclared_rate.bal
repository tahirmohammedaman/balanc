currency USD { scale = 2 }
currency ETB { scale = 2 }

account assets:usd_cash { currency = USD, kind = asset, normal = debit }
account assets:etb_cash { currency = ETB, kind = asset, normal = debit }
account income:fx_rounding { currency = ETB, kind = income, normal = credit }

txn "fx" {
  let m = credit(assets:usd_cash, 100.00);
  let (m2, r) = convert(m, usd_etb);
  debit(assets:etb_cash, m2);
  absorb(r, income:fx_rounding);
}
