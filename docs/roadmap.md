# Rusty Budgets — Roadmap

> **Status audit: 2026-08-14.** Sprints 1–4 below were verified against the code
> and are genuinely done (one item deferred). Everything from "Phase 5" onward is
> **new, not started**, and reflects two findings from the audit: the domain is
> missing the one mechanic that makes envelope budgeting work (carryover), and the
> UI has outgrown its single-page layout.

---

## 🔖 Handoff — picking this up on another machine (2026-08-15)

**State: everything is committed, working tree clean.** Last three commits:

| Commit | What |
| --- | --- |
| `0b0b2f3` | `.env` setup fix — `.env.example`, path bug, actionable panics |
| `f8508bb` | **Phase 7.1 + 7.2** — the tab restructure + routing (20 files, +1927/−300) |
| `95aa461` | Clippy pedantic cleanup (UI half) |

### First thing on the new machine

The app **panics on startup without `DATABASE_URL`** — this is not a bug in the
feature work, it is missing local config (`.env` is gitignored, so a fresh clone
has none):

```sh
cp .env.example .env                            # pick SQLite or Postgres inside
cargo run -p api --bin api --features server    # creates + migrates the DB
dx serve --package web
```

> ⚠️ **Read before running the migration on the machine with the real data.**
> `api/src/main.rs` has `let migrate = true;` hardcoded, so the binary *always*
> copies a JoyDB file into SQL, defaulting to `data.json` in the working
> directory. The committed `data.json` is **the real event log — 8179 events,
> last written 2026-04-27**. So a bare run imports that April snapshot into
> whatever `DATABASE_URL` points at.
>
> - Migrating the April log into a **fresh** DB: fine, that is the intent.
> - Running it against a DB that **already holds newer events**: check first.
>   Set `DATA_FILE` to the file you actually mean, or point it at a file
>   containing `{}` to skip the copy entirely.
>
> If the other machine has a more recent `data.json` than the committed one,
> that local copy — not git — is the source of truth. Compare before importing.

### Verified green as of the handoff

- **142 native tests** (`cargo test --workspace`)
- **6 E2E specs** (`cd e2e && npm test`) — needs `npm install` +
  `npx playwright install chromium` on a fresh machine
- **Clippy pedantic clean** on all three bacon jobs (server / client / mobile)
- `dx serve` boots clean, `GET /` → 200, no panics

### ⚠️ Not verified — do this first

**Nobody has looked at the new UI in a browser.** The tab *logic* is covered by
SSR render tests and E2E, but the CSS written for the tab bar, filter chips,
attention list, and report table (`ui/assets/styling/workspace.css`, 397 new
lines) has **never been rendered visually**. Expect to spend a pass on polish.

### Suggested next step

Phase 5 (envelope carryover) is the highest-value work and unblocks the billing
buffer. **But 5.4 (cash-based global Ready-to-Assign) is a genuine philosophical
pivot** from forecast budgeting to envelope budgeting — decide that deliberately
before building toward it, rather than discovering it half-way through.

---

## Part I — Completed work (audited 2026-08-14)

### UX Overhaul (May 2026)

Goal: make the app match the YNAB mental model — every dollar has a job, the budget must balance — and reduce the cognitive load of the main view.

#### Sprint 1: Clean up the main view ✓ Done 2026-05-22 — **verified**

- [x] **1.1 Hide running deficit feature** — removed `RunningDeficitView` section from `BudgetOverview`; component kept in file for later revival. *(Verified: `budget_hero.rs:335`, defined but never mounted.)*
- [x] **1.2 Collapse maintenance sections** — "Taggade transaktioner", "Taggar", "Taggningsregler" moved into collapsible `<details>` elements at bottom of page. *(Verified: `budget_hero.rs:316-329`.)*
- [x] **1.3 "Att fördela" banner** — prominent badge in header right column. Uses `Income.remaining_budget` (already computed server-side). Red=over-assigned, amber=money left to assign, green=balanced. *(Verified: `budget_hero.rs:218-234`.)*
- [x] **1.4 Compact period navigation** — replaced full-text buttons with `‹ 2025-05 ›` chevron style. *(Verified: `budget_hero.rs:178-190`.)*
- [x] **1.5 Move auto-budget buttons** — collapsed behind "Verktyg ▾" `<details>` in header, keeping header clean. *(Verified: `budget_hero.rs:192-214`.)*

