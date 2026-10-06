//! Avanza's transaction export (`transaktioner_*.csv`): semicolon-separated,
//! Swedish decimal commas, newest row first. One file may cover several
//! accounts, so rows are grouped by the `Konto` column.

use super::book::Book;
use super::{AccountReport, Provider, csv};
use chrono::NaiveDate;

pub fn is_transactions(header: &str) -> bool {
    header.starts_with("Datum;Konto;Typ av transaktion")
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
            books.push((account.clone(), Book::new(currency)));
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
        (k, _, Some(a)) if k.starts_with("Utdelning") => book.dividend(date, a),
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
}
