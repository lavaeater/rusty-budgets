//! Deserialises a real `Budget` snapshot to prove schema changes stay
//! backward-compatible, and reports the classification/suggestion split.
//! Usage: cargo run -p api --example `verify_snapshot` -- <file.json>
use std::collections::HashMap;

fn main() {
    let path = std::env::args().nth(1).expect("usage: verify_snapshot <file.json>");
    let raw = std::fs::read_to_string(&path).expect("read snapshot");
    let budget: api::models::Budget = serde_json::from_str(&raw).expect("deserialize Budget");
    println!("ok: {} tags, {} periods, version {}", budget.tags.len(), budget.periods.len(), budget.version);

    let live: Vec<_> = budget.tags.iter().filter(|t| !t.deleted).collect();
    let auto = live.iter().filter(|t| t.is_automatic()).count();
    let review = live.iter().filter(|t| t.needs_review).count();
    println!("live tags: {} | automatic: {} | needing review: {}", live.len(), auto, review);

    let untagged = budget
        .periods
        .iter()
        .flat_map(|p| p.transactions.iter())
        .filter(|t| t.tag_id.is_none() && !t.ignored)
        .count();
    let applied = budget.evaluate_tag_rules().len();
    let suggested = budget.suggest_tag_rules().len();
    println!("untagged: {untagged} | auto-applies: {applied} | suggestions: {suggested}");

    let names: HashMap<_, _> = budget.tags.iter().map(|t| (t.id, t.name.as_str())).collect();
    let mut by_tag: HashMap<&str, usize> = HashMap::new();
    for (_, tag_id) in budget.suggest_tag_rules() {
        *by_tag.entry(names.get(&tag_id).copied().unwrap_or("?")).or_default() += 1;
    }
    let mut groups: Vec<_> = by_tag.into_iter().collect();
    groups.sort_by_key(|g| std::cmp::Reverse(g.1));
    for (name, n) in groups.iter().take(10) {
        println!("  suggestion group: {n:>4} x {name}");
    }
}
