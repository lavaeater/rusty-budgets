//! The bookkeeping engine shared by every broker parser.
//!
//! Parsers translate their rows into calls on [`Book`], which keeps a cash
//! balance and one position per asset using the average-cost method
//! (genomsnittsmetoden): buying adds to the position's total cost, selling
//! removes cost proportionally and the difference to the proceeds is the
//! realised result. All amounts are in the account's own currency.

use super::{AccountReport, AssetMovement, BrokerCheck, FlowSummary, Holding, Provider, YearSummary};
use chrono::{Datelike, NaiveDate};
use std::collections::BTreeMap;

/// Quantities below this are treated as zero (rounding noise in exports).
const QTY_EPSILON: f64 = 1e-9;

#[derive(Debug, Default)]
struct Position {
    name: String,
    quantity: f64,
    cost: f64,
    last_price: Option<f64>,
    last_price_date: Option<NaiveDate>,
}

#[derive(Debug)]
pub struct Book {
    currency: String,
    cash: f64,
    positions: BTreeMap<String, Position>,
    total: FlowSummary,
    years: BTreeMap<i32, FlowSummary>,
    first_date: Option<NaiveDate>,
    last_date: Option<NaiveDate>,
    moved_in: BTreeMap<String, AssetMovement>,
    moved_out: BTreeMap<String, AssetMovement>,
    broker_check: Option<BrokerCheck>,
    warnings: Vec<String>,
}

impl Book {
    pub fn new(currency: &str) -> Self {
        Self {
            currency: currency.to_string(),
            cash: 0.0,
            positions: BTreeMap::new(),
            total: FlowSummary::default(),
            years: BTreeMap::new(),
            first_date: None,
            last_date: None,
            moved_in: BTreeMap::new(),
            moved_out: BTreeMap::new(),
            broker_check: None,
            warnings: Vec::new(),
        }
    }

    pub fn cash(&self) -> f64 {
        self.cash
    }

    pub fn quantity(&self, asset: &str) -> f64 {
        self.positions.get(asset).map_or(0.0, |p| p.quantity)
    }

    /// Adds a warning once, however many rows trigger it.
    pub fn warn(&mut self, message: impl Into<String>) {
        let message = message.into();
        if !self.warnings.contains(&message) {
            self.warnings.push(message);
        }
    }

    /// Records the result the broker itself reports for a row, next to
    /// ours for the same row, for cross-checking.
    pub fn add_broker_result(&mut self, broker: f64, ours: f64) {
        let check = self.broker_check.get_or_insert_with(BrokerCheck::default);
        check.broker_realised += broker;
        check.our_realised += ours;
        check.rows += 1;
    }

    /// Applies `update` to both the all-time summary and `date`'s year.
    fn record(&mut self, date: NaiveDate, update: impl Fn(&mut FlowSummary)) {
        self.first_date = Some(self.first_date.map_or(date, |d| d.min(date)));
        self.last_date = Some(self.last_date.map_or(date, |d| d.max(date)));
        update(&mut self.total);
        update(self.years.entry(date.year()).or_default());
    }

    fn position(&mut self, asset: &str, name: &str) -> &mut Position {
        let position = self.positions.entry(asset.to_string()).or_default();
        // Rows that move securities often carry a generic description, so
        // parsers pass the asset key instead; any real name wins over that.
        if position.name.is_empty() || (position.name == asset && name != asset) {
            position.name = name.to_string();
        }
        position
    }

    fn set_price(&mut self, asset: &str, name: &str, date: NaiveDate, price: f64) {
        if price.is_finite() && price > 0.0 {
            let position = self.position(asset, name);
            position.last_price = Some(price);
            position.last_price_date = Some(date);
        }
    }

    /// Removes `quantity` of `asset` and returns the cost basis it carried.
    fn take(&mut self, asset: &str, quantity: f64) -> f64 {
        let position = self.positions.entry(asset.to_string()).or_default();
        let held = position.quantity;
        let basis = if held > QTY_EPSILON {
            position.cost * (quantity.min(held) / held)
        } else {
            0.0
        };
        position.quantity -= quantity;
        position.cost -= basis;
        if position.quantity.abs() < QTY_EPSILON {
            position.quantity = 0.0;
            position.cost = 0.0;
        }
        if quantity > held + QTY_EPSILON {
            self.warn(format!(
                "Mer {asset} såldes/flyttades än vad som fanns i filen — \
                 anskaffningsvärdet för överskottet är okänt och räknas som 0."
            ));
        }
        basis
    }

    /// Money paid in from outside (a bank account). `fee` is charged on top.
    pub fn deposit(&mut self, date: NaiveDate, amount: f64, fee: f64) {
        self.cash += amount - fee;
        self.record(date, |s| {
            s.deposited += amount;
            s.fees += fee;
        });
    }

