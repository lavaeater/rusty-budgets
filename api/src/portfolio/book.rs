//! The bookkeeping engine shared by every broker parser.
//!
//! Parsers translate their rows into calls on [`Book`], which keeps a cash
//! balance and one position per asset using the average-cost method
//! (genomsnittsmetoden): buying adds to the position's total cost, selling
//! removes cost proportionally and the difference to the proceeds is the
//! realised result. All amounts are in the account's own currency.

use super::{
    AccountReport, AssetMovement, BrokerCheck, DatedAmount, FlowSummary, Holding, InstrumentSummary, Provider,
    YearSummary,
};
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

/// Decides from an asset key and its name whether it is crypto.
pub type CryptoClassifier = fn(asset: &str, name: &str) -> bool;

#[derive(Debug, Default)]
struct InstrumentStats {
    bought: f64,
    sold: f64,
    realised: f64,
    fees: f64,
    dividends: f64,
    buys: u32,
    sells: u32,
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
    /// Keyed by asset and whether it left at market value, so a crypto
    /// withdrawal at cost never merges with a disposal of the same asset.
    moved_out: BTreeMap<(String, bool), AssetMovement>,
    broker_check: Option<BrokerCheck>,
    warnings: Vec<String>,
    instruments: BTreeMap<String, InstrumentStats>,
    /// Owner's net contribution per day; see [`DatedAmount`].
    contributions: BTreeMap<NaiveDate, f64>,
    is_crypto: CryptoClassifier,
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
            instruments: BTreeMap::new(),
            contributions: BTreeMap::new(),
            is_crypto: |_, _| false,
        }
    }

    /// Sets how holdings and instruments are classified as crypto.
    #[must_use]
    pub fn crypto_when(mut self, is_crypto: CryptoClassifier) -> Self {
        self.is_crypto = is_crypto;
        self
    }

    fn instrument(&mut self, asset: &str) -> &mut InstrumentStats {
        self.instruments.entry(asset.to_string()).or_default()
    }

    fn contribute(&mut self, date: NaiveDate, amount: f64) {
        *self.contributions.entry(date).or_default() += amount;
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
        self.contribute(date, amount);
        self.record(date, |s| {
            s.deposited += amount;
            s.fees += fee;
        });
    }

    /// Money paid out to a bank account; `amount` is positive and `fee` is
    /// charged on top.
    pub fn withdraw(&mut self, date: NaiveDate, amount: f64, fee: f64) {
        self.cash -= amount + fee;
        self.contribute(date, -amount);
        self.record(date, |s| {
            s.withdrawn += amount;
            s.fees += fee;
        });
    }

    /// Cash moved to (negative) or from (positive) another own account.
    pub fn transfer_cash(&mut self, date: NaiveDate, amount: f64) {
        self.cash += amount;
        self.contribute(date, amount);
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
        let stats = self.instrument(asset);
        stats.bought += cost;
        stats.fees += fee;
        stats.buys += 1;
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
        let stats = self.instrument(asset);
        stats.sold += proceeds;
        stats.realised += proceeds - basis;
        stats.fees += fee;
        stats.sells += 1;
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
        self.instrument(from).sells += 1;
        self.instrument(to).buys += 1;
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
        self.contribute(date, value);
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
        let key = (asset.to_string(), market_value.is_some());
        let moved = self.moved_out.entry(key).or_insert_with(|| AssetMovement {
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
        // Whatever leaves goes back to the owner: at market value when the
        // export states one, otherwise at cost.
        self.contribute(date, -market_value.unwrap_or(basis));
        let realised = market_value.map_or(0.0, |value| value - basis) - fee_basis;
        let stats = self.instrument(asset);
        stats.realised += realised;
        stats.fees += fee_basis;
        if market_value.is_some() {
            stats.sells += 1;
        }
        self.record(date, |s| {
            s.fees += fee_basis;
            s.realised -= fee_basis;
            if let Some(value) = market_value {
                s.realised += value - basis;
                s.transferred_out += value;
            }
        });
        realised
    }

    /// Interest received (positive) or paid (negative).
    pub fn interest(&mut self, date: NaiveDate, amount: f64) {
        self.cash += amount;
        self.record(date, |s| s.interest += amount);
    }

    pub fn dividend(&mut self, date: NaiveDate, asset: Option<&str>, amount: f64) {
        self.cash += amount;
        if let Some(asset) = asset {
            self.instrument(asset).dividends += amount;
        }
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
        let is_crypto = self.is_crypto;
        let display_name = |asset: &str, positions: &BTreeMap<String, Position>| {
            positions
                .get(asset)
                .map(|p| p.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| asset.to_string())
        };
        let mut instruments: Vec<InstrumentSummary> = self
            .instruments
            .iter()
            .map(|(asset, stats)| {
                let name = display_name(asset, &self.positions);
                InstrumentSummary {
                    crypto: is_crypto(asset, &name),
                    asset: asset.clone(),
                    name,
                    bought: stats.bought,
                    sold: stats.sold,
                    realised: stats.realised,
                    fees: stats.fees,
                    dividends: stats.dividends,
                    buys: stats.buys,
                    sells: stats.sells,
                    quantity: self.positions.get(asset).map_or(0.0, |p| p.quantity),
                }
            })
            .collect();
        instruments.sort_by(|a, b| b.realised.total_cmp(&a.realised));
        let holdings = self
            .positions
            .into_iter()
            .filter(|(_, p)| p.quantity.abs() > QTY_EPSILON)
            .map(|(asset, p)| {
                let name = if p.name.is_empty() { asset.clone() } else { p.name };
                Holding {
                    crypto: is_crypto(&asset, &name),
                    name,
                    asset,
                    quantity: p.quantity,
                    cost_basis: p.cost,
                    last_price: p.last_price,
                    last_price_date: p.last_price_date,
                }
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
            instruments,
            contributions: self
                .contributions
                .into_iter()
                .filter(|(_, amount)| amount.abs() > f64::EPSILON)
                .map(|(date, amount)| DatedAmount { date, amount })
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
    #[test]
    fn selling_everything_leaves_no_residual_cost() {
        let mut book = Book::new("SEK");
        book.buy(day(1), "X", "X", 3.0, 100.0, 0.0);
        book.sell(day(2), "X", "X", 1.0, 40.0, 0.0);
        book.sell(day(3), "X", "X", 1.0, 40.0, 0.0);
        book.sell(day(4), "X", "X", 1.0, 40.0, 0.0);
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        assert!(report.holdings.is_empty(), "{:?}", report.holdings);
        // 120 in, 100 out: every cent of cost is accounted for.
        assert!(close(report.summary.realised, 20.0));
    }

    #[test]
    fn fees_are_part_of_cost_and_proceeds() {
        let mut book = Book::new("SEK");
        // 10 units at 10 plus 5 in courtage, sold at 15 minus 5 in courtage.
        book.buy(day(1), "X", "X", 10.0, 105.0, 5.0);
        let realised = book.sell(day(2), "X", "X", 10.0, 145.0, 5.0);
        assert!(close(realised, 40.0));
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        assert!(close(report.summary.fees, 10.0));
        assert!(close(report.summary.bought, 105.0));
        assert!(close(report.summary.sold, 145.0));
    }

    #[test]
    fn last_price_excludes_fees() {
        let mut book = Book::new("SEK");
        book.buy(day(1), "X", "X", 10.0, 105.0, 5.0);
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        assert_eq!(report.holdings[0].last_price, Some(10.0));
        assert_eq!(report.holdings[0].last_price_date, Some(day(1)));
    }

    #[test]
    fn transferred_securities_keep_their_market_value_as_basis() {
        let mut book = Book::new("SEK");
        book.receive_transfer(day(1), "X", "X", 10.0, 200.0);
        book.sell(day(2), "X", "X", 10.0, 250.0, 0.0);
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        assert!(close(report.summary.realised, 50.0));
        assert!(close(report.summary.transferred_in, 200.0));
        assert_eq!(report.assets_moved_in[0].value, Some(200.0));
    }

    #[test]
    fn withdrawal_at_cost_and_disposal_of_same_asset_are_kept_apart() {
        let mut book = Book::new("SEK");
        book.buy(day(1), "X", "X", 2.0, 100.0, 0.0);
        book.send_out(day(2), "X", "X", 1.0, 0.0, None);
        book.send_out(day(3), "X", "X", 1.0, 0.0, Some(80.0));
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        assert_eq!(report.assets_moved_out.len(), 2);
        assert!(close(report.moved_out_at_cost(), 50.0));
        assert!(close(report.summary.realised, 30.0));
    }

    #[test]
    fn years_sum_to_the_total() {
        let mut book = Book::new("SEK");
        let date = |y| NaiveDate::from_ymd_opt(y, 6, 1).unwrap();
        book.deposit(date(2021), 500.0, 0.0);
        book.buy(date(2021), "X", "X", 5.0, 500.0, 1.0);
        book.sell(date(2022), "X", "X", 2.0, 300.0, 1.0);
        book.interest(date(2023), 3.0);
        book.withdraw(date(2023), 100.0, 0.0);
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        assert_eq!(report.years.iter().map(|y| y.year).collect::<Vec<_>>(), [2021, 2022, 2023]);
        let mut sum = FlowSummary::default();
        for year in &report.years {
            sum += year.summary;
        }
        assert_eq!(sum, report.summary);
        assert_eq!(report.first_date, Some(date(2021)));
        assert_eq!(report.last_date, Some(date(2023)));
    }

    /// Everything the account holds now (at cost), plus what was sent out
    /// at cost, minus everything put in, must equal the sum of the result
    /// components. Checked after every step of a long pseudo-random mix of
    /// operations, so any operation that leaks or invents money fails it.
    #[test]
    fn accounting_identity_holds_for_random_operations() {
        // Small deterministic LCG, so failures are reproducible.
        let mut seed: u64 = 0x5eed;
        let mut next = move |max: u64| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % max
        };
        #[allow(clippy::cast_precision_loss)]
        let amount = |max: u64, next: &mut dyn FnMut(u64) -> u64| next(max * 100) as f64 / 100.0 + 0.01;

        let mut book = Book::new("SEK");
        let assets = ["A", "B", "C"];
        for step in 0..2000 {
            let date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap() + chrono::Days::new(step);
            let asset = assets[usize::try_from(next(3)).unwrap()];
            let other = assets[usize::try_from(next(3)).unwrap()];
            let held = book.quantity(asset).max(0.0);
            match next(12) {
                0 => book.deposit(date, amount(1000, &mut next), 0.0),
                1 => book.withdraw(date, amount(300, &mut next), 0.0),
                2 => book.transfer_cash(date, amount(400, &mut next) - 200.0),
                3 | 4 => {
                    let fee = amount(5, &mut next);
                    book.buy(date, asset, asset, amount(10, &mut next), amount(500, &mut next) + fee, fee);
                }
                5 | 6 => {
                    // Mostly within the holding, sometimes all of it.
                    let quantity = if next(4) == 0 { held } else { held * amount(1, &mut next).min(1.0) };
                    book.sell(date, asset, asset, quantity, amount(600, &mut next), amount(5, &mut next));
                }
                7 if asset != other => book.swap(date, asset, held / 2.0, other, amount(10, &mut next)),
                8 => book.receive_transfer(date, asset, asset, amount(5, &mut next), amount(300, &mut next)),
                9 => {
                    let market = (next(2) == 0).then(|| amount(300, &mut next));
                    book.send_out(date, asset, asset, held / 3.0, held / 10.0, market);
                }
                10 => book.interest(date, amount(20, &mut next) - 10.0),
                _ => book.tax(date, -amount(10, &mut next)),
            }
        }
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        let s = report.summary;
        let left = report.cash_balance + report.holdings_cost_basis() + report.moved_out_at_cost()
            - s.net_deposited()
            - s.net_transferred();
        let right = s.realised + s.interest + s.dividends + s.other - s.tax;
        assert!((left - right).abs() < 1e-6, "left {left} right {right}");

        // The per-instrument figures split the totals without remainder.
        let instruments = |f: fn(&InstrumentSummary) -> f64| report.instruments.iter().map(f).sum::<f64>();
        assert!((instruments(|i| i.realised) - s.realised).abs() < 1e-6);
        assert!((instruments(|i| i.fees) - s.fees).abs() < 1e-6);
        assert!((instruments(|i| i.bought) - s.bought).abs() < 1e-6);
        assert!((instruments(|i| i.sold) - s.sold).abs() < 1e-6);
        // The dated contributions add up to what was put in, net.
        let contributed: f64 = report.contributions.iter().map(|c| c.amount).sum();
        let expected = s.net_deposited() + s.net_transferred() - report.moved_out_at_cost();
        assert!((contributed - expected).abs() < 1e-6, "{contributed} vs {expected}");
        assert!(report.contributions.windows(2).all(|w| w[0].date < w[1].date));
        // Sales never exceed holdings here, so no basis was guessed.
        assert!(!report.warnings.iter().any(|w| w.starts_with("Mer ")), "{:?}", report.warnings);
    }

    #[test]
    fn instruments_are_tracked_and_classified() {
        let mut book = Book::new("SEK").crypto_when(|asset, _| asset == "BTC");
        book.buy(day(1), "BTC", "Bitcoin", 1.0, 100.0, 1.0);
        book.buy(day(1), "X", "Fond", 1.0, 50.0, 0.0);
        book.sell(day(2), "BTC", "Bitcoin", 0.5, 80.0, 1.0);
        book.sell(day(2), "X", "Fond", 1.0, 40.0, 0.0);
        book.dividend(day(3), Some("X"), 2.0);
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        let names: Vec<_> = report.instruments.iter().map(|i| i.asset.as_str()).collect();
        // Largest realised result first.
        assert_eq!(names, ["BTC", "X"]);
        let btc = &report.instruments[0];
        assert!(btc.crypto && btc.buys == 1 && btc.sells == 1);
        assert!(close(btc.realised, 30.0) && close(btc.fees, 2.0) && close(btc.quantity, 0.5));
        let fund = &report.instruments[1];
        assert!(!fund.crypto && close(fund.realised, -10.0) && close(fund.dividends, 2.0));
        assert!(report.holdings[0].crypto);
    }

    #[test]
    fn contributions_are_netted_per_day() {
        let mut book = Book::new("SEK");
        book.deposit(day(1), 100.0, 0.0);
        book.deposit(day(1), 50.0, 0.0);
        book.withdraw(day(2), 30.0, 0.0);
        book.transfer_cash(day(3), 20.0);
        book.transfer_cash(day(3), -20.0);
        let report = book.finish("a".into(), Provider::Avanza, "f".into());
        let flows: Vec<_> = report.contributions.iter().map(|c| (c.date, c.amount)).collect();
        assert_eq!(flows, [(day(1), 150.0), (day(2), -30.0)]);
    }
}
