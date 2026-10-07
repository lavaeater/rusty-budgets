//! Kraken's "Ledgers" export: one row per balance change, where a trade is
//! two rows (one per asset) sharing a `refid`.

use super::book::Book;
use super::{AccountReport, Provider, csv};
use chrono::NaiveDate;
use std::collections::{BTreeMap, HashMap, HashSet};

const FIAT: &[&str] = &["EUR", "USD", "SEK", "GBP", "CHF", "CAD", "AUD", "JPY", "NOK", "DKK", "PLN"];
const FIAT_EPSILON: f64 = 0.005;
const CRYPTO_EPSILON: f64 = 1e-8;

pub fn is_ledger(header: &str) -> bool {
    header.contains("refid") && header.contains("balance") && header.contains("asset")
}

pub fn is_trades(header: &str) -> bool {
    header.contains("ordertxid") && header.contains("pair")
}

/// Kraken's legacy asset codes (`XXBT`, `ZEUR`, …) mapped to common tickers.
fn normalize_asset(raw: &str) -> String {
    let code = match raw {
        "XXBT" | "XBT" => "BTC",
        "XXDG" | "XDG" => "DOGE",
        "XETH" => "ETH",
        "XLTC" => "LTC",
        "XXRP" => "XRP",
        "XXLM" => "XLM",
        "XXMR" => "XMR",
        "XETC" => "ETC",
        "XZEC" => "ZEC",
        "ZEUR" => "EUR",
        "ZUSD" => "USD",
        "ZGBP" => "GBP",
        "ZCAD" => "CAD",
        "ZJPY" => "JPY",
        other => other,
    };
    code.to_string()
}

#[derive(Debug)]
struct Row {
    refid: String,
    time: String,
    date: NaiveDate,
    kind: String,
    asset: String,
    fiat: bool,
    amount: f64,
    fee: f64,
    balance: Option<f64>,
}

impl Row {
    fn is_trade(&self) -> bool {
        matches!(self.kind.as_str(), "trade" | "spend" | "receive")
    }
}

pub fn parse(text: &str, file_name: &str) -> Result<AccountReport, String> {
    let records = csv::parse(text, ',');
    let (header, body) = records.split_first().ok_or("Filen är tom")?;
    let column = |name: &str| {
        header
            .iter()
            .position(|h| h == name)
            .ok_or(format!("Kolumnen \"{name}\" saknas"))
    };
    let (refid, time, kind, subclass, asset, amount, fee, balance) = (
        column("refid")?,
        column("time")?,
        column("type")?,
        column("subclass").ok(),
        column("asset")?,
        column("amount")?,
        column("fee")?,
        column("balance")?,
    );

    let mut rows = Vec::with_capacity(body.len());
    for record in body {
        let cell = |i: usize| record.get(i).map_or("", String::as_str);
        let time_cell = cell(time);
        let Some(date) = time_cell
            .get(..10)
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        else {
            continue;
        };
        let asset_code = normalize_asset(cell(asset));
        rows.push(Row {
            refid: cell(refid).to_string(),
            time: time_cell.to_string(),
            date,
            kind: cell(kind).to_string(),
            fiat: subclass.is_some_and(|i| cell(i) == "fiat") || FIAT.contains(&asset_code.as_str()),
            asset: asset_code,
            amount: csv::number(cell(amount)).unwrap_or(0.0),
            fee: csv::number(cell(fee)).unwrap_or(0.0),
            balance: csv::number(cell(balance)),
        });
    }
    // Timestamps are ISO-formatted, so string order is chronological; the
    // sort is stable so rows sharing a timestamp keep the export's order.
    rows.sort_by(|a, b| a.time.cmp(&b.time));

    let currency = rows
        .iter()
        .find(|r| r.fiat && r.kind == "deposit")
        .or_else(|| rows.iter().find(|r| r.fiat))
        .map_or_else(|| "EUR".to_string(), |r| r.asset.clone());

    let mut book = Book::new(&currency).crypto_when(|asset, _| !FIAT.contains(&asset));
    add_opening_balances(&mut book, &rows, &currency);

    let mut trade_legs: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, row) in rows.iter().enumerate() {
        if row.is_trade() {
            trade_legs.entry(row.refid.as_str()).or_default().push(i);
        }
    }

    let mut handled_trades = HashSet::new();
    for row in &rows {
        if row.is_trade() {
            if handled_trades.insert(row.refid.as_str()) {
                let legs: Vec<&Row> = trade_legs[row.refid.as_str()].iter().map(|&i| &rows[i]).collect();
                apply_trade(&mut book, &legs, &currency);
            }
            continue;
        }
        let is_base = row.asset == currency;
        match (row.kind.as_str(), is_base) {
            ("deposit", true) => book.deposit(row.date, row.amount, row.fee),
            ("withdrawal", true) => book.withdraw(row.date, -row.amount, row.fee),
            (_, true) => book.other(row.date, row.amount - row.fee),
            ("deposit" | "transfer" | "staking" | "earn" | "reward" | "airdrop", false) if row.amount > 0.0 => {
                book.receive_free(row.date, &row.asset, &row.asset, row.amount - row.fee);
                book.warn(format!(
                    "{} {} togs emot ({}) utan känt anskaffningsvärde — räknas som 0.",
                    row.amount, row.asset, row.kind
                ));
            }
            (_, false) if row.amount < 0.0 => {
                book.send_out(row.date, &row.asset, &row.asset, -row.amount, row.fee, None);
            }
            (kind, false) => {
                if row.amount > 0.0 {
                    book.receive_free(row.date, &row.asset, &row.asset, row.amount - row.fee);
                }
                book.warn(format!("Okänd radtyp \"{kind}\" för {} — behandlad som gratis tillskott.", row.asset));
            }
        }
    }

    check_closing_balances(&mut book, &rows, &currency);

    Ok(book.finish(
        format!("Kraken ({currency})"),
        Provider::Kraken,
        file_name.to_string(),
    ))
}