    /// Money paid out to a bank account; `amount` is positive and `fee` is
    /// charged on top.
    pub fn withdraw(&mut self, date: NaiveDate, amount: f64, fee: f64) {
        self.cash -= amount + fee;
        self.record(date, |s| {
            s.withdrawn += amount;
            s.fees += fee;
        });
    }

    /// Cash moved to (negative) or from (positive) another own account.
    pub fn transfer_cash(&mut self, date: NaiveDate, amount: f64) {
        self.cash += amount;
        self.record(date, |s| {
            if amount >= 0.0 {
                s.transferred_in += amount;
            } else {
                s.transferred_out -= amount;
            }
        });
    }

    /// Buys `quantity` for `cost` in total, of which `fee` is the fee part.
    pub fn buy(&mut self, date: NaiveDate, asset: &str, name: &str, quantity: f64, cost: f64, fee: f64) {
        self.cash -= cost;
        let position = self.position(asset, name);
        position.quantity += quantity;
        position.cost += cost;
        if quantity > QTY_EPSILON {
            self.set_price(asset, name, date, (cost - fee) / quantity);
        }
        self.record(date, |s| {
            s.bought += cost;
            s.fees += fee;
        });
    }

    /// Sells `quantity` for `proceeds` after the fee `fee` has been deducted.
    pub fn sell(&mut self, date: NaiveDate, asset: &str, name: &str, quantity: f64, proceeds: f64, fee: f64) -> f64 {
        let basis = self.take(asset, quantity);
        self.cash += proceeds;
        if quantity > QTY_EPSILON {
            self.set_price(asset, name, date, (proceeds + fee) / quantity);
        }
        self.record(date, |s| {
            s.sold += proceeds;
            s.realised += proceeds - basis;
            s.fees += fee;
        });
        proceeds - basis
    }

    /// Exchanges one asset for another without going through the account
    /// currency. The acquired asset inherits the disposed asset's cost
    /// basis, so no result is realised until it is eventually sold.
    pub fn swap(&mut self, date: NaiveDate, from: &str, from_quantity: f64, to: &str, to_quantity: f64) {
        let basis = self.take(from, from_quantity);
        let position = self.position(to, to);
        position.quantity += to_quantity;
        position.cost += basis;
        self.record(date, |_| {});
        self.warn(
            "Byten mellan två kryptovalutor flyttar över anskaffningsvärdet till den nya \
             tillgången — resultatet realiseras först när den säljs mot pengar.",
        );
    }

    /// Asset received for free (airdrop, fork, unexplained opening balance).
    pub fn receive_free(&mut self, date: NaiveDate, asset: &str, name: &str, quantity: f64) {
        self.position(asset, name).quantity += quantity;
        self.record(date, |_| {});
    }

    /// Securities moved in from another account, valued at `value`, which
    /// becomes their cost basis.
    pub fn receive_transfer(&mut self, date: NaiveDate, asset: &str, name: &str, quantity: f64, value: f64) {
        let position = self.position(asset, name);
        position.quantity += quantity;
        position.cost += value;
        if quantity > QTY_EPSILON {
            self.set_price(asset, name, date, value / quantity);
        }
        let moved = self.moved_in.entry(asset.to_string()).or_insert_with(|| AssetMovement {
            asset: asset.to_string(),
            name: name.to_string(),
            ..AssetMovement::default()
        });
        moved.quantity += quantity;
        moved.cost_basis += value;
        *moved.value.get_or_insert(0.0) += value;
        self.record(date, |s| s.transferred_in += value);
    }

    /// An asset leaves the account. With `market_value` it is a disposal
    /// at that value (Swedish rules treat moving securities into an ISK as
    /// a sale); without it, it is sent out at cost (e.g. crypto withdrawn to
    /// an own wallet). `fee_quantity` is consumed on top as a fee.
    pub fn send_out(
        &mut self,
        date: NaiveDate,
        asset: &str,
        name: &str,
        quantity: f64,
        fee_quantity: f64,
        market_value: Option<f64>,
    ) -> f64 {
        let basis = self.take(asset, quantity);
        let fee_basis = self.take(asset, fee_quantity);
        let moved = self.moved_out.entry(asset.to_string()).or_insert_with(|| AssetMovement {
            asset: asset.to_string(),
            name: name.to_string(),
            ..AssetMovement::default()
        });
        moved.quantity += quantity;
        moved.cost_basis += basis;
        if let Some(value) = market_value {
            *moved.value.get_or_insert(0.0) += value;
            if quantity > QTY_EPSILON {
                self.set_price(asset, name, date, value / quantity);
            }
        }
        self.record(date, |s| {
            s.fees += fee_basis;
            s.realised -= fee_basis;
            if let Some(value) = market_value {
                s.realised += value - basis;
                s.transferred_out += value;
            }
        });
        market_value.map_or(0.0, |value| value - basis) - fee_basis
    }

