currency USD { scale = 2 }
currency ETB { scale = 2 }

account assets:usd_cash { currency = USD }
account assets:etb_cash { currency = ETB }
account income:fx_rounding { currency = ETB }

rate usd_etb from USD to ETB = 57.20 round down;

txn "fx" {
  let m = credit(assets:usd_cash, 100.01);
  let (m2, r) = convert(m, usd_etb);
  debit(assets:etb_cash, m2);
  absorb(r, income:fx_rounding);
}
