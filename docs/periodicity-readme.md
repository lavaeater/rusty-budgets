# Tag periodicity/cost_kind — findings and recommendations

> Written 2026-08-21 after a user-reported concern: tagging a mixed-cadence
> category (e.g. "Parking" — sometimes a predictable monthly garage fee,
> sometimes an ad hoc cash payment) with a single `Recurring` periodicity
> distorts the budgeting math. This doc captures what the code actually does,
> confirms the concern is real, and lays out the practical fix — no code
> changes included, this is a reference for manually reclassifying tags.

## The core problem

`Tag.cost_kind` (`api/src/models/tag.rs`) is one of:

- `Recurring(Periodicity)` — a fixed-schedule bill (rent, an annual premium).
  Drives buffering/periodisation so the monthly budget doesn't spike the
  month the bill is due.
- `Variable` — ad hoc spending, budgeted as a plain monthly average, never
  buffered.

This is **one classification per tag**, applied uniformly to every
transaction tagged with it. There is no per-transaction or per-`ActualItem`
cadence field anywhere in the domain model — `BankTransaction` and
`ActualItem` were both checked and carry no such field. Cadence lives
exclusively on `Tag`. So there is no way to mark "this one instance was
atypical" short of using a different tag.

## Where it actually bites

- `Budget::periodised_monthly()` (`api/src/models/budget.rs:452-480`) reads
  `tag.cost_kind.cycle_months()`. For `Variable` (or a 1-month cycle) it's a
  whole-history monthly average. For a multi-month cycle (Quarterly/Annual)
  it sums the **trailing cycle window** of that tag's transactions and
  divides by the cycle length.
- `TagSummary::buffer_target()` (`api/src/view_models/tag_summary.rs:36-39`)
  and `required_monthly_contribution`
  (`api/src/view_models/budget_item_view_model.rs:75-84`) build the monthly
  buffer contribution off that same number.

A one-off transaction tagged under a `Recurring` tag isn't flagged as
atypical — it's silently folded into the trailing-window sum, distorting the
"expected monthly cost" the buffer math is built around.

## The fix: this isn't a workaround, it's what `Variable` is for

`Recurring`/buffering only makes sense for a real fixed-schedule commitment
— something you could point to and say "this specific amount, roughly this
often." Routine variable-amount spending that merely happens often (parking,
groceries, eating out) is exactly what `Variable` already models correctly:
a plain historical average, no buffering, no cycle assumptions.

**Recommendation: reclassify any tag like this to `Variable`.** Reserve
`Recurring` for tags backed by an actual bill. Most tags probably belong
under `Variable` — that should be the default reach, not the exception.

## When two tags really is the right call

If a category genuinely bundles two different financial behaviours — e.g. a
literal monthly garage *subscription* (fixed amount/date, wants buffering)
plus unrelated ad hoc cash parking — that's not one thing wearing two hats,
it's two things that happen to share a name. Splitting them is correct
modelling, not tag-proliferation:

- `Parkering – Garage` → `Recurring(Monthly)`
- `Parkering – Övrigt` → `Variable`

Both can roll up into **one** `BudgetItem` via `BudgetItem.tag_ids` (CLAUDE.md's
own example: "Transport = Car + Bus Pass + Train"), so the reporting layer
doesn't fragment even though the tag layer grows a little.

## Two related landmines found while tracing this

1. **CLAUDE.md overstates the wiring.** It says "tag periodicity is the
   canonical source" for `BudgetItem.periodicity`, but in code that field is
   set independently via `modify_item`
   (`api/src/models/budget.rs:245-266`) and is **never derived** from the
   item's tags' `cost_kind`. Reclassifying a tag does not retroactively
   update a `BudgetItem`'s displayed periodicity — both need to be touched
   by hand if they should agree.
2. **Mixed-cadence `BudgetItem`s flatten to the longest cycle.** If one
   `BudgetItem` groups a Monthly tag and an Annual tag,
   `required_monthly_contribution`
   (`api/src/view_models/budget_item_view_model.rs:75-84`) takes the **max**
   `cycle_months` across all the item's tags, not a per-tag breakdown. So
   grouping a real annual bill with routine variable spending under one item
   calibrates the buffer to the annual cycle for the *whole* item. Prefer
   keeping `Recurring` and `Variable` tags in separate `BudgetItem`s even
   when they're conceptually the same category, rather than merging
   everything into one line.

## What we deliberately did not build

An automatic fix — detecting a recurring sub-pattern inside one tag's
history (cluster by amount/day-of-month, buffer only the "regular" cluster,
average the rest as overflow) — is possible but a real feature: a new
algorithm, new UI to show the split, and it touches core budgeting math that
already has test coverage riding on the current one-classification-per-tag
behaviour. Not attempted here. Manually reclassifying misclassified tags to
`Variable`, and splitting genuinely-dual-behaviour categories into two tags
under one `BudgetItem`, gets correct numbers today with zero code changes.
