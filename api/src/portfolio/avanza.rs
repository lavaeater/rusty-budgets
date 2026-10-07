//! Avanza's transaction export (`transaktioner_*.csv`): semicolon-separated,
//! Swedish decimal commas, newest row first. One file may cover several
//! accounts, so rows are grouped by the `Konto` column.

use super::book::Book;
use super::{AccountReport, Provider, csv};
use chrono::NaiveDate;

pub fn is_transactions(header: &str) -> bool {
    header.starts_with("Datum;Konto;Typ av transaktion")
}

/// Whole words in an instrument name that mark it as a crypto product
/// (ETPs and trackers such as "XBT Provider Bitcoin Tracker One").
const CRYPTO_WORDS: &[&str] = &[
    "bitcoin", "btc", "xbt", "ethereum", "ether", "eth", "crypto", "krypto", "solana", "cardano",
    "xrp", "ripple", "litecoin", "polkadot", "avalanche", "chainlink", "dogecoin", "valour",
    "coinshares", "21shares",
];

fn is_crypto_instrument(_isin: &str, name: &str) -> bool {
    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| CRYPTO_WORDS.contains(&word))
}

struct Columns {
    date: usize,
    account: usize,
    kind: usize,
    description: usize,
    quantity: usize,
    price: usize,
    amount: usize,
    currency: usize,
    fee: usize,
    fx_rate: Option<usize>,
    isin: Option<usize>,
    result: Option<usize>,
}

pub fn parse(text: &str, file_name: &str) -> Result<Vec<AccountReport>, String> {
    let records = csv::parse(text, ';');
    let (header, body) = records.split_first().ok_or("Filen är tom")?;
    let column = |name: &str| header.iter().position(|h| h.trim() == name);
    let required = |name: &str| column(name).ok_or(format!("Kolumnen \"{name}\" saknas"));
    let columns = Columns {
        date: required("Datum")?,
        account: required("Konto")?,
        kind: required("Typ av transaktion")?,
        description: required("Värdepapper/beskrivning")?,
        quantity: required("Antal")?,
        price: required("Kurs")?,
        amount: required("Belopp")?,
        currency: required("Transaktionsvaluta")?,
        fee: required("Courtage")?,
        fx_rate: column("Valutakurs"),
        isin: column("ISIN"),
        result: column("Resultat"),
    };

    // Accounts in order of first appearance, each with its own book.
    let mut books: Vec<(String, Book)> = Vec::new();
    // The export is newest first; walk it oldest first. Rows sharing a date
    // keep their relative order reversed, which matches how they happened.
    for record in body.iter().rev() {
        let cell = |i: usize| record.get(i).map_or("", |s| s.trim());
        let Ok(date) = NaiveDate::parse_from_str(cell(columns.date), "%Y-%m-%d") else {
            continue;
        };
        let account = cell(columns.account).to_string();
        let index = if let Some(i) = books.iter().position(|(a, _)| *a == account) {
            i
        } else {
            let currency = match cell(columns.currency) {
                "" => "SEK",
                c => c,
            };
            books.push((account.clone(), Book::new(currency).crypto_when(is_crypto_instrument)));
            books.len() - 1
        };
        let book = &mut books[index].1;
        apply_row(book, date, record, &columns);
    }

    Ok(books
        .into_iter()
        .map(|(account, book)| book.finish(format!("Avanza {account}"), Provider::Avanza, file_name.to_string()))
        .collect())
}