/// The first row of each asset should start from zero; anything else is a
/// balance that predates the export.
fn add_opening_balances(book: &mut Book, rows: &[Row], currency: &str) {
    let mut seen = HashSet::new();
    for row in rows {
        if !seen.insert(row.asset.as_str()) {
            continue;
        }
        let Some(balance) = row.balance else { continue };
        let opening = balance - (row.amount - row.fee);
        if row.asset == currency && opening.abs() > FIAT_EPSILON {
            book.deposit(row.date, opening, 0.0);
            book.warn(format!(
                "Ingående saldo {opening:.2} {currency} saknar insättning i filen — räknas som insättning."
            ));
        } else if row.asset != currency && opening.abs() > CRYPTO_EPSILON {
            book.receive_free(row.date, &row.asset, &row.asset, opening);
            book.warn(format!(
                "Ingående saldo {opening} {} saknar historik i filen — anskaffningsvärdet räknas som 0.",
                row.asset
            ));
        }
    }
}

fn apply_trade(book: &mut Book, legs: &[&Row], currency: &str) {
    let (Some(out), Some(inc), 2) = (
        legs.iter().find(|l| l.amount < 0.0),
        legs.iter().find(|l| l.amount > 0.0),
        legs.len(),
    ) else {
        book.warn(format!("Handel {} har oväntade rader och hoppades över.", legs[0].refid));
        return;
    };
    let date = out.date;
    // Fees are charged on whichever leg Kraken chose, in that leg's asset.
    let out_quantity = -out.amount + out.fee;
    let in_quantity = inc.amount - inc.fee;
    if out.asset == currency {
        let fee = out.fee + inc.fee * (out_quantity / inc.amount.max(f64::MIN_POSITIVE));
        book.buy(date, &inc.asset, &inc.asset, in_quantity, out_quantity, fee);
    } else if inc.asset == currency {
        if out.fee > 0.0 {
            book.warn("Avgifter dragna i kryptovaluta vid försäljning ingår i såld mängd.");
        }
        book.sell(date, &out.asset, &out.asset, out_quantity, in_quantity, inc.fee);
    } else {
        book.swap(date, &out.asset, out_quantity, &inc.asset, in_quantity);
    }
}