#### Sprint 2: Budget balance enforcement ✓ Done 2026-05-22 — **verified**

- [x] **2.1 Over-budget warning** — covered by the `rta-badge rta-over` header badge from Sprint 1 (shows "Överbudgeterat" in red with exact amount when `income_remaining < 0`).
- [x] **2.2 "Ta från..." quick reallocation** — on any over-budget item: new "Ta från..." button expands a picker listing all Expense/Savings items with enough remaining budget. Selecting one calls `reallocate_funds` (new server fn + db fn wiring up the existing `BudgetedFundsReallocated` domain event). Amount moved = exact shortfall.
- [x] **2.3 Unassigned income guard** — amber nudge banner below BudgetTabs when `ready_to_assign > 0` and at least one Income item exists. Soft reminder, not a hard block. *(Verified: `budget_hero.rs:265-276`.)*

#### Sprint 3: Better landing snapshot ✓ Done 2026-05-22 — **verified**

- [x] **3.1 Spending progress bars** — `BudgetingTypeOverviewView` now shows a horizontal progress bar (actual vs budgeted) above the stats. Blue while under, red when over.
- [x] **3.2 Per-item progress in collapsed view** — each `BudgetItemView` collapsed row has a thin mini-bar under the name. Same colour logic as the overview card.
- [x] **3.3 Current month callout** — when `period_id != today`, a grey banner appears above the tabs with a "Gå till nuvarande månad →" link. *(Verified: `budget_hero.rs:250-259`.)*

#### Sprint 4: Workflow speed-ups ✓ Done 2026-05-22 — **verified**

- [x] **4.1 Bulk-accept auto-matched transactions** — "Godkänn alla regelträffar" button in `TagTransactionsView`, visible when rules exist and untagged transactions remain. Calls new `apply_all_rules` server function (wraps existing `evaluate_tag_rules` domain logic).
- [ ] **4.2 Keyboard navigation in tagging** — arrow keys to select tag chip, Enter to confirm, Space to skip. *(Still deferred — requires focus management wiring. Now folded into Phase 7.3.)*

### Infrastructure (Aug 2026) — **done, undocumented until now**

- [x] **SQL migration** — the production runtime moved from JoyDB to `PgRuntime`
  (welds/sqlx, SQLite + Postgres) with a migration binary. `JoyDbBudgetRuntime`
  survives as the in-memory test harness only.
- [x] **Test suite** — 127 tests green across four layers. See `docs/testing.md`.
- [x] **CI** — `.github/workflows/{ci,e2e}.yml`.
- [x] **Clippy pedantic** — enabled workspace-wide (`[workspace.lints.clippy]`
  plus `[lints] workspace = true` on all six crates) and clean across the
  `clippy-server` / `clippy-client` / `clippy-mobile` bacon jobs.
  > The lints were declared in the workspace `Cargo.toml` but **no crate had
  > opted in**, so pedantic had never actually run. Enabling it surfaced ~214
  > warnings; all fixed.
- [x] **Schema now applied automatically at startup** ✓ 2026-08-15 — a second
  first-run papercut: with `DATABASE_URL=sqlite://data.sqlite?mode=rwc`, the
  server **silently created an empty database file** and then failed every query
  with `no such table: users`, panicking with a useless
  `"Could not get default user"`. `create_runtime()` now runs
  `migrations::up` before handing the client out — idempotent, and it means a
  fresh clone works with only a `.env`.
  > Deliberate split: **schema is automatic, data import is not.** Importing a
  > JoyDB log overwrites real data, so that stays an explicit
  > `cargo run -p api --bin api --features server`. The panic message now names
  > that command and includes the underlying error.
