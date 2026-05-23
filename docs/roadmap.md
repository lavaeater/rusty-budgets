# Rusty Budgets — Roadmap

## UX Overhaul (May 2026)

Goal: make the app match the YNAB mental model — every dollar has a job, the budget must balance — and reduce the cognitive load of the main view.

---

### Sprint 1: Clean up the main view ✓ Done 2026-05-22

- [x] **1.1 Hide running deficit feature** — removed `RunningDeficitView` section from `BudgetOverview`; component kept in file for later revival.
- [x] **1.2 Collapse maintenance sections** — "Taggade transaktioner", "Taggar", "Taggningsregler" moved into collapsible `<details>` elements at bottom of page.
- [x] **1.3 "Att fördela" banner** — prominent badge in header right column. Uses `Income.remaining_budget` (already computed server-side). Red=over-assigned, amber=money left to assign, green=balanced.
- [x] **1.4 Compact period navigation** — replaced full-text buttons with `‹ 2025-05 ›` chevron style.
- [x] **1.5 Move auto-budget buttons** — collapsed behind "Verktyg ▾" `<details>` in header, keeping header clean.

---

### Sprint 2: Budget balance enforcement ✓ Done 2026-05-22

- [x] **2.1 Over-budget warning** — covered by the `rta-badge rta-over` header badge from Sprint 1 (shows "Överbudgeterat" in red with exact amount when `income_remaining < 0`).
- [x] **2.2 "Ta från..." quick reallocation** — on any over-budget item: new "Ta från..." button expands a picker listing all Expense/Savings items with enough remaining budget. Selecting one calls `reallocate_funds` (new server fn + db fn wiring up the existing `BudgetedFundsReallocated` domain event). Amount moved = exact shortfall.
- [x] **2.3 Unassigned income guard** — amber nudge banner below BudgetTabs when `ready_to_assign > 0` and at least one Income item exists. Soft reminder, not a hard block.

---

### Sprint 3: Better landing snapshot ✓ Done 2026-05-22

- [x] **3.1 Spending progress bars** — `BudgetingTypeOverviewView` now shows a horizontal progress bar (actual vs budgeted) above the stats. Blue while under, red when over.
- [x] **3.2 Per-item progress in collapsed view** — each `BudgetItemView` collapsed row has a thin mini-bar under the name. Same colour logic as the overview card.
- [x] **3.3 Current month callout** — when `period_id != today`, a grey banner appears above the tabs with a "Gå till nuvarande månad →" link.

---

### Sprint 4: Workflow speed-ups ✓ Done 2026-05-22

- [x] **4.1 Bulk-accept auto-matched transactions** — "Godkänn alla regelträffar" button in `TagTransactionsView`, visible when rules exist and untagged transactions remain. Calls new `apply_all_rules` server function (wraps existing `evaluate_tag_rules` domain logic).
- [ ] **4.2 Keyboard navigation in tagging** — arrow keys to select tag chip, Enter to confirm, Space to skip. (deferred — requires focus management wiring)

---

### Deferred / Mothballed

- **Running deficit / surplus table** (`RunningDeficitView`) — useful data but the wrong place in the UX. Will revisit once the main view is clean. Hidden as of Sprint 1.
- **Billing buffer** (`buffer_target` on `BudgetItem`) — already modelled in the domain; UI deferred until Sprint 3+ is done.
