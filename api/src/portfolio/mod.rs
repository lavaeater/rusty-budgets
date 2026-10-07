//! Stand-alone analysis of broker/exchange exports (Kraken, Avanza).
//!
//! Completely separate from the budget aggregate: nothing here is stored or
//! emits events. Files go in, a [`PortfolioReport`] comes out.

mod avanza;
mod book;
mod csv;
mod kraken;
mod xirr;

pub use xirr::xirr;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::ops::AddAssign;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    Kraken,
    Avanza,
}

/// Money flows over some span of time, in the account's currency. Every
/// field is positive in its natural direction (e.g. `withdrawn` is the
/// amount taken out), except the net figures `realised`, `interest`,
/// `dividends` and `other`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct FlowSummary {
    /// Paid in from a bank account.
    pub deposited: f64,
    /// Paid out to a bank account.
    pub withdrawn: f64,
    /// Received from other own accounts (cash, or securities at market value).
    pub transferred_in: f64,
    /// Sent to other own accounts (cash, or securities at market value).
    pub transferred_out: f64,
    /// Spent on purchases, fees included.
    pub bought: f64,
    /// Received from sales, fees deducted.
    pub sold: f64,
    /// Realised result using average cost.
    pub realised: f64,
    /// All fees, including those already part of `bought`/`sold`.
    pub fees: f64,
    pub interest: f64,
    pub dividends: f64,
    /// Tax paid.
    pub tax: f64,
    pub other: f64,
}

impl FlowSummary {
    /// Money put in from outside, net of what was taken back out.
    pub fn net_deposited(&self) -> f64 {
        self.deposited - self.withdrawn
    }

    /// Value moved in from other own accounts, net of what was moved out.
    pub fn net_transferred(&self) -> f64 {
        self.transferred_in - self.transferred_out
    }
}

impl AddAssign for FlowSummary {
    fn add_assign(&mut self, rhs: Self) {
        self.deposited += rhs.deposited;
        self.withdrawn += rhs.withdrawn;
        self.transferred_in += rhs.transferred_in;
        self.transferred_out += rhs.transferred_out;
        self.bought += rhs.bought;
        self.sold += rhs.sold;
        self.realised += rhs.realised;
        self.fees += rhs.fees;
        self.interest += rhs.interest;
        self.dividends += rhs.dividends;
        self.tax += rhs.tax;
        self.other += rhs.other;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YearSummary {
    pub year: i32,
    pub summary: FlowSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Holding {
    /// Ticker (crypto) or ISIN (securities); the key for prices.
    pub asset: String,
    pub name: String,
    pub quantity: f64,
    pub cost_basis: f64,
    /// Most recent price per unit seen in the file, in the account currency.
    pub last_price: Option<f64>,
    pub last_price_date: Option<NaiveDate>,
    pub crypto: bool,
}

/// Everything that happened to one asset in one account.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InstrumentSummary {
    pub asset: String,
    pub name: String,
    pub crypto: bool,
    /// Spent on purchases, fees included.
    pub bought: f64,
    /// Received from sales, fees deducted.
    pub sold: f64,
    pub realised: f64,
    pub fees: f64,
    pub dividends: f64,
    pub buys: u32,
    pub sells: u32,
    /// Quantity still held at the end of the file.
    pub quantity: f64,
}

/// Value the owner put into (positive) or took out of (negative) the
/// account on one day: deposits, withdrawals, transfers between own
/// accounts and assets sent out at cost. The input for [`xirr`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DatedAmount {
    pub date: NaiveDate,
    pub amount: f64,
}

/// An asset that entered or left the account without being bought or sold.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AssetMovement {
    pub asset: String,
    pub name: String,
    pub quantity: f64,
    pub cost_basis: f64,
    /// Market value at the time, when the export states it.
    pub value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountReport {
    pub name: String,
    pub provider: Provider,
    pub source_file: String,
    pub currency: String,
    pub first_date: Option<NaiveDate>,
    pub last_date: Option<NaiveDate>,
    pub summary: FlowSummary,
    pub cash_balance: f64,
    pub holdings: Vec<Holding>,
    pub assets_moved_in: Vec<AssetMovement>,
    pub assets_moved_out: Vec<AssetMovement>,
    pub years: Vec<YearSummary>,
    /// Per asset, largest realised result first.
    pub instruments: Vec<InstrumentSummary>,
    /// One entry per day with external flows, oldest first.
    pub contributions: Vec<DatedAmount>,
    /// Present when the export states its own result per sale.
    pub broker_check: Option<BrokerCheck>,
    pub warnings: Vec<String>,
}

impl AccountReport {
    pub fn holdings_cost_basis(&self) -> f64 {
        self.holdings.iter().map(|h| h.cost_basis).sum()
    }

