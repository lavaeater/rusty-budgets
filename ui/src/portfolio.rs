//! Investment overview built from broker/exchange exports (Kraken, Avanza).
//!
//! Self-contained: it never reads or writes the budget. The uploaded files
//! are analysed by [`api::analyze_portfolio`] and the result lives only in
//! this view's state. Current prices are typed in here, since the exports
//! only know historical ones.

use crate::Input;
use crate::file_chooser::{FileData, FileDialog};
use api::analyze_portfolio;
use api::portfolio::{AccountReport, FlowSummary, Holding, PortfolioReport, xirr};
use chrono::{NaiveDate, Utc};
use dioxus::prelude::*;
use std::collections::HashMap;

const PORTFOLIO_CSS: Asset = asset!("assets/styling/portfolio.css");

/// Typed-in prices, keyed by [`price_key`].
type Prices = Signal<HashMap<String, String>>;

fn price_key(currency: &str, asset: &str) -> String {
    format!("{currency}|{asset}")
}

fn parse_decimal(text: &str) -> Option<f64> {
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c == ',' { '.' } else { c })
        .collect();
    cleaned.parse().ok().filter(|v: &f64| v.is_finite())
}

/// Swedish formatting: `-1 234 567,89`.
fn format_number(value: f64, decimals: usize) -> String {
    let rounded = format!("{:.*}", decimals, value.abs());
    let (int_part, frac_part) = rounded.split_once('.').unwrap_or((&rounded, ""));
    let mut grouped = String::new();
    for (i, digit) in int_part.chars().enumerate() {
        if i > 0 && (int_part.len() - i).is_multiple_of(3) {
            grouped.push('\u{a0}');
        }
        grouped.push(digit);
    }
    let is_zero = rounded.chars().all(|c| c == '0' || c == '.');
    let sign = if value < 0.0 && !is_zero { "-" } else { "" };
    if frac_part.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped},{frac_part}")
    }
}

fn format_money(value: f64, currency: &str) -> String {
    format!("{} {currency}", format_number(value, 2))
}

/// Up to eight decimals, without trailing zeros.
fn format_quantity(value: f64) -> String {
    let text = format_number(value, 8);
    if text.contains(',') {
        text.trim_end_matches('0').trim_end_matches(',').to_string()
    } else {
        text
    }
}

fn sign_class(value: f64) -> &'static str {
    if value > 0.005 {
        "pos"
    } else if value < -0.005 {
        "neg"
    } else {
        ""
    }
}

/// Market value of `holdings`, falling back to cost basis where no price
/// has been entered.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Valuation {
    cost: f64,
    market_value: f64,
    missing_prices: usize,
}

impl Valuation {
    fn of(holdings: &[Holding], currency: &str, prices: &HashMap<String, String>) -> Self {
        let mut valuation = Valuation::default();
        for holding in holdings {
            valuation.cost += holding.cost_basis;
            let price = prices
                .get(&price_key(currency, &holding.asset))
                .and_then(|p| parse_decimal(p));
            if let Some(price) = price {
                valuation.market_value += price * holding.quantity;
            } else {
                valuation.market_value += holding.cost_basis;
                valuation.missing_prices += 1;
            }
        }
        valuation
    }

    fn unrealised(&self) -> f64 {
        self.market_value - self.cost
    }
}

/// Everything the account is worth now (plus assets sent out to own
/// wallets, at cost) minus everything that was put into it.
fn total_result(summary: &FlowSummary, cash: f64, market_value: f64, moved_out_at_cost: f64) -> f64 {
    cash + market_value + moved_out_at_cost - summary.net_deposited() - summary.net_transferred()
}

fn format_percent(rate: Option<f64>) -> String {
    rate.map_or_else(|| "—".to_string(), |r| format!("{} %/år", format_number(r * 100.0, 1)))
}

/// Annualised money-weighted return: the owner's dated contributions
/// (positive = put in), with everything still in the accounts treated as
/// taken out at `end`.
fn annual_return(contributions: &[(NaiveDate, f64)], end: NaiveDate, end_value: f64) -> Option<f64> {
    let mut flows: Vec<(NaiveDate, f64)> = contributions.iter().map(|(d, a)| (*d, -a)).collect();
    flows.push((end, end_value));
    xirr(&flows)
}