- [x] **Local-dev setup made discoverable** ✓ 2026-08-14 (`0b0b2f3`) — the app
  panicked on startup with a bare `NotPresent` when `DATABASE_URL` was unset,
  and nothing in the repo said it was required (`.env` is gitignored). Three
  fixes:
  - `.env.example` added and **tracked**, covering SQLite + Postgres and the
    optional `DATA_FILE`.
  - **Real path bug:** the `.env` fallback did `.parent().and_then(|p| p.parent())`
    from `CARGO_MANIFEST_DIR` — but that is `<workspace>/api`, so two parents
    overshoot the workspace root and land *outside the project*. The stale
    comment ("two levels up from packages/web") described an older layout. It
    only ever worked because `dotenvy::dotenv()` searches upward from the cwd,
    making it depend on where you launched from. Now one parent.
  - Actionable panic messages in `create_runtime()` and the migration binary,
    naming the fix; connection failures now report the URL that was tried.
  > This had been latent since the JoyDB→SQL move. The E2E suite never caught it
  > because `playwright.config.js` injects `DATABASE_URL` itself — the same
  > class of blind spot as the JoyDB-vs-`PgRuntime` gap in `docs/testing.md`.

---

## Part II — Open work

### ⚠️ Documentation drift to fix first

`CLAUDE.md` is stale in three places and will mislead any future work:

- [ ] **DB layer** — says "JoyDB — custom JSON/RON file-based database (no SQL)"
  and "`api/src/db.rs` is a thin wrapper around `JoyDbBudgetRuntime`". Production
  is now `PgRuntime` (welds/sqlx over SQLite/Postgres); `db.rs:24` holds a
  `OnceCell<PgRuntime>`. JoyDB is now **test-harness only**.
- [ ] **Dioxus version** — says 0.7.3, workspace is on 0.8.0-alpha.1.
- [ ] **`ui/src/lib.rs` comment** — refers to `budget_a` / `budget_b` module
  variants that no longer exist.

---

### 🐛 Known issues — found, not yet fixed

Small things surfaced while working; none block Phase 5.

- [ ] **Migration binary always migrates** — `let migrate = true;` is hardcoded
  in `api/src/main.rs`, so every run copies `DATA_FILE` (default `data.json`, and
  the repo ships one) into SQL. Should be a flag or an env var. **This is the
  handoff hazard called out at the top of this file.**
- [ ] **`BudgetHero` SSR renders only "Laddar…"** — the state machine starts in
  `Loading` and resolves in a `use_effect`, which does not run during SSR. Every
  first paint is a spinner, hydrating to real content. Fine functionally, but it
  wastes the SSR pass entirely and hurts perceived load. Worth moving the
  resolution into the `use_server_future` branch instead.
- [ ] **`data.json` at the repo root is the real event log, and it is committed**
  — 8179 `StoredBudgetEvent`s, 1 budget, 1 user (plus a 7.5 MB `data.bak`).
  Both are tracked, last written 2026-04-27. Consequences worth a decision:
  - It is the **default `DATA_FILE`**, so any bare run of the migration binary
    imports these 8179 events into whatever `DATABASE_URL` points at.
  - Personal financial data is in git history, and two ~7.5 MB blobs are carried
    by every clone.
  - It is the JoyDB-era source of truth; post-migration the SQL DB is. Decide
    whether `data.json` is now an **archive** (untrack, keep a local copy) or a
    **fixture** (shrink to an anonymised sample) — right now it is quietly both.
- [ ] **Layer 2 tests run against the wrong runtime** — 34 CQRS tests use
  `JoyDbBudgetRuntime`, production uses `PgRuntime`. Full detail and the
  recommended fix are in `docs/testing.md` (rollout item 6).

---

### Phase 5 — Envelope carryover (the missing domain mechanic) 🔴 blocking

