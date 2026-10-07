//! Annualised money-weighted return (XIRR), as in spreadsheet `XIRR()`.

use chrono::NaiveDate;

/// The yearly rate `r` at which the dated cash flows have a net present
/// value of zero, e.g. `0.12` for 12 %/year. Use the investor's view: money
/// put in is negative, money taken out (and the final value) is positive.
///
/// Returns `None` when no rate exists: fewer than two flows, all flows of
/// the same sign, or no root between −99.99 % and +1 000 000 %/year.
pub fn xirr(flows: &[(NaiveDate, f64)]) -> Option<f64> {
    let has_in = flows.iter().any(|(_, a)| *a < 0.0);
    let has_out = flows.iter().any(|(_, a)| *a > 0.0);
    if !has_in || !has_out {
        return None;
    }
    let start = flows.iter().map(|(d, _)| *d).min()?;
    let years: Vec<(f64, f64)> = flows
        .iter()
        .map(|(d, a)| {
            #[allow(clippy::cast_precision_loss)]
            let t = (*d - start).num_days() as f64 / 365.0;
            (t, *a)
        })
        .collect();
    let npv = |rate: f64| -> f64 { years.iter().map(|(t, a)| a / (1.0 + rate).powf(*t)).sum() };

    // NPV falls as the rate rises (for a typical invest-then-withdraw
    // pattern), but not necessarily monotonically for arbitrary flows, so
    // scan for a sign change first and then bisect, which always converges.
    let mut low = -0.9999;
    let mut f_low = npv(low);
    let mut high = low;
    let mut found = false;
    for step in [-0.99, -0.9, -0.5, -0.2, 0.0, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 100.0, 1000.0, 10_000.0] {
        let f_step = npv(step);
        if f_low.signum() != f_step.signum() {
            high = step;
            found = true;
            break;
        }
        low = step;
        f_low = f_step;
    }
    if !found {
        return None;
    }
    for _ in 0..200 {
        let mid = f64::midpoint(low, high);
        let f_mid = npv(mid);
        if f_mid.abs() < 1e-9 || (high - low) < 1e-12 {
            return Some(mid);
        }
        if f_low.signum() == f_mid.signum() {
            low = mid;
            f_low = f_mid;
        } else {
            high = mid;
        }
    }
    Some(f64::midpoint(low, high))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn doubling_in_one_year_is_one_hundred_percent() {
        let rate = xirr(&[(date(2021, 1, 1), -100.0), (date(2022, 1, 1), 200.0)]).unwrap();
        assert!(close(rate, 1.0), "{rate}");
    }

    #[test]
    fn halving_over_two_years() {
        let rate = xirr(&[(date(2020, 1, 1), -100.0), (date(2021, 12, 31), 50.0)]).unwrap();
        // 0.5^(365/730) − 1
        assert!(close(rate, 0.5_f64.powf(1.0 / 2.0) - 1.0), "{rate}");
    }

    #[test]
    fn matches_the_spreadsheet_reference_example() {
        // The example from the LibreOffice/Excel XIRR documentation.
        let flows = [
            (date(2008, 1, 1), -10_000.0),
            (date(2008, 3, 1), 2_750.0),
            (date(2008, 10, 30), 4_250.0),
            (date(2009, 2, 15), 3_250.0),
            (date(2009, 4, 1), 2_750.0),
        ];
        let rate = xirr(&flows).unwrap();
        assert!((rate - 0.373_362_535).abs() < 1e-6, "{rate}");
    }

    #[test]
    fn several_deposits_and_a_final_value() {
        // 100 in at the start and 100 after a year, 231 back after two:
        // exactly 10 %/year on both deposits.
        let rate = xirr(&[
            (date(2020, 1, 1), -100.0),
            (date(2020, 12, 31), -100.0),
            (date(2021, 12, 31), 231.0),
        ])
        .unwrap();
        assert!((rate - 0.1).abs() < 1e-3, "{rate}");
    }

    #[test]
    fn no_rate_without_flows_in_both_directions() {
        assert_eq!(xirr(&[]), None);
        assert_eq!(xirr(&[(date(2020, 1, 1), -100.0)]), None);
        assert_eq!(xirr(&[(date(2020, 1, 1), -100.0), (date(2021, 1, 1), -5.0)]), None);
    }

    #[test]
    fn total_loss_has_no_finite_rate_but_near_total_loss_does() {
        assert_eq!(xirr(&[(date(2020, 1, 1), -100.0), (date(2021, 1, 1), 0.0)]), None);
        let rate = xirr(&[(date(2021, 1, 1), -100.0), (date(2022, 1, 1), 1.0)]).unwrap();
        assert!(close(rate, -0.99), "{rate}");
    }
}
