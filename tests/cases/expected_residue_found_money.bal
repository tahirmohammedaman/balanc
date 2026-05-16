currency USD { scale = 2 }
currency ETB { scale = 2 }

account assets:usd_cash { currency = USD, kind = asset, normal = debit }
account assets:etb_cash { currency = ETB, kind = asset, normal = debit }
account income:fx_rounding { currency = ETB, kind = income, normal = credit }

rate usd_etb from USD to ETB = 57.20 round down;

txn "fx" {
  let m = credit(assets:usd_cash, 100.00);
  let (m2, r) = convert(m, usd_etb);
  absorb(m2, income:fx_rounding);
  absorb(r, income:fx_rounding);
}