**This is the highest-value change in the roadmap and everything in Phase 6 depends on it.**

The app currently computes, per period and independently:

```
remaining_budget = budgeted_amount − actual_amount
```

Every period starts from zero. There is no link between a category's balance in
month N and month N+1. That single omission is why:

- **Sinking funds / the billing buffer cannot work.** `buffer_target` and
  `required_monthly_contribution` are already modelled on `BudgetItem` /
  `BudgetItemViewModel`, but there is nothing for the contributions to *accumulate
  into*. Phase 6 of `CLAUDE.md` is not a UI task — it is blocked on this.
- **Overspending silently disappears.** Blowing the food budget in March has no
  consequence in April.
- **Underspending is not rewarded.** Money saved in a category evaporates instead
  of building a cushion.

The fix is the YNAB identity:

```
Available(cat, month) = Available(cat, month−1) + Assigned(cat, month) − Activity(cat, month)
```

- [ ] **5.1 Add `carried_over: Money` to `ActualItem`**, plus an `available()`
  accessor implementing the identity above. New event
  `actual_carryover_set` (or derive it during replay — decide explicitly, and
  document the choice; deriving keeps the event log smaller but makes replay
  order-sensitive).
- [ ] **5.2 Carry forward on period creation** — when a `BudgetPeriod` is
  materialised, seed each `ActualItem.carried_over` from the previous period's
  `available()`.
- [ ] **5.3 Decide overspend policy** — YNAB subtracts cash overspending from
  next month's Ready-to-Assign, and rolls credit overspending forward as a
  negative category balance. Pick one, and make it a documented invariant with a
  CQRS test.
- [ ] **5.4 Global Ready-to-Assign** — today `income_remaining = income_budgeted −
  expense_budgeted − savings_budgeted` (`budget_view_model.rs:137`), which is
  **period-local and derived from *budgeted* income**. Two problems: leftover
  money in a month vanishes rather than flowing to the next month, and the app
  budgets *forecast* income rather than *money that actually exists*. Move to
  `RTA = Σ(account balances) − Σ(assigned across all periods)`.
  `BankAccount.balance` already exists and is populated by import but is unused
  in this calculation.
  > This is the single biggest philosophical gap with YNAB. Budgeting forecast
  > income is what makes budgets fail when income is irregular — precisely the
  > case (Swedish households with barnbidrag/CSN/variable income) the app is for.
- [ ] **5.5 Buffer UI** — only after 5.1–5.4: show buffer balance vs
  `buffer_target`, with progress and a "fill this month" action.

### Phase 6 — Concept gaps vs mainstream household budgeting

Ranked by household value. See "Concept review" at the bottom for reasoning.

- [ ] **6.1 Transaction splitting in the tagging flow** 🔴 — `BankTransaction.tag_id`
  is `Option<Uuid>`: **one tag per transaction**. The canonical Swedish case is a
  bolån payment that is part *ränta* (Expense) and part *amortering* (Savings /
  debt reduction), and it cannot be represented. `TransactionAllocation` already
  supports splitting but is wired only to the **legacy** `actual_id` path, not to
  the tag workflow. The old design notes flagged this explicitly.
- [ ] **6.2 Unify the two categorisation systems** 🔴 — there are currently two
  half-connected paths: legacy `actual_id` + `TransactionAllocation` → `ActualItem`
  (supports splits), and the new `tag_id` → `Tag` → `BudgetItem.tag_ids`
  (single-valued). `budget_view_model.rs:111-113` admits it outright: overviews
  are computed from tag-based amounts *"rather than from the RulePackages/ActualItem
  system **which is never updated by the tagging workflow**"* — while
  `budgeted_amount` still lives on `ActualItem`. Budgeted amounts live in the old
  system, actuals in the new one. Pick one and delete the other.
- [ ] **6.3 Targets / goals** — YNAB's most-used feature after RTA. Generalise
  `buffer_target` into a `Target` enum: *save X by date*, *save X per month*,
  *spend up to X per month*, *have X available*. Drives "underfunded this month"
  and a one-click "fund all targets".