/// Accounts that still hold something are valued today; emptied ones on
/// their last transaction, so idle time after closing does not dilute the
/// rate.
fn valuation_date<'a>(accounts: impl IntoIterator<Item = &'a AccountReport>) -> NaiveDate {
    let today = Utc::now().date_naive();
    let mut last: Option<NaiveDate> = None;
    for account in accounts {
        if !account.holdings.is_empty() {
            return today;
        }
        last = last.max(account.last_date);
    }
    last.unwrap_or(today)
}

fn contributions_of<'a>(accounts: impl IntoIterator<Item = &'a AccountReport>, rate: f64) -> Vec<(NaiveDate, f64)> {
    accounts
        .into_iter()
        .flat_map(|a| a.contributions.iter().map(move |c| (c.date, c.amount * rate)))
        .collect()
}

/// Result attributable to crypto (or to everything else) in `accounts`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Split {
    realised: f64,
    unrealised: f64,
    fees: f64,
    dividends: f64,
}

impl Split {
    fn of(accounts: &[&AccountReport], crypto: bool, prices: &HashMap<String, String>) -> Self {
        let mut split = Split::default();
        for account in accounts {
            for instrument in account.instruments.iter().filter(|i| i.crypto == crypto) {
                split.realised += instrument.realised;
                split.fees += instrument.fees;
                split.dividends += instrument.dividends;
            }
            let holdings: Vec<Holding> = account.holdings.iter().filter(|h| h.crypto == crypto).cloned().collect();
            split.unrealised += Valuation::of(&holdings, &account.currency, prices).unrealised();
        }
        split
    }

    fn result(&self) -> f64 {
        self.realised + self.unrealised + self.dividends
    }

    fn is_empty(&self) -> bool {
        self.realised == 0.0 && self.unrealised == 0.0 && self.fees == 0.0 && self.dividends == 0.0
    }
}

/// Seeds the price inputs with the most recent price seen in the files.
fn default_prices(report: &PortfolioReport) -> HashMap<String, String> {
    let mut latest: HashMap<String, (Option<chrono::NaiveDate>, f64)> = HashMap::new();
    for account in &report.accounts {
        for holding in &account.holdings {
            let Some(price) = holding.last_price else { continue };
            let key = price_key(&account.currency, &holding.asset);
            let entry = latest.entry(key).or_insert((holding.last_price_date, price));
            if holding.last_price_date > entry.0 {
                *entry = (holding.last_price_date, price);
            }
        }
    }
    latest
        .into_iter()
        .map(|(key, (_, price))| (key, format!("{price:.2}")))
        .collect()
}

#[component]
pub fn PortfolioView() -> Element {
    let mut report: Signal<Option<PortfolioReport>> = use_signal(|| None);
    let mut status: Signal<Option<String>> = use_signal(|| None);
    let mut prices: Prices = use_signal(HashMap::new);
    // Currency → SEK rate, for the combined total.
    let rates: Prices = use_signal(HashMap::new);

    let analyze = move |file: FileData| {
        status.set(Some(format!("Analyserar {}…", file.name)));
        spawn(async move {
            match analyze_portfolio(file.name, file.contents).await {
                Ok(result) => {
                    prices.set(default_prices(&result));
                    status.set(None);
                    report.set(Some(result));
                }
                Err(e) => status.set(Some(format!("Kunde inte analysera filen: {e}"))),
            }
        });
    };

    rsx! {
        document::Link { rel: "stylesheet", href: PORTFOLIO_CSS }
        div { class: "tab-panel",
            section { class: "report-section",
                h3 { class: "report-section-title", "Investeringar" }
                p { class: "settings-hint",
                    "Välj en zip-fil med exporter — en CSV-fil per konto — eller en enskild CSV-fil. "
                    "Stöds: Kraken (Ledgers-export) och Avanza (transaktioner). "
                    "Inget sparas och budgeten påverkas inte."
                }
                FileDialog {
                    on_chosen: analyze,
                    label: "Analysera exporter",
                    title: "Välj en zip- eller CSV-fil",
                    filter_name: "Zip/CSV",
                    filter_extensions: vec!["zip".to_string(), "csv".to_string()],
                }
                if let Some(message) = status() {
                    p { class: "portfolio-status", {message} }
                }
            }

            if let Some(report) = report() {
                if !report.skipped_files.is_empty() {
                    section { class: "report-section portfolio-skipped",
                        h4 { "Filer som inte analyserades" }
                        ul {
                            for skipped in report.skipped_files.iter() {
                                li {
                                    strong { {skipped.file_name.clone()} }
                                    " — {skipped.reason}"
                                }
                            }
                        }
                    }
                }
                if report.accounts.is_empty() {
                    p { class: "report-empty", "Inga konton hittades i filen." }
                } else {
                    PortfolioTotals { report: report.clone(), prices, rates }
                    for account in report.accounts.iter() {
                        AccountCard { account: account.clone(), prices }
                    }
                }
            }
        }
    }
}

