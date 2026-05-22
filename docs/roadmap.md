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

### Sprint 2: Budget balance enforcement

- [ ] **2.1 Over-budget warning** — inline banner when total budgeted expenses + savings exceeds budgeted income, with the exact shortfall.
- [ ] **2.2 "Ta från..." quick reallocation** — on any over-budget item, a button that lets the user pick another item to pull funds from. Writes a `FundsReallocated` event.
- [ ] **2.3 Unassigned income guard** — prevent navigating away from the budget tab when "Att fördela" is still positive, with a gentle nudge to assign the remaining income.

---

### Sprint 3: Better landing snapshot

- [ ] **3.1 Spending progress bars** — replace the plain "Budgeterat / Faktiskt / Återstår" numbers in `BudgetingTypeOverviewView` with a visual progress bar (actual vs budgeted) per type.
- [ ] **3.2 Per-item progress in collapsed view** — `BudgetItemView` collapsed row shows a mini bar so you can scan the list and immediately see which items are close to or over budget.
- [ ] **3.3 Current month callout** — if viewing a past period, show a banner "Du ser {period} — klicka här för nuvarande månad."

---

### Sprint 4: Workflow speed-ups

- [ ] **4.1 Bulk-accept auto-matched transactions** — in `TagTransactionsView`, a "Godkänn alla regelträffar" button that accepts all transactions already matched by an existing rule in one click.
- [ ] **4.2 Keyboard navigation in tagging** — arrow keys to select tag chip, Enter to confirm, Space to skip.

---

### Deferred / Mothballed

- **Running deficit / surplus table** (`RunningDeficitView`) — useful data but the wrong place in the UX. Will revisit once the main view is clean. Hidden as of Sprint 1.
- **Billing buffer** (`buffer_target` on `BudgetItem`) — already modelled in the domain; UI deferred until Sprint 3+ is done.