    /// Interest received (positive) or paid (negative).
    pub fn interest(&mut self, date: NaiveDate, amount: f64) {
        self.cash += amount;
        self.record(date, |s| s.interest += amount);
    }

    pub fn dividend(&mut self, date: NaiveDate, amount: f64) {
        self.cash += amount;
        self.record(date, |s| s.dividends += amount);
    }

    /// Tax drawn from (negative) or refunded to (positive) the account.
    pub fn tax(&mut self, date: NaiveDate, amount: f64) {
        self.cash += amount;
        self.record(date, |s| s.tax -= amount);
    }

    /// Any other cash movement the parser could not classify.
    pub fn other(&mut self, date: NaiveDate, amount: f64) {
        self.cash += amount;
        self.record(date, |s| s.other += amount);
    }

    pub fn finish(mut self, name: String, provider: Provider, source_file: String) -> AccountReport {
        for moved in self.moved_in.values_mut().chain(self.moved_out.values_mut()) {
            if let Some(position) = self.positions.get(&moved.asset) {
                moved.name.clone_from(&position.name);
            }
        }
        let holdings = self
            .positions
            .into_iter()
            .filter(|(_, p)| p.quantity.abs() > QTY_EPSILON)
            .map(|(asset, p)| Holding {
                name: if p.name.is_empty() { asset.clone() } else { p.name },
                asset,
                quantity: p.quantity,
                cost_basis: p.cost,
                last_price: p.last_price,
                last_price_date: p.last_price_date,
            })
            .collect();
        AccountReport {
            name,
            provider,
            source_file,
            currency: self.currency,
            first_date: self.first_date,
            last_date: self.last_date,
            summary: self.total,
            cash_balance: self.cash,
            holdings,
            assets_moved_in: self.moved_in.into_values().collect(),
            assets_moved_out: self.moved_out.into_values().collect(),
            years: self
                .years
                .into_iter()
                .map(|(year, summary)| YearSummary { year, summary })
                .collect(),
            broker_check: self.broker_check,
            warnings: self.warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 1, d).unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn average_cost_realises_the_difference_on_sale() {
        let mut book = Book::new("SEK");
        book.deposit(day(1), 1000.0, 0.0);
        book.buy(day(2), "X", "X", 10.0, 100.0, 0.0);
        book.buy(day(3), "X", "X", 10.0, 300.0, 0.0);
        // Average cost is now 20/unit; selling 5 for 150 realises 50.
        book.sell(day(4), "X", "X", 5.0, 150.0, 0.0);
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        assert!(close(report.summary.realised, 50.0));
        assert!(close(report.holdings[0].quantity, 15.0));
        assert!(close(report.holdings[0].cost_basis, 300.0));
        assert!(close(report.cash_balance, 750.0));
    }

    #[test]
    fn swap_carries_basis_and_send_out_removes_it_at_cost() {
        let mut book = Book::new("EUR");
        book.buy(day(1), "BTC", "BTC", 1.0, 1000.0, 0.0);
        book.swap(day(2), "BTC", 0.5, "ETH", 10.0);
        book.send_out(day(3), "BTC", "BTC", 0.25, 0.05, None);
        let report = book.finish("a".into(), Provider::Kraken, "f".into());
        let eth = report.holdings.iter().find(|h| h.asset == "ETH").unwrap();
        assert!(close(eth.cost_basis, 500.0));
        let btc = report.holdings.iter().find(|h| h.asset == "BTC").unwrap();
        assert!(close(btc.quantity, 0.2));
        assert!(close(btc.cost_basis, 200.0));
        assert!(close(report.assets_moved_out[0].cost_basis, 250.0));
        // The fee's share of the basis is lost.
        assert!(close(report.summary.realised, -50.0));
    }

    #[test]
    fn transfer_out_at_market_value_realises_result() {
        let mut book = Book::new("SEK");
        book.buy(day(1), "X", "X", 10.0, 100.0, 0.0);
        book.send_out(day(2), "X", "X", 10.0, 0.0, Some(250.0));
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        assert!(close(report.summary.realised, 150.0));
        assert!(close(report.summary.transferred_out, 250.0));
        assert!(report.holdings.is_empty(), "{:?}", report.holdings);
    }

    #[test]
    fn overselling_is_reported() {
        let mut book = Book::new("SEK");
        book.sell(day(1), "X", "X", 1.0, 10.0, 0.0);
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        assert_eq!(report.warnings.len(), 1);
        assert!(close(report.summary.realised, 10.0));
    }
}