    /// Cost basis of assets sent out without a sale (e.g. crypto withdrawn
    /// to an own wallet), which still belong to the owner.
    pub fn moved_out_at_cost(&self) -> f64 {
        self.assets_moved_out
            .iter()
            .filter(|m| m.value.is_none())
            .map(|m| m.cost_basis)
            .sum()
    }
}

/// The broker's own realised result (Avanza's `Resultat` column) next to
/// ours, summed over the rows where the broker states one. Avanza only fills
/// it in for newer rows, so this rarely covers the whole history.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BrokerCheck {
    pub broker_realised: f64,
    pub our_realised: f64,
    pub rows: usize,
}

/// Sums for all accounts sharing a currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrencyTotals {
    pub currency: String,
    pub summary: FlowSummary,
    pub cash_balance: f64,
    pub holdings_cost_basis: f64,
    pub moved_out_at_cost: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkippedFile {
    pub file_name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PortfolioReport {
    pub accounts: Vec<AccountReport>,
    pub totals: Vec<CurrencyTotals>,
    pub skipped_files: Vec<SkippedFile>,
}

/// Analyses an uploaded file: either a zip with one export per account, or
/// a single CSV export.
pub fn analyze_upload(file_name: &str, bytes: &[u8]) -> PortfolioReport {
    let files = match zip::ZipArchive::new(Cursor::new(bytes)) {
        Ok(mut archive) => extract_files(&mut archive),
        Err(_) => vec![(file_name.to_string(), bytes.to_vec())],
    };
    analyze_files(&files)
}

fn extract_files(archive: &mut zip::ZipArchive<Cursor<&[u8]>>) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else { continue };
        let path = entry.name().to_string();
        let file_name = path.rsplit('/').next().unwrap_or(&path).to_string();
        // Skip directories, macOS resource forks and other hidden files.
        if !entry.is_file() || path.starts_with("__MACOSX") || file_name.starts_with('.') {
            continue;
        }
        let mut contents = Vec::new();
        if entry.read_to_end(&mut contents).is_ok() {
            files.push((file_name, contents));
        }
    }
    files
}