fn apply_row(book: &mut Book, date: NaiveDate, record: &[String], columns: &Columns) {
    let cell = |i: usize| record.get(i).map_or("", |s| s.trim());
    let kind = cell(columns.kind);
    let description = cell(columns.description);
    let quantity = csv::number(cell(columns.quantity));
    let amount = csv::number(cell(columns.amount));
    let fee = csv::number(cell(columns.fee)).unwrap_or(0.0);
    let price = csv::number(cell(columns.price));
    let fx_rate = columns.fx_rate.and_then(|i| csv::number(cell(i))).unwrap_or(1.0);
    let result = columns.result.and_then(|i| csv::number(cell(i)));
    let asset = columns
        .isin
        .map(cell)
        .filter(|isin| !isin.is_empty())
        .unwrap_or(description);
    // Market value in SEK for rows that move securities without money.
    let value = |q: f64| price.map(|p| p * fx_rate * q.abs());

    match (kind, quantity, amount) {
        ("Köp", Some(q), Some(a)) => book.buy(date, asset, description, q.abs(), -a, fee),
        ("Sälj", Some(q), Some(a)) => {
            let ours = book.sell(date, asset, description, q.abs(), a, fee);
            if let Some(r) = result {
                book.add_broker_result(r, ours);
            }
        }
        ("Insättning", _, Some(a)) => book.deposit(date, a, 0.0),
        ("Uttag", _, Some(a)) => book.withdraw(date, -a, 0.0),
        ("Intern överföring", _, Some(a)) => book.transfer_cash(date, a),
        ("Värdepappersuttag", Some(q), _) => {
            let ours = book.send_out(date, asset, asset, q.abs(), 0.0, Some(value(q).unwrap_or(0.0)));
            if let Some(r) = result {
                book.add_broker_result(r, ours);
            }
        }
        ("Värdepappersinsättning", Some(q), _) => {
            book.receive_transfer(date, asset, asset, q.abs(), value(q).unwrap_or(0.0));
        }
        ("Inbokning", Some(q), _) => book.receive_free(date, asset, description, q.abs()),
        // Expired or redeemed without payment, e.g. unused subscription rights.
        ("Utbokning", Some(q), None) => {
            book.send_out(date, asset, description, q.abs(), 0.0, Some(0.0));
        }
        (k, _, Some(a)) if k.starts_with("Utdelning") => {
            let has_asset = columns.isin.is_some_and(|i| !cell(i).is_empty());
            book.dividend(date, has_asset.then_some(asset), a);
        }
        (k, _, Some(a)) if k.to_lowercase().contains("skatt") => book.tax(date, a),
        (k, _, Some(a)) if k.to_lowercase().contains("ränta") => book.interest(date, a),
        (k, _, Some(a)) => {
            book.other(date, a);
            book.warn(format!("Okänd transaktionstyp \"{k}\" — beloppet räknas som övrigt."));
        }
        (k, _, None) => book.warn(format!("Raden \"{k}\" ({description}) saknar belopp och hoppades över.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "\u{feff}Datum;Konto;Typ av transaktion;Värdepapper/beskrivning;Antal;Kurs;Belopp;\
        Transaktionsvaluta;Courtage;Valutakurs;Instrumentvaluta;ISIN;Resultat";

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn groups_accounts_and_walks_oldest_first() {
        let text = [
            HEADER,
            "2024-03-01;ISK;Sälj;Fond;-5;30;148;SEK;2;;SEK;SE01;48",
            "2024-02-15;ISK;Värdepappersinsättning;Flytt;10;20;;;;;SEK;SE01;",
            "2024-02-15;AF;Värdepappersuttag;Flytt;-10;20;;;;;SEK;SE01;100",
            "2024-02-01;AF;Preliminärskatt kapitalränta;;;;-0,43;SEK;;;;;",
            "2024-02-01;AF;Inlåningsränta;;;;1,42;SEK;;;;;",
            "2024-01-02;AF;Köp;Fond;10;10;-101;SEK;1;;SEK;SE01;",
            "2024-01-01;AF;Insättning;Bank;;;1000;SEK;;;;;",
        ]
        .join("\n");
        let reports = parse(&csv::decode(text.as_bytes()), "t.csv").unwrap();
        assert_eq!(reports.len(), 2);

        let af = &reports[0];
        assert_eq!(af.name, "Avanza AF");
        assert!(af.warnings.is_empty(), "{:?}", af.warnings);
        assert!(close(af.summary.deposited, 1000.0));
        assert!(close(af.summary.realised, 200.0 - 101.0));
        let check = af.broker_check.unwrap();
        assert!(close(check.broker_realised, 100.0) && close(check.our_realised, 99.0));
        assert_eq!(check.rows, 1);
        assert!(close(af.summary.transferred_out, 200.0));
        assert!(close(af.summary.interest, 1.42));
        assert!(close(af.summary.tax, 0.43));
        assert!(close(af.cash_balance, 1000.0 - 101.0 + 1.42 - 0.43));
        assert!(af.holdings.is_empty(), "{:?}", af.holdings);

        let isk = &reports[1];
        assert!(close(isk.summary.transferred_in, 200.0));
        assert!(close(isk.summary.realised, 148.0 - 100.0));
        assert!(close(isk.holdings[0].quantity, 5.0));
        assert!(close(isk.holdings[0].cost_basis, 100.0));
        assert_eq!(isk.holdings[0].last_price, Some(30.0));
    }
    fn parse_rows(rows: &[&str]) -> Vec<AccountReport> {
        let text = std::iter::once(HEADER).chain(rows.iter().copied()).collect::<Vec<_>>().join("\n");
        parse(&csv::decode(text.as_bytes()), "t.csv").unwrap()
    }

    #[test]
    fn average_cost_with_courtage_over_several_buys() {
        // Newest first, like the real export.
        let reports = parse_rows(&[
            "2024-01-04;1;Sälj;Fond;-15;40;590;SEK;10;;SEK;SE01;155",
            "2024-01-03;1;Köp;Fond;10;30;-305;SEK;5;;SEK;SE01;",
            "2024-01-02;1;Köp;Fond;10;10;-105;SEK;5;;SEK;SE01;",
        ]);
        let account = &reports[0];
        // 20 units cost 410 (20,50 each); 15 of them carry 307,50.
        assert!(close(account.summary.realised, 590.0 - 307.5));
        assert!(close(account.holdings[0].quantity, 5.0));
        assert!(close(account.holdings[0].cost_basis, 102.5));
        assert!(close(account.summary.fees, 20.0));
        assert!(close(account.cash_balance, -105.0 - 305.0 + 590.0));
    }

    #[test]
    fn foreign_instruments_use_the_sek_amount() {
        let reports = parse_rows(&[
            "2021-11-05;1;Sälj;GME;-10;25;2400;SEK;20;10;USD;US01;",
            "2021-11-01;1;Köp;GME;10;20;-2010;SEK;10;9,95;USD;US01;",
        ]);
        let account = &reports[0];
        assert!(close(account.summary.realised, 390.0));
        assert!(account.broker_check.is_none());
        // Moving it out values it at quote × exchange rate.
        let moved = parse_rows(&[
            "2021-11-02;1;Värdepappersuttag;Flytt;-10;20;;;;9,95;USD;US01;",
            "2021-11-01;1;Köp;GME;10;20;-2010;SEK;10;9,95;USD;US01;",
        ]);
        assert!(close(moved[0].summary.transferred_out, 1990.0));
        assert!(close(moved[0].summary.realised, 1990.0 - 2010.0));
    }

    #[test]
    fn booked_in_and_out_securities() {
        let reports = parse_rows(&[
            "2021-10-13;1;Utbokning;Rätter;-100;;;;;;;SE02;",
            "2021-10-12;1;Utbokning;Teckningsoption;-10;;;;;;;SE01;",
            "2021-09-01;1;Köp;Rätter;100;1;-100;SEK;0;;SEK;SE02;",
            "2019-10-24;1;Inbokning;Teckningsoption;10;;;;;;;SE01;",
        ]);
        let account = &reports[0];
        assert!(account.holdings.is_empty(), "{:?}", account.holdings);
        // The free warrants expire at no loss; the bought rights lose their cost.
        assert!(close(account.summary.realised, -100.0));
        assert!(account.warnings.is_empty(), "{:?}", account.warnings);
    }

    #[test]
    fn cash_only_rows() {
        let reports = parse_rows(&[
            "2024-05-01;1;Okänd sak;;;;5;SEK;;;;;",
            "2024-04-30;1;Utlåningsränta;;;;-8,33;SEK;;;;;",
            "2024-04-01;1;Utländsk källskatt;;;;-1,5;SEK;;;;;",
            "2024-04-01;1;Utdelning;Bolag;10;1;10;SEK;;;SEK;SE03;",
            "2024-03-01;1;Uttag;Uttag till 123;;;-50;SEK;;;;;",
            "2024-02-01;1;Intern överföring;Till 2;;;-25;SEK;;;;;",
            "2024-01-01;1;Insättning;Swish;;;100;SEK;;;;;",
        ]);
        let account = &reports[0];
        let s = account.summary;
        assert!(close(s.deposited, 100.0) && close(s.withdrawn, 50.0));
        assert!(close(s.transferred_out, 25.0));
        assert!(close(s.dividends, 10.0) && close(s.tax, 1.5) && close(s.interest, -8.33));
        assert!(close(s.other, 5.0));
        assert!(close(account.cash_balance, 100.0 - 25.0 - 50.0 + 10.0 - 1.5 - 8.33 + 5.0));
        assert_eq!(account.warnings.len(), 1, "{:?}", account.warnings);
        assert!(account.warnings[0].contains("Okänd sak"));
    }

    #[test]
    fn latin1_export_is_read() {
        let mut bytes = b"Datum;Konto;Typ av transaktion;V\xE4rdepapper/beskrivning;Antal;Kurs;Belopp;Transaktionsvaluta;Courtage\n".to_vec();
        bytes.extend_from_slice(b"2024-01-01;1;Ins\xE4ttning;;;;100;SEK;\n");
        let reports = parse(&csv::decode(&bytes), "t.csv").unwrap();
        assert!(close(reports[0].summary.deposited, 100.0));
    }

    #[test]
    fn missing_column_is_an_error() {
        assert!(parse("Datum;Konto;Typ av transaktion\n", "t.csv").is_err());
    }

    #[test]
    fn crypto_products_are_recognised_by_name() {
        for name in [
            "XBT Provider Bitcoin Tracker One - SEK",
            "Valour Bitcoin (BTC) Zero SEK",
            "Bitwise Core Bitcoin ETP",
            "CoinShares Physical Ethereum",
        ] {
            assert!(is_crypto_instrument("", name), "{name}");
        }
        for name in ["GameStop", "Avanza Zero", "Länsförsäkringar Japan Index", "Ethos Holding", "Bethesda"] {
            assert!(!is_crypto_instrument("", name), "{name}");
        }
    }
}