/// One row per currency, plus a combined SEK figure once exchange rates for
/// the other currencies are entered.
#[component]
fn PortfolioTotals(report: PortfolioReport, prices: Prices, mut rates: Prices) -> Element {
    let price_map = prices();
    let rate_map = rates();
    let rows: Vec<_> = report
        .totals
        .iter()
        .map(|totals| {
            let holdings: Vec<Holding> = report
                .accounts
                .iter()
                .filter(|a| a.currency == totals.currency)
                .flat_map(|a| a.holdings.iter().cloned())
                .collect();
            let valuation = Valuation::of(&holdings, &totals.currency, &price_map);
            let value = totals.cash_balance + valuation.market_value;
            let result = total_result(
                &totals.summary,
                totals.cash_balance,
                valuation.market_value,
                totals.moved_out_at_cost,
            );
            let accounts: Vec<&AccountReport> =
                report.accounts.iter().filter(|a| a.currency == totals.currency).collect();
            let annual = annual_return(
                &contributions_of(accounts.iter().copied(), 1.0),
                valuation_date(accounts.iter().copied()),
                value,
            );
            (totals.clone(), valuation, value, result, annual)
        })
        .collect();
    let splits: Vec<(String, &str, Split)> = report
        .totals
        .iter()
        .flat_map(|totals| {
            let accounts: Vec<&AccountReport> =
                report.accounts.iter().filter(|a| a.currency == totals.currency).collect();
            [(true, "Krypto"), (false, "Övrigt")]
                .map(|(crypto, label)| (totals.currency.clone(), label, Split::of(&accounts, crypto, &price_map)))
        })
        .filter(|(.., split)| !split.is_empty())
        .collect();

    let foreign: Vec<String> = rows
        .iter()
        .map(|(t, ..)| t.currency.clone())
        .filter(|c| c != "SEK")
        .collect();
    let rate = |currency: &str| {
        if currency == "SEK" {
            Some(1.0)
        } else {
            rate_map.get(currency).and_then(|r| parse_decimal(r))
        }
    };
    let combined = rows.iter().try_fold((0.0, 0.0, 0.0, Vec::new()), |mut acc, (t, _, value, result, _)| {
        let r = rate(&t.currency)?;
        acc.0 += (t.summary.net_deposited() + t.summary.net_transferred()) * r;
        acc.1 += value * r;
        acc.2 += result * r;
        let accounts = report.accounts.iter().filter(|a| a.currency == t.currency);
        acc.3.extend(contributions_of(accounts, r));
        Some(acc)
    });
    let combined = combined.map(|(invested, value, result, contributions)| {
        let annual = annual_return(&contributions, valuation_date(&report.accounts), value);
        (invested, value, result, annual)
    });

    rsx! {
        section { class: "report-section",
            h3 { class: "report-section-title", "Totalt" }
            table { class: "report-table",
                thead {
                    tr {
                        th { "Valuta" }
                        th { class: "num", "Netto insatt" }
                        th { class: "num", "Värde nu" }
                        th { class: "num", "Realiserat" }
                        th { class: "num", "Orealiserat" }
                        th { class: "num", "Avgifter" }
                        th { class: "num", "Totalt resultat" }
                        th { class: "num", "Årlig avkastning" }
                    }
                }
                tbody {
                    for (totals , valuation , value , result , annual) in rows.iter() {
                        tr {
                            td { {totals.currency.clone()} }
                            td { class: "num",
                                {format_money(totals.summary.net_deposited() + totals.summary.net_transferred(), &totals.currency)}
                            }
                            td { class: "num", {format_money(*value, &totals.currency)} }
                            td { class: "num {sign_class(totals.summary.realised)}",
                                {format_money(totals.summary.realised, &totals.currency)}
                            }
                            td { class: "num {sign_class(valuation.unrealised())}",
                                {format_money(valuation.unrealised(), &totals.currency)}
                            }
                            td { class: "num", {format_money(totals.summary.fees, &totals.currency)} }
                            td { class: "num portfolio-strong {sign_class(*result)}",
                                {format_money(*result, &totals.currency)}
                            }
                            td { class: "num {sign_class(annual.unwrap_or_default())}", {format_percent(*annual)} }
                        }
                    }
                }
            }

            h4 { class: "portfolio-subtitle", "Krypto och övrigt" }
            table { class: "report-table",
                thead {
                    tr {
                        th { "Valuta" }
                        th { "Slag" }
                        th { class: "num", "Realiserat" }
                        th { class: "num", "Orealiserat" }
                        th { class: "num", "Utdelningar" }
                        th { class: "num", "Resultat" }
                        th { class: "num", "Varav avgifter" }
                    }
                }
                tbody {
                    for (currency , label , split) in splits.iter() {
                        tr {
                            td { {currency.clone()} }
                            td { {*label} }
                            td { class: "num {sign_class(split.realised)}", {format_money(split.realised, currency)} }
                            td { class: "num {sign_class(split.unrealised)}", {format_money(split.unrealised, currency)} }
                            td { class: "num", {format_money(split.dividends, currency)} }
                            td { class: "num portfolio-strong {sign_class(split.result())}",
                                {format_money(split.result(), currency)}
                            }
                            td { class: "num", {format_money(split.fees, currency)} }
                        }
                    }
                }
            }
            p { class: "settings-hint portfolio-note",
                "Netto insatt = insättningar − uttag (överföringar mellan egna konton tar ut varandra). "
                "Totalt resultat = kassa + marknadsvärde + krypto flyttat till egen plånbok (till anskaffningsvärde) − netto insatt. "
                "Årlig avkastning är pengaviktad (XIRR): tar hänsyn till när pengarna sattes in och togs ut. "
                "Krypto flyttat till egen plånbok räknas som uttag till anskaffningsvärdet. "
                "Krypto och övrigt delar upp resultatet per värdepapper; ränta och skatt hör till kontot och ingår inte."
            }

            if !foreign.is_empty() {
                div { class: "portfolio-rates",
                    for currency in foreign {
                        label { class: "portfolio-rate",
                            "1 {currency} = "
                            Input {
                                class: "portfolio-price-input",
                                placeholder: "kurs",
                                value: rate_map.get(&currency).cloned().unwrap_or_default(),
                                oninput: {
                                    let currency = currency.clone();
                                    move |e: FormEvent| {
                                        rates.write().insert(currency.clone(), e.value());
                                    }
                                },
                            }
                            " SEK"
                        }
                    }
                }
                if let Some((invested, value, result, annual)) = combined {
                    p { class: "portfolio-combined",
                        "Allt omräknat till SEK med dagens kurs: netto insatt "
                        strong { {format_money(invested, "SEK")} }
                        ", värde nu "
                        strong { {format_money(value, "SEK")} }
                        ", resultat "
                        strong { class: sign_class(result), {format_money(result, "SEK")} }
                        ", årlig avkastning "
                        strong { class: sign_class(annual.unwrap_or_default()), {format_percent(annual)} }
                    }
                }
            }
        }
    }
}