pub fn analyze_files(files: &[(String, Vec<u8>)]) -> PortfolioReport {
    let mut report = PortfolioReport::default();
    let has_kraken_ledger = files
        .iter()
        .any(|(_, bytes)| kraken::is_ledger(first_line(&csv::decode(bytes))));

    for (file_name, bytes) in files {
        let text = csv::decode(bytes);
        let header = first_line(&text);
        let parsed = if kraken::is_ledger(header) {
            kraken::parse(&text, file_name).map(|account| vec![account])
        } else if avanza::is_transactions(header) {
            avanza::parse(&text, file_name)
        } else if kraken::is_trades(header) {
            Err(if has_kraken_ledger {
                "Krakens trades-export behövs inte — ledger-exporten innehåller samma affärer."
            } else {
                "Krakens trades-export saknar insättningar och uttag — exportera \"Ledgers\" i stället."
            }
            .to_string())
        } else {
            Err("Okänt filformat — stöds: Kraken ledgers och Avanza transaktioner (CSV).".to_string())
        };
        match parsed {
            Ok(accounts) => report.accounts.extend(accounts),
            Err(reason) => report.skipped_files.push(SkippedFile {
                file_name: file_name.clone(),
                reason,
            }),
        }
    }

    report.accounts.sort_by_key(|a| a.first_date);
    report.totals = totals(&report.accounts);
    report
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

fn totals(accounts: &[AccountReport]) -> Vec<CurrencyTotals> {
    let mut by_currency: BTreeMap<&str, CurrencyTotals> = BTreeMap::new();
    for account in accounts {
        let totals = by_currency
            .entry(account.currency.as_str())
            .or_insert_with(|| CurrencyTotals {
                currency: account.currency.clone(),
                summary: FlowSummary::default(),
                cash_balance: 0.0,
                holdings_cost_basis: 0.0,
                moved_out_at_cost: 0.0,
            });
        totals.summary += account.summary;
        totals.cash_balance += account.cash_balance;
        totals.holdings_cost_basis += account.holdings_cost_basis();
        totals.moved_out_at_cost += account.moved_out_at_cost();
    }
    by_currency.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    #[test]
    fn zip_entries_are_detected_and_unknown_files_skipped() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        for (name, contents) in [
            ("konton/avanza.csv", "Datum;Konto;Typ av transaktion;Värdepapper/beskrivning;Antal;Kurs;Belopp;Transaktionsvaluta;Courtage\n2024-01-01;1;Insättning;;;;100;SEK;"),
            ("__MACOSX/._avanza.csv", "junk"),
            ("anteckningar.txt", "hej"),
        ] {
            writer.start_file(name, options).unwrap();
            writer.write_all(contents.as_bytes()).unwrap();
        }
        let bytes = writer.finish().unwrap().into_inner();

        let report = analyze_upload("upload.zip", &bytes);
        assert_eq!(report.accounts.len(), 1);
        assert_eq!(report.accounts[0].source_file, "avanza.csv");
        assert_eq!(report.skipped_files.len(), 1);
        assert_eq!(report.skipped_files[0].file_name, "anteckningar.txt");
        assert_eq!(report.totals[0].summary.deposited, 100.0);
    }

    const AVANZA_HEADER: &str = "Datum;Konto;Typ av transaktion;Värdepapper/beskrivning;Antal;Kurs;Belopp;Transaktionsvaluta;Courtage";
    const KRAKEN_LEDGER: &str = "\"txid\",\"refid\",\"time\",\"type\",\"subtype\",\"aclass\",\"subclass\",\"asset\",\"wallet\",\"amount\",\"fee\",\"balance\"\n\
        \"L1\",\"D1\",\"2021-01-01 10:00:00\",\"deposit\",\"\",\"currency\",\"fiat\",\"EUR\",\"spot\",100.0,0,100.0";
    const KRAKEN_TRADES: &str = "\"txid\",\"ordertxid\",\"pair\",\"time\",\"type\"";

    fn file(name: &str, text: &str) -> (String, Vec<u8>) {
        (name.to_string(), text.as_bytes().to_vec())
    }

    #[test]
    fn a_plain_csv_upload_is_analysed() {
        let text = format!("{AVANZA_HEADER}\n2024-01-01;1;Insättning;;;;100;SEK;");
        let report = analyze_upload("transaktioner.csv", text.as_bytes());
        assert_eq!(report.accounts.len(), 1);
        assert_eq!(report.accounts[0].source_file, "transaktioner.csv");
    }

    #[test]
    fn kraken_trades_export_is_skipped_with_advice() {
        let alone = analyze_files(&[file("trades.csv", KRAKEN_TRADES)]);
        assert!(alone.skipped_files[0].reason.contains("Ledgers"));

        let with_ledger = analyze_files(&[file("trades.csv", KRAKEN_TRADES), file("ledger.csv", KRAKEN_LEDGER)]);
        assert_eq!(with_ledger.accounts.len(), 1);
        assert!(with_ledger.skipped_files[0].reason.contains("behövs inte"));
    }

    #[test]
    fn totals_are_kept_per_currency_and_internal_transfers_cancel() {
        let avanza = format!(
            "{AVANZA_HEADER}\n\
             2024-01-02;ISK;Intern överföring;Från AF;;;300;SEK;\n\
             2024-01-02;AF;Intern överföring;Till ISK;;;-300;SEK;\n\
             2024-01-01;AF;Insättning;;;;1000;SEK;"
        );
        let report = analyze_files(&[file("avanza.csv", &avanza), file("kraken.csv", KRAKEN_LEDGER)]);
        assert_eq!(report.accounts.len(), 3);
        let currencies: Vec<_> = report.totals.iter().map(|t| t.currency.as_str()).collect();
        assert_eq!(currencies, ["EUR", "SEK"]);
        let sek = &report.totals[1];
        assert!((sek.summary.net_transferred()).abs() < 1e-9);
        assert!((sek.summary.net_deposited() - 1000.0).abs() < 1e-9);
        assert!((sek.cash_balance - 1000.0).abs() < 1e-9);
        assert!((report.totals[0].summary.deposited - 100.0).abs() < 1e-9);
    }

    /// Runs against the real exports in `kraken/` (git-ignored, so this is
    /// a no-op elsewhere). `cargo test -p api real_exports -- --nocapture`
    /// prints the full report. Only invariants are asserted, so no personal
    /// figures end up in the repository.
    #[test]
    fn real_exports() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../kraken");
        let Ok(entries) = std::fs::read_dir(&dir) else { return };
        let files: Vec<(String, Vec<u8>)> = entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "csv"))
            .map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap()))
            .collect();
        let report = analyze_files(&files);
        for account in &report.accounts {
            println!("== {} ({})", account.name, account.currency);
            for i in &account.instruments {
                println!("  {:<45} crypto={} realised={:>12.2} fees={:>9.2} buys={} sells={}", i.name, i.crypto, i.realised, i.fees, i.buys, i.sells);
            }
            let mut flows: Vec<(NaiveDate, f64)> = account.contributions.iter().map(|c| (c.date, -c.amount)).collect();
            if let Some(last) = account.last_date {
                flows.push((last, account.cash_balance + account.holdings_cost_basis()));
            }
            println!("  xirr (holdings at cost) = {:?}", xirr(&flows));
        }
        for account in &report.accounts {
            // Kraken: our balances match the ledger's running balances.
            assert!(
                !account.warnings.iter().any(|w| w.contains("stämmer inte") || w.starts_with("Mer ")),
                "{}: {:?}",
                account.name,
                account.warnings
            );
            // Avanza: our realised result is within 1 % of Avanza's own,
            // on the rows where Avanza states one.
            if let Some(check) = account.broker_check {
                let tolerance = check.broker_realised.abs().max(1.0) * 0.01;
                assert!(
                    (check.our_realised - check.broker_realised).abs() <= tolerance,
                    "{}: {check:?}",
                    account.name
                );
            }
            // An emptied account has no cash left over beyond rounding.
            assert!(account.cash_balance > -1.0, "{}: {}", account.name, account.cash_balance);
        }
        // Money moved between own accounts arrives where it was sent.
        for totals in &report.totals {
            assert!(totals.summary.net_transferred().abs() < 1.0, "{totals:?}");
        }
    }
}