- [ ] **6.4 Reconciliation** — `BankTransaction.balance` is imported but unused.
  A "does my budget match the bank?" flow is what makes the numbers trustworthy;
  without it every other figure is provisional.
- [ ] **6.5 Upcoming / scheduled transactions** — no forward-looking cashflow
  exists. Households need *"what is due before next payday, and do I have it?"*
  Swedish autogiro/e-faktura bills are highly regular and easy to model.
- [ ] **6.6 Reports** — `PeriodSummary` / `running_net` are already computed and
  the `RunningDeficitView` component already exists but is unmounted. Revive it
  in a proper Reports tab (Phase 7) with spend-by-tag-over-time.
- [ ] **6.7 Accounts & net worth view** — `BankAccount` exists with balances, but
  there is no screen showing accounts, balances, or net worth. Needed for 5.4.
- [ ] **6.8 Debt tracking** — mortgage principal is currently just "Savings".
  Real debt modelling (balance, rate, payoff projection) is standard in
  mainstream tools and directly relevant to Swedish amortisation rules.
- [ ] **6.9 Age of Money / buffer months** — YNAB Rule 4. The strongest single
  indicator of household financial stability ("am I living on last month's
  income?"). Cheap to compute once 5.4 lands.
- [ ] **6.10 Credit card handling** — deferred. YNAB's hardest mechanic (card
  spending moves budgeted money into a payment category). Lower priority in a
  Swedish context where debit dominates.

**Already aligned with YNAB — no work needed:** Rule 1 *Give Every Dollar a Job*
(the "Att fördela" badge, Sprint 1.3), Rule 3 *Roll With the Punches* (the
"Ta från..." reallocation, Sprint 2.2), and the strong tagging/auto-rule engine,
which is genuinely better than YNAB's payee matching for Swedish bank exports.

---

### Phase 7 — UI restructure: from one page to a period workspace 🔴

**Problem.** `BudgetOverview` (`ui/src/budget/budget_hero.rs:142-332`) renders
**twelve stacked sections** in a single scroll: header, past-period banner,
`BudgetTabs`, assign nudge, `TagTransactionsView`, `TransferPairsView`,
`CreateBudgetItemsView`, `TransactionsView` (to-connect), `TransactionsView`
(ignored), then `<details>` blocks for `RetagTransactionsView`, `TagsView` and
`RulesView`.

Worse, most of those sections are **conditional**, so the page *changes shape* as
you work: `CreateBudgetItemsView` only appears once `untagged_transaction_count == 0`
**and** `potential_transfer_count == 0`. The layout is a side effect of your data
rather than a stable place to stand.

The existing `BudgetTabs` is *not* a solution to this — it switches
`BudgetingType` (Inkomst/Utgift/Sparande/Överföring), which is a **filter inside
the budget table**, not workflow separation.