/// Compares the book against the running balances in the export, which
/// catches both parser bugs and rows the export left out.
fn check_closing_balances(book: &mut Book, rows: &[Row], currency: &str) {
    let mut closing: BTreeMap<&str, f64> = BTreeMap::new();
    for row in rows {
        if let Some(balance) = row.balance {
            closing.insert(row.asset.as_str(), balance);
        }
    }
    for (asset, balance) in closing {
        let (ours, epsilon) = if asset == currency {
            (book.cash(), FIAT_EPSILON)
        } else {
            (book.quantity(asset), CRYPTO_EPSILON)
        };
        if (ours - balance).abs() > epsilon {
            book.warn(format!(
                "Beräknat saldo för {asset} ({ours}) stämmer inte med exportens slutsaldo ({balance})."
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "\"txid\",\"refid\",\"time\",\"type\",\"subtype\",\"aclass\",\"subclass\",\"asset\",\"wallet\",\"amount\",\"fee\",\"balance\"";

    fn ledger(rows: &[&str]) -> String {
        std::iter::once(HEADER).chain(rows.iter().copied()).collect::<Vec<_>>().join("\n")
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn deposits_trades_and_withdrawals() {
        let text = ledger(&[
            r#""L1","D1","2021-01-01 10:00:00","deposit","","currency","fiat","ZEUR","spot",1000.0,0,1000.0"#,
            r#""L2","T1","2021-01-02 10:00:00","trade","tradespot","currency","fiat","ZEUR","spot",-500.0,2.0,498.0"#,
            r#""L3","T1","2021-01-02 10:00:00","trade","tradespot","currency","crypto","XXBT","spot",0.01,0,0.01"#,
            r#""L4","S1","2021-02-01 10:00:00","spend","","currency","crypto","XXBT","spot",-0.005,0,0.005"#,
            r#""L5","S1","2021-02-01 10:00:00","receive","","currency","fiat","ZEUR","spot",400.0,4.0,894.0"#,
            r#""L6","W1","2021-03-01 10:00:00","withdrawal","","currency","crypto","XXBT","spot",-0.004,0.001,0.0"#,
            r#""L7","W2","2021-03-02 10:00:00","withdrawal","","currency","fiat","ZEUR","spot",-99.0,1.0,794.0"#,
        ]);
        let report = parse(&text, "ledger.csv").unwrap();
        assert_eq!(report.currency, "EUR");
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert!(close(report.summary.deposited, 1000.0));
        assert!(close(report.summary.withdrawn, 99.0));
        assert!(close(report.summary.bought, 502.0));
        assert!(close(report.summary.sold, 396.0));
        // Half the BTC (basis 251) sold for 396 net.
        assert!(close(report.summary.realised, 145.0 - 50.2));
        assert!(close(report.cash_balance, 794.0));
        assert!(report.holdings.is_empty(), "{:?}", report.holdings);
        assert!(close(report.assets_moved_out[0].quantity, 0.004));
    }

    #[test]
    fn opening_balance_is_reported() {
        let text = ledger(&[
            r#""L1","D1","2021-01-01 10:00:00","deposit","","currency","fiat","EUR","spot",100.0,0,102.5"#,
        ]);
        let report = parse(&text, "ledger.csv").unwrap();
        assert!(close(report.summary.deposited, 102.5));
        assert_eq!(report.warnings.len(), 1);
    }
    #[test]
    fn crypto_to_crypto_trade_carries_basis() {
        let text = ledger(&[
            r#""L1","D1","2017-11-01 10:00:00","deposit","","currency","fiat","EUR","spot",100.0,0,100.0"#,
            r#""L2","T1","2017-11-02 10:00:00","trade","tradespot","currency","fiat","EUR","spot",-100.0,0,0.0"#,
            r#""L3","T1","2017-11-02 10:00:00","trade","tradespot","currency","crypto","BCH","spot",1.0,0,1.0"#,
            r#""L4","T2","2017-12-01 10:00:00","trade","tradespot","currency","crypto","BCH","spot",-1.0,0,0.0"#,
            r#""L5","T2","2017-12-01 10:00:00","trade","tradespot","currency","crypto","XXBT","spot",0.1,0.001,0.099"#,
        ]);
        let report = parse(&text, "l.csv").unwrap();
        assert!(close(report.summary.realised, 0.0));
        let btc = &report.holdings[0];
        assert_eq!(btc.asset, "BTC");
        assert!(btc.crypto);
        assert!(close(btc.quantity, 0.099) && close(btc.cost_basis, 100.0));
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
    }

    #[test]
    fn rows_are_sorted_and_fees_on_the_crypto_leg_reduce_quantity() {
        // Newest first; the buy's fee is taken in BTC.
        let text = ledger(&[
            r#""L3","T1","2021-01-02 10:00:00","trade","tradespot","currency","crypto","BTC","spot",0.02,0.0002,0.0198"#,
            r#""L2","T1","2021-01-02 10:00:00","trade","tradespot","currency","fiat","EUR","spot",-1000.0,0,0.0"#,
            r#""L1","D1","2021-01-01 10:00:00","deposit","","currency","fiat","EUR","spot",1000.0,0,1000.0"#,
        ]);
        let report = parse(&text, "l.csv").unwrap();
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        let btc = &report.holdings[0];
        assert!(close(btc.quantity, 0.0198) && close(btc.cost_basis, 1000.0));
        // 0.0002 BTC at 50 000 EUR/BTC.
        assert!(close(report.summary.fees, 10.0));
    }

    #[test]
    fn free_coins_have_zero_basis() {
        let text = ledger(&[
            r#""L1","A1","2018-11-18 10:00:00","transfer","","currency","crypto","BSV","spot",0.16,0,0.16"#,
            r#""L2","S1","2019-12-08 10:00:00","spend","","currency","crypto","BSV","spot",-0.16,0,0.0"#,
            r#""L3","S1","2019-12-08 10:00:00","receive","","currency","fiat","EUR","spot",20.0,0,20.0"#,
        ]);
        let report = parse(&text, "l.csv").unwrap();
        assert!(close(report.summary.realised, 20.0));
        assert!(report.warnings.iter().any(|w| w.contains("BSV")), "{:?}", report.warnings);
    }

    #[test]
    fn missing_rows_show_up_as_balance_mismatch() {
        let text = ledger(&[
            r#""L1","D1","2021-01-01 10:00:00","deposit","","currency","fiat","EUR","spot",100.0,0,100.0"#,
            r#""L2","D2","2021-01-03 10:00:00","deposit","","currency","fiat","EUR","spot",50.0,0,250.0"#,
        ]);
        let report = parse(&text, "l.csv").unwrap();
        assert!(report.warnings.iter().any(|w| w.contains("stämmer inte")), "{:?}", report.warnings);
    }

    #[test]
    fn base_currency_follows_the_first_fiat_deposit() {
        let text = ledger(&[
            r#""L1","D1","2021-01-01 10:00:00","deposit","","currency","fiat","ZUSD","spot",100.0,0,100.0"#,
        ]);
        assert_eq!(parse(&text, "l.csv").unwrap().currency, "USD");
    }
}