#[component]
fn AccountCard(account: AccountReport, mut prices: Prices) -> Element {
    let currency = account.currency.clone();
    let price_map = prices();
    let summary = account.summary;
    let valuation = Valuation::of(&account.holdings, &currency, &price_map);
    let moved_out_at_cost = account.moved_out_at_cost();
    let result = total_result(&summary, account.cash_balance, valuation.market_value, moved_out_at_cost);
    let annual = annual_return(
        &contributions_of([&account], 1.0),
        valuation_date([&account]),
        account.cash_balance + valuation.market_value,
    );
    let money = |v: f64| format_money(v, &currency);
    let period = match (account.first_date, account.last_date) {
        (Some(first), Some(last)) => format!("{first} – {last}"),
        _ => String::new(),
    };

    let mut kpis: Vec<(&str, f64, bool)> = vec![
        ("Insatt", summary.deposited, false),
        ("Uttaget", summary.withdrawn, false),
        ("Netto insatt", summary.net_deposited(), false),
    ];
    if summary.transferred_in != 0.0 || summary.transferred_out != 0.0 {
        kpis.push(("Överfört in (egna konton)", summary.transferred_in, false));
        kpis.push(("Överfört ut (egna konton)", summary.transferred_out, false));
    }
    kpis.extend([
        ("Kassa", account.cash_balance, false),
        ("Innehav, anskaffningsvärde", valuation.cost, false),
        ("Innehav, marknadsvärde", valuation.market_value, false),
        ("Realiserat resultat", summary.realised, true),
        ("Orealiserat resultat", valuation.unrealised(), true),
        ("Avgifter", summary.fees, false),
    ]);
    if summary.interest != 0.0 {
        kpis.push(("Ränta (netto)", summary.interest, true));
    }
    if summary.dividends != 0.0 {
        kpis.push(("Utdelningar", summary.dividends, true));
    }
    if summary.tax != 0.0 {
        kpis.push(("Skatt", summary.tax, false));
    }
    if summary.other != 0.0 {
        kpis.push(("Övrigt", summary.other, true));
    }

    rsx! {
        section { class: "report-section portfolio-account",
            div { class: "portfolio-account-header",
                h3 { class: "report-section-title", {account.name.clone()} }
                span { class: "portfolio-account-meta", "{account.source_file} · {period}" }
            }

            div { class: "portfolio-result",
                span { "Totalt resultat" }
                strong { class: sign_class(result), {money(result)} }
                span { "Årlig avkastning" }
                strong { class: sign_class(annual.unwrap_or_default()), {format_percent(annual)} }
            }

            dl { class: "portfolio-kpis",
                for (label , value , signed) in kpis {
                    div { class: "portfolio-kpi",
                        dt { {label} }
                        dd { class: if signed { sign_class(value) } else { "" }, {money(value)} }
                    }
                }
            }

            if let Some(check) = account.broker_check {
                p { class: "settings-hint",
                    "Kontroll: på de {check.rows} rader där exporten anger resultat säger mäklaren "
                    strong { {money(check.broker_realised)} }
                    ", vår beräkning ger "
                    strong { {money(check.our_realised)} }
                    "."
                }
            }

            if !account.instruments.is_empty() {
                InstrumentTable { account: account.clone() }
            }

            if !account.holdings.is_empty() {
                h4 { class: "portfolio-subtitle", "Innehav" }
                table { class: "report-table",
                    thead {
                        tr {
                            th { "Tillgång" }
                            th { class: "num", "Antal" }
                            th { class: "num", "GAV" }
                            th { class: "num", "Anskaffning" }
                            th { class: "num", "Kurs nu ({currency})" }
                            th { class: "num", "Värde" }
                            th { class: "num", "Orealiserat" }
                        }
                    }
                    tbody {
                        for holding in account.holdings.iter() {
                            {
                                let key = price_key(&currency, &holding.asset);
                                let price_text = price_map.get(&key).cloned().unwrap_or_default();
                                let price = parse_decimal(&price_text);
                                let value = price.map(|p| p * holding.quantity);
                                let average = if holding.quantity.abs() > f64::EPSILON {
                                    holding.cost_basis / holding.quantity
                                } else {
                                    0.0
                                };
                                let hint = match (holding.last_price, holding.last_price_date) {
                                    (Some(p), Some(d)) => format!("Senast i filen: {} ({d})", format_number(p, 2)),
                                    _ => String::new(),
                                };
                                rsx! {
                                    tr { key: "{key}",
                                        td {
                                            {holding.name.clone()}
                                            if holding.name != holding.asset {
                                                span { class: "portfolio-asset-id", " {holding.asset}" }
                                            }
                                        }
                                        td { class: "num", {format_quantity(holding.quantity)} }
                                        td { class: "num", {format_number(average, 2)} }
                                        td { class: "num", {money(holding.cost_basis)} }
                                        td { class: "num", title: hint,
                                            Input {
                                                class: "portfolio-price-input",
                                                value: price_text,
                                                oninput: move |e: FormEvent| {
                                                    prices.write().insert(key.clone(), e.value());
                                                },
                                            }
                                        }
                                        td { class: "num", {value.map_or_else(|| "—".to_string(), money)} }
                                        td { class: "num {value.map_or(\"\", |v| sign_class(v - holding.cost_basis))}",
                                            {value.map_or_else(|| "—".to_string(), |v| money(v - holding.cost_basis))}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if valuation.missing_prices > 0 {
                    p { class: "settings-hint",
                        "Kurs saknas för {valuation.missing_prices} innehav — de värderas till anskaffningsvärdet."
                    }
                }
            }

            if !account.assets_moved_out.is_empty() || !account.assets_moved_in.is_empty() {
                h4 { class: "portfolio-subtitle", "Flyttat utan köp eller försäljning" }
                table { class: "report-table",
                    thead {
                        tr {
                            th { "Riktning" }
                            th { "Tillgång" }
                            th { class: "num", "Antal" }
                            th { class: "num", "Anskaffningsvärde" }
                            th { class: "num", "Marknadsvärde då" }
                        }
                    }
                    tbody {
                        for (direction , moved) in account.assets_moved_in.iter().map(|m| ("In", m)).chain(account.assets_moved_out.iter().map(|m| ("Ut", m))) {
                            tr {
                                td { {direction} }
                                td { {moved.name.clone()} }
                                td { class: "num", {format_quantity(moved.quantity)} }
                                td { class: "num", {money(moved.cost_basis)} }
                                td { class: "num", {moved.value.map_or_else(|| "—".to_string(), money)} }
                            }
                        }
                    }
                }
            }

            h4 { class: "portfolio-subtitle", "Per år" }
            div { class: "portfolio-table-scroll",
                table { class: "report-table",
                    thead {
                        tr {
                            th { "År" }
                            th { class: "num", title: "Insättningar och överföringar från egna konton", "In" }
                            th { class: "num", title: "Uttag och överföringar till egna konton", "Ut" }
                            th { class: "num", "Köpt" }
                            th { class: "num", "Sålt" }
                            th { class: "num", "Realiserat" }
                            th { class: "num", "Avgifter" }
                        }
                    }
                    tbody {
                        for year in account.years.iter() {
                            tr {
                                td { "{year.year}" }
                                td { class: "num", {money(year.summary.deposited + year.summary.transferred_in)} }
                                td { class: "num", {money(year.summary.withdrawn + year.summary.transferred_out)} }
                                td { class: "num", {money(year.summary.bought)} }
                                td { class: "num", {money(year.summary.sold)} }
                                td { class: "num {sign_class(year.summary.realised)}", {money(year.summary.realised)} }
                                td { class: "num", {money(year.summary.fees)} }
                            }
                        }
                    }
                }
            }

            if !account.warnings.is_empty() {
                details { class: "portfolio-warnings",
                    summary { "{account.warnings.len()} anmärkningar" }
                    ul {
                        for warning in account.warnings.iter() {
                            li { {warning.clone()} }
                        }
                    }
                }
            }
        }
    }
}

/// Per-instrument results, with crypto and everything else subtotalled.
#[component]
fn InstrumentTable(account: AccountReport) -> Element {
    let currency = account.currency.clone();
    let money = |v: f64| format_money(v, &currency);
    let subtotal = |crypto: bool| {
        let instruments = account.instruments.iter().filter(move |i| i.crypto == crypto);
        let count = instruments.clone().count();
        let realised: f64 = instruments.clone().map(|i| i.realised).sum();
        let fees: f64 = instruments.map(|i| i.fees).sum();
        (count, realised, fees)
    };
    let subtotals: Vec<(&str, usize, f64, f64)> = [(true, "Summa krypto"), (false, "Summa övrigt")]
        .into_iter()
        .map(|(crypto, label)| {
            let (count, realised, fees) = subtotal(crypto);
            (label, count, realised, fees)
        })
        .filter(|(_, count, ..)| *count > 0)
        .collect();

    rsx! {
        h4 { class: "portfolio-subtitle", "Per värdepapper" }
        div { class: "portfolio-table-scroll",
            table { class: "report-table",
                thead {
                    tr {
                        th { "Värdepapper" }
                        th { class: "num", "Köp" }
                        th { class: "num", "Sälj" }
                        th { class: "num", "Köpt för" }
                        th { class: "num", "Sålt för" }
                        th { class: "num", "Avgifter" }
                        th { class: "num", "Realiserat" }
                    }
                }
                tbody {
                    for instrument in account.instruments.iter() {
                        tr { key: "{instrument.asset}",
                            td {
                                {instrument.name.clone()}
                                if instrument.crypto {
                                    span { class: "portfolio-crypto-badge", "krypto" }
                                }
                                if instrument.quantity.abs() > 1e-9 {
                                    span { class: "portfolio-asset-id", " innehav {format_quantity(instrument.quantity)}" }
                                }
                            }
                            td { class: "num", "{instrument.buys}" }
                            td { class: "num", "{instrument.sells}" }
                            td { class: "num", {money(instrument.bought)} }
                            td { class: "num", {money(instrument.sold)} }
                            td { class: "num", {money(instrument.fees)} }
                            td { class: "num {sign_class(instrument.realised)}", {money(instrument.realised)} }
                        }
                    }
                    for (label , _ , realised , fees) in subtotals {
                        tr { class: "portfolio-subtotal",
                            td { colspan: 5, {label} }
                            td { class: "num", {money(fees)} }
                            td { class: "num {sign_class(realised)}", {money(realised)} }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_swedish_numbers() {
        assert_eq!(format_number(1_234_567.891, 2), "1\u{a0}234\u{a0}567,89");
        assert_eq!(format_number(-0.001, 2), "0,00");
        assert_eq!(format_number(-12.5, 2), "-12,50");
        assert_eq!(format_quantity(0.387_563_57), "0,38756357");
        assert_eq!(format_quantity(2723.0), "2\u{a0}723");
    }

    #[test]
    fn parses_typed_prices() {
        assert_eq!(parse_decimal("75 703,19"), Some(75_703.19));
        assert_eq!(parse_decimal(""), None);
        assert_eq!(parse_decimal("abc"), None);
    }
    fn holding(asset: &str, quantity: f64, cost_basis: f64) -> Holding {
        Holding {
            asset: asset.to_string(),
            name: asset.to_string(),
            quantity,
            cost_basis,
            last_price: None,
            last_price_date: None,
            crypto: asset == "BTC",
        }
    }

    #[test]
    fn valuation_uses_typed_prices_and_falls_back_to_cost() {
        let holdings = [holding("BTC", 0.5, 10_000.0), holding("ETH", 2.0, 3_000.0)];
        let prices = HashMap::from([(price_key("EUR", "BTC"), "30 000".to_string())]);
        let valuation = Valuation::of(&holdings, "EUR", &prices);
        assert_eq!(valuation.market_value, 15_000.0 + 3_000.0);
        assert_eq!(valuation.unrealised(), 5_000.0);
        assert_eq!(valuation.missing_prices, 1);
        // A price for another currency does not apply.
        assert_eq!(Valuation::of(&holdings, "SEK", &prices).missing_prices, 2);
    }

    #[test]
    fn total_result_is_value_minus_what_was_put_in() {
        let summary = FlowSummary {
            deposited: 1_000.0,
            withdrawn: 200.0,
            transferred_in: 50.0,
            transferred_out: 150.0,
            ..FlowSummary::default()
        };
        // Net 700 put in; 100 cash + 900 holdings + 50 sent to a wallet.
        assert_eq!(total_result(&summary, 100.0, 900.0, 50.0), 350.0);
    }

    #[test]
    fn default_prices_take_the_most_recent_price_across_accounts() {
        let date = |d| chrono::NaiveDate::from_ymd_opt(2024, 1, d);
        let account = |price: f64, day: u32| AccountReport {
            name: String::new(),
            provider: api::portfolio::Provider::Avanza,
            source_file: String::new(),
            currency: "SEK".to_string(),
            first_date: None,
            last_date: None,
            summary: FlowSummary::default(),
            cash_balance: 0.0,
            holdings: vec![Holding { last_price: Some(price), last_price_date: date(day), ..holding("X", 1.0, 1.0) }],
            assets_moved_in: vec![],
            assets_moved_out: vec![],
            years: vec![],
            instruments: vec![],
            contributions: vec![],
            broker_check: None,
            warnings: vec![],
        };
        let report = PortfolioReport { accounts: vec![account(12.0, 5), account(10.0, 1)], ..PortfolioReport::default() };
        assert_eq!(default_prices(&report)[&price_key("SEK", "X")], "12.00");
    }

    fn empty_account(currency: &str) -> AccountReport {
        AccountReport {
            name: String::new(),
            provider: api::portfolio::Provider::Avanza,
            source_file: String::new(),
            currency: currency.to_string(),
            first_date: None,
            last_date: None,
            summary: FlowSummary::default(),
            cash_balance: 0.0,
            holdings: vec![],
            assets_moved_in: vec![],
            assets_moved_out: vec![],
            years: vec![],
            instruments: vec![],
            contributions: vec![],
            broker_check: None,
            warnings: vec![],
        }
    }

    #[test]
    fn split_separates_crypto_from_the_rest() {
        let instrument = |asset: &str, crypto: bool, realised: f64| api::portfolio::InstrumentSummary {
            asset: asset.to_string(),
            crypto,
            realised,
            fees: 1.0,
            ..Default::default()
        };
        let mut account = empty_account("SEK");
        account.instruments = vec![instrument("BTC", true, 100.0), instrument("X", false, -40.0)];
        account.holdings = vec![holding("BTC", 1.0, 50.0), holding("X", 1.0, 10.0)];
        let prices = HashMap::from([
            (price_key("SEK", "BTC"), "80".to_string()),
            (price_key("SEK", "X"), "5".to_string()),
        ]);
        let crypto = Split::of(&[&account], true, &prices);
        assert_eq!((crypto.realised, crypto.unrealised, crypto.fees), (100.0, 30.0, 1.0));
        assert_eq!(crypto.result(), 130.0);
        let other = Split::of(&[&account], false, &prices);
        assert_eq!(other.result(), -45.0);
    }

    #[test]
    fn annual_return_values_whats_left_at_the_end() {
        let date = |y| NaiveDate::from_ymd_opt(y, 1, 1).unwrap();
        // 100 in grows to 110, 50 out, the remaining 60 grows to 66: 10 %/year.
        let rate = annual_return(&[(date(2021), 100.0), (date(2022), -50.0)], date(2023), 66.0).unwrap();
        assert!((rate - 0.1).abs() < 1e-6, "{rate}");
    }

    #[test]
    fn emptied_accounts_are_valued_on_their_last_day() {
        let mut closed = empty_account("SEK");
        closed.last_date = NaiveDate::from_ymd_opt(2024, 5, 1);
        assert_eq!(valuation_date([&closed]), NaiveDate::from_ymd_opt(2024, 5, 1).unwrap());
        let mut open = empty_account("SEK");
        open.holdings = vec![holding("BTC", 1.0, 1.0)];
        assert_eq!(valuation_date([&closed, &open]), Utc::now().date_naive());
    }
}
