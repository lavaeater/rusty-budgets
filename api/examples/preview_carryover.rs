//! Previews what envelope carryover would produce on a real budget snapshot,
//! without writing anything. Usage:
//!   cargo run -p api --example `preview_carryover` -- <file.json> <YYYY-MM from> <YYYY-MM show>
use api::models::PeriodId;
use api::view_models::BudgetViewModel;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: preview_carryover <file.json> <from> <show>");
    let from: PeriodId = args.next().expect("missing <from>").parse().expect("bad <from>");
    let show: PeriodId = args.next().expect("missing <show>").parse().expect("bad <show>");

    let raw = std::fs::read_to_string(&path).expect("read snapshot");
    let mut budget: api::models::Budget = serde_json::from_str(&raw).expect("deserialize Budget");

    let without = BudgetViewModel::from_budget(&budget, show);
    budget.carryover_from = Some(from);
    let with = BudgetViewModel::from_budget(&budget, show);

    println!("carryover from {from}, showing {show}\n");
    println!("{:<26} {:>12} {:>12} {:>12} {:>12}", "Item", "Carried in", "Budgeted", "Remaining", "Available");
    println!("{}", "-".repeat(80));
    let mut rows: Vec<_> = with
        .items
        .iter()
        .filter(|i| !i.budgeted_amount.is_zero() || !i.carried_over.is_zero())
        .collect();
    rows.sort_by_key(|i| i.available.amount_in_cents());
    for i in &rows {
        println!(
            "{:<26} {:>12} {:>12} {:>12} {:>12}",
            i.name.chars().take(26).collect::<String>(),
            i.carried_over.to_string(),
            i.budgeted_amount.to_string(),
            i.remaining_budget.to_string(),
            i.available.to_string(),
        );
    }
    let neg = rows.iter().filter(|i| i.available.amount_in_cents() < 0).count();
    println!("\n{} items shown | {neg} would be carrying a negative balance", rows.len());
    println!("(without carryover, every 'Available' would equal 'Remaining': {})",
        without.items.iter().all(|i| i.available == i.remaining_budget));
}