**Proposal — six period-scoped tabs.** Period navigation stays global in the
header (tabs are views *within* the selected month, matching "every month should
have several tabs"):

| Tab | Purpose | Absorbs |
| --- | --- | --- |
| **Översikt** | Landing snapshot: RTA hero, per-type progress, biggest overspends, "what needs attention". Read-mostly. | header stats, assign nudge, past-period banner |
| **Budget** | The envelope table — assign money, adjust, "Ta från…". The day-to-day surface. | `BudgetTabs`, `BudgetItemView`, `BudgetingTypeCard` |
| **Transaktioner** | Every transaction in the period; filter by tagged/untagged/ignored. | both `TransactionsView`s + `RetagTransactionsView` |
| **Att göra** | The work queue, with a **count badge on the tab**: untagged batch, transfer pairs, unconnected. | `TagTransactionsView`, `TransferPairsView` |
| **Rapporter** | Period summaries, running net, spend-by-tag trends. | revives `RunningDeficitView` (6.6) |
| **Inställningar** | Tags, rules, accounts, import, `month_begins_on`, currency. | `TagsView`, `RulesView`, `FileDialog`, Verktyg |

`CreateBudgetItemsView` is deliberately **not** a tab — it is an onboarding
wizard, not a recurring task. Surface it from Översikt whenever unbudgeted tags
exist.

- [x] **7.1 Introduce routing** ✓ Done 2026-08-14 — `web` now serves
  `/budget/:period/:tab`. `PeriodId` gained `FromStr` (+ `PeriodIdParseError`,
  4 tests) so it can be a URL segment. `ui` stays router-agnostic: `BudgetHero`
  takes an optional `location: BudgetLocation` and `on_navigate` handler,
  syncing both ways, so desktop/mobile keep their internal state with **no
  changes** to those crates. Tab switches `push` (not `replace`) so the back
  button returns to the previous tab.
  > **Gotcha 1 — inbound sync needs `use_reactive!`.** `location` is a plain
  > prop, so a bare `use_effect` captures the first render's value and never
  > sees a change; combined with `peek()` (which doesn't subscribe) the effect
  > had no reactive dependencies at all and ran exactly once. Back/forward would
  > have silently done nothing. A `published` signal guards both directions
  > against a URL echo bouncing back as a fresh navigation.
  >
  > **Gotcha 2 — `#[redirect]` does not rewrite the address bar.** `/` resolves
  > to and *renders* `Route::Budget { current period, Översikt }`, but the URL
  > stays `/`. This was caught by the E2E suite, not by any native test. That is
  > acceptable behaviour — `/` is a legitimate home URL, and the first tab click
  > canonicalises it — but do not assume the redirect produces a canonical URL.
- [x] **7.2 Split `BudgetOverview`** ✓ Done 2026-08-14 — the 190-line
  twelve-section component is now a 9-line delegation to `BudgetWorkspace`
  (`ui/src/budget/workspace.rs`) plus six tab components under
  `ui/src/budget/tabs/`. `BudgetTabs` was renamed **`BudgetingTypeTabs`** to stop
  it being confused with the new workspace-level `BudgetTab`.
  Inactive tabs do not mount — the vendored `TabContent` only renders children
  when selected — so only the active tab's hooks and server calls run.
- [x] **7.2a Restore the orphaned stylesheet** ✓ 2026-08-15 — **regression from
  7.2, reported as "styling seems to be missing".** `budget-hero.css` (155 rules:
  container, header, RTA badge, dashboard/overview cards, period nav, progress
  bars) was linked from the top of the old `BudgetOverview` body. Replacing that
  body with `BudgetWorkspace` took the `document::Link` with it, leaving the
  sheet referenced **only** in the `NoDefaultBudget` branch — so the create-budget
  screen was styled and the entire loaded-budget view was not. `HERO_CSS` is now
  `pub(crate)` and linked from `BudgetWorkspace` alongside `workspace.css`.
  > Lesson for the remaining tab work: the other sheets (`tags.css`, `rules.css`,
  > `tag-transactions.css`, `retag-transactions.css`, `create-budget-items.css`)
  > are each linked *inside* the component that needs them, so they survived the
  > split. `budget-hero.css` was the one linked by the *container*. When moving a
  > component between parents, check what its old parent was linking on its
  > behalf. Note also that **no test caught this** — SSR render tests assert on
  > markup, not on stylesheet links.
- [ ] **7.3 Keyboard navigation** in the Att göra queue — the deferred Sprint 4.2,
  which makes far more sense now that tagging has a dedicated surface.
- [ ] **7.4 Trim the view model per tab** — `BudgetViewModel` is one large
  projection fetched for every render. With tabs, most screens need a fraction of
  it; consider per-tab projections (and note `from_budget` is already large enough
  to have earned a `#[allow(clippy::too_many_lines)]`). **Now unblocked by 7.2** —
  each tab component owns its own data needs.
- [ ] **7.5 Add `data-testid` hooks** — partially addressed: the tab bar is
  reachable via ARIA roles (`getByRole("tab", …)`) because the vendored `Tabs`
  primitive emits them, and the E2E suite now uses that. Deeper views still need
  stable hooks.

**Sequencing note.** 7.1 + 7.2 landed **before** Phase 5/6 UI work by design.
Every new feature (buffers, targets, reports, reconciliation) now has an obvious
home: buffers and targets → Budget, reconciliation → Transaktioner, Age of Money
and trends → Rapporter.

#### What landed in 7.1 + 7.2

| Tab | Contents | Notes |
| --- | --- | --- |
| Översikt | Attention list, per-type overview cards, empty state | Each attention row is a button that switches to the owning tab |
| Budget | `BudgetingTypeTabs`, assign nudge, `CreateBudgetItemsView` | Item-creation section auto-opens when unbudgeted tags exist |
| Transaktioner | Filter chips: Taggade / Att koppla / Ignorerade | Replaces two stacked `TransactionsView`s + a `<details>` block |
| Att göra | Tagging batch, transfer pairs, to-connect | **Count badge on the tab**; single "inget att göra" state |
| Rapporter | `RunningDeficitView` | **Revived** — unmounted since Sprint 1 |
| Inställningar | Import, Verktyg, Taggar, Taggningsregler | Was the page header + three `<details>` blocks |

Tests added: 4 (`BudgetTab` slug round-trip/uniqueness) + 4 (`PeriodId::from_str`)
+ 5 (workspace render: all tabs present, default tab, badge arithmetic, attention
list) + 3 E2E specs (tab bar, URL updates on tab switch, deep link). **Workspace
total: 139 native tests green, clippy pedantic clean on all three bacon jobs.**

---

## Deferred / Mothballed

- **Running deficit / surplus table** (`RunningDeficitView`) — useful data, wrong
  place in the UX. Component still exists, unmounted since Sprint 1. **Revival
  path: the Rapporter tab (7.x + 6.6).**
- **Billing buffer** (`buffer_target` on `BudgetItem`) — partially modelled
  (`buffer_target`, `required_monthly_contribution`), no UI. **No longer merely
  deferred: it is blocked on Phase 5 carryover** and cannot be built without it.
- **Credit card payment categories** (6.10) — genuinely deferred; low value in a
  debit-dominant market.

---

## Concept review — how this compares to YNAB and mainstream budgeting

*Audit performed 2026-08-14 against the domain model in `api/src/models/`.*

**The four YNAB rules, scored:**

| Rule | Status | Evidence |
| --- | --- | --- |
| 1. Give Every Dollar a Job | ✅ **Good** | "Att fördela" badge, over-assignment warning, assign nudge |
| 2. Embrace Your True Expenses | ❌ **Cannot work today** | `buffer_target` modelled but no balance accumulates across periods → Phase 5 |
| 3. Roll With the Punches | ✅ **Good** | "Ta från…" reallocation via `BudgetedFundsReallocated` |
| 4. Age Your Money | ❌ **Not modelled** | → 6.9 |

**The one structural difference that matters most.** YNAB is an *envelope*
system: category balances persist and accumulate, and you budget money that
physically exists in your accounts. Rusty Budgets is currently a *budget-vs-actual
variance tracker*: each period is independent, and the budget is built from
*forecast* income. Both are legitimate designs — but the app's stated goal
("match the YNAB mental model", `CLAUDE.md` billing-buffer concept) requires the
envelope model, and the buffer feature is impossible without it. Phase 5 is
therefore the pivotal decision, not a refinement.

**Where Rusty Budgets is genuinely better than mainstream tools:** the
tag + auto-`MatchRule` engine with Swedish-localised tokenisation, and the
transfer-pair resolution that correctly models a savings contribution as a
budget event on the *spending* side with the receipt side ignored. Neither YNAB
nor the Swedish incumbents handle bank-export categorisation this well.
