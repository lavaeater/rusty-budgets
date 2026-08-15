# Rusty Budgets — Roadmap

> **Last updated 2026-08-15.** Started as an audit on 2026-08-14, when Sprints
> 1–4 were verified done and two gaps were identified: the domain was missing the
> mechanic that makes envelope budgeting work (carryover), and the UI had outgrown
> its single-page layout. **Both have since been addressed** — Phase 7 (tabs),
> Phase 8 (bills vs variable spending) and Phase 5.1–5.3 (carryover) have landed.
> What remains open is listed under Part II; the largest single item is
> **5.4, cash-based Ready-to-Assign, which is deliberately deferred** with reasons.

---

## 🔖 Handoff — picking this up on another machine

**Branch `forecast`. Everything is committed, working tree clean.**

| Commit | What |
| --- | --- |
| `3347e83` | **Phase 5.1–5.3 wiring** — carryover settings screen, "Tillgängligt" badge, `preview_carryover` example |
| `7ec89bf` | Carryover domain — `carryover_into`, `CarryoverConfigured`, envelope identity + 6 CQRS tests |
| `72b4ef2` | **Phase 8** — bills vs variable spending: `CostKind`/`Matching`, `TagClassified`, tag review, suggestion inbox |
| `0b0b2f3` | `.env` setup fix — `.env.example`, path bug, actionable panics |
| `f8508bb` | **Phase 7.1 + 7.2** — the tab restructure + routing |

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

- **179 native tests** (`cargo test --workspace`)
- **7 E2E specs** (`cd e2e && npm test`) — needs `npm install` +
  `npx playwright install chromium` on a fresh machine
- **Clippy pedantic clean** on all three bacon jobs (server / client / mobile)
- `dx serve` boots clean, `GET /` → 200, no panics
- The **real production snapshot** still deserialises:
  `cargo run -p api --example verify_snapshot -- <budget.json>`
  (57 tags, 28 periods, version 8179 — 23 automatic, 34 needing review)

### ⚠️ Not verified — do this first

**Five screens have never been looked at in a browser.** Their *logic* is covered
by domain, SSR-render and E2E tests, but the CSS has only ever been reasoned
about:

| Screen | Stylesheet |
| --- | --- |
| Workspace tab bar, filter chips, attention list, report table | `workspace.css` (397 lines) |
| Tag classification review | `tag-review.css` |
| Suggestion inbox | `tag-suggestions.css` |
| Carryover settings + "Tillgängligt" badge | `carryover.css` |

The E2E suite runs against an **empty** database, so it proves nothing
regressed — not that these screens look right. Two of them cannot render at all
on the current data (see below). Expect a polish pass.

### ⚠️ Two features are invisible on the current production data

Both are working and tested; neither will show you anything until the data
changes. Worth knowing before concluding something is broken:

- **Suggestion inbox** — the live snapshot has **0 untagged transactions**, so
  there is nothing to suggest and Att göra reads "Inget att göra". It appears
  after the next import.
- **Carryover** — `preview_carryover` reports **"Carried in" = 0 kr for every
  item**, because the historical budgets came from auto-budget
  (`budgeted == actual`) and left nothing over. It starts producing numbers once
  you budget *forward* rather than auto-budgeting from actuals — which is the
  whole point of the annual-bill case.

### Suggested next step

Pick one of:

1. **Polish pass in the browser** — cheapest way to de-risk five unseen screens,
   and the only outstanding work on already-shipped features.
2. **6.11 buffer progress UI** — now purely presentational: `TagSummary::buffer_target()`
   gives the target and carryover accumulates towards it. Makes the annual-bill
   case visible rather than just correct.
3. **E2E fixture that imports a Skandia file** — unlocks browser coverage for the
   tagging loop, the suggestion inbox and carryover, all of which are currently
   untestable end-to-end because the suite starts from an empty DB. Already the
   named next step in `docs/testing.md`.

**Not recommended yet: 5.4 (cash-based Ready-to-Assign).** See Phase 5 below —
account balances are unpopulated, four months stale, and the account registry
has duplicate entries. Prerequisites are listed there.

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

Small things surfaced while working; none block the open phases.

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
- [ ] **Layer 2 tests run against the wrong runtime** — the CQRS tests (now 53)
  use `JoyDbBudgetRuntime`; production uses `PgRuntime`. Full detail and the
  recommended fix are in `docs/testing.md` (rollout item 6).
- [ ] **Duplicate accounts from inconsistent number formats** — the registry has
  **14 entries for 11 distinct accounts**: `9159-482.485-3` and `91594824853`
  are the same account written two ways. Latent today because only 3 accounts
  carry transactions and each uses one format consistently, but anything summing
  over `budget.accounts` would double-count. Normalise on import (strip
  non-digits) before 5.4 or 6.7 rely on it.
- [x] **`data.sqlite` was not gitignored** ✓ 2026-08-15 — `*.sqlite` added to
  `.gitignore`. Verified no earlier commit captured it
  (`git log --all -- '*.sqlite'` is empty); it is the live database, ~4.8 MB of
  real financial data.

---

### Phase 8 — Bills vs variable spending ◐ shipped except the buffer

**The insight (2026-08-15):** recurring bills and card spending want opposite
treatment. A bill's payee text is stable, so rules can auto-apply it; a card
purchase needs confirming. And a bill billed yearly should be *periodised* — the
12 000 kr dog insurance budgets as 1 000 kr/month — while variable spending is
budgeted as it happens.

**What the data forced:** the two behaviours do **not** collapse into one flag.
`Livsmedel` is variable spending whose payees (ICA, Coop) match perfectly;
`Bil - Underhåll` is a genuine recurring cost billed by a different garage every
time. So `Tag` carries two independent fields with linked defaults:

```rust
cost_kind: CostKind,   // Recurring(Periodicity) | Variable  -> budgeting
matching:  Matching,   // Automatic | Suggest                -> import
```

`CostKind::default_matching()` links them (Recurring⇒Automatic, Variable⇒Suggest),
so the inline pickers set one thing and the review screen exposes both.

- [x] **8.1 Classification on `Tag`** ✓ 2026-08-15 — `CostKind`/`Matching`, with
  `cycle_months`, `needs_buffer`, `default_matching`.
- [x] **8.2 New `TagClassified` event** ✓ — deliberately *additive*. Historical
  `TagCreated` events carry only a `Periodicity` and are immutable; replay
  derives a provisional classification via `Tag::from_legacy_periodicity`, and
  `TagClassified` overrides it. `explicitly_classified` keeps a legacy
  periodicity edit from clobbering a deliberate answer.
- [x] **8.3 Snapshot back-compat** ✓ 🔴 — **this would have broken production.**
  `PgRuntime::load` deserialises a stored `Budget` *snapshot* and replays only
  the events after it; the live snapshot holds all 57 tags in the old shape
  `{id, name, periodicity, deleted}`. Adding required fields would have made
  every budget fail to load. A `TagRepr` shim accepts both shapes. Guarded by
  5 unit tests **and** `cargo run -p api --example verify_snapshot -- <file>`,
  which loads a real snapshot end-to-end — run it after any aggregate schema
  change.
- [x] **8.4 Gated auto-categorisation** ✓ — `evaluate_tag_rules` now returns
  only `Automatic` matches; `suggest_tag_rules` returns the `Suggest` ones. Same
  engine, always disjoint. Deleted tags match neither. 6 CQRS tests.
  > `apply_all_rules` (the "Godkänn alla regelträffar" button) deliberately
  > applies **both** sets — implicit evaluation after an import is gated, but an
  > explicit bulk approval is the user asking for the suggestions too. Without
  > this the button would only re-apply bills that had already auto-applied.
- [x] **8.5 Guided review of the 34 ambiguous tags** ✓ — `Periodicity::default()`
  was `OneOff`, so 34 of 57 production tags sit in the "never answered" bucket,
  mixing real bills (`Bredband`, `Fackförbund`, `Skatt`, `Lön Tommie`) with
  genuine ad-hoc spending (`Café`, `Shopping`, `Äta ute`). A mechanical
  `OneOff → Variable` migration would have silently switched off
  auto-categorisation for the bills, so each is asked about instead, with its
  real spend shown. `TagReviewView` in Inställningar, surfaced from the Översikt
  attention list via `tags_needing_review_count`.
  > Evidence of default-drift: `Lön Lisa` is Monthly while `Lön Tommie` is
  > OneOff — the same thing classified two ways.
- [x] **8.6 Periodised monthly contribution** ✓ —
  `TagSummary::monthly_budget_contribution` spreads a bill across its cycle. Uses
  the **trailing cycle's** actual spend ÷ cycle length, not the whole-history
  average, because the naive average is badly skewed by a partial window: 13
  months containing two annual payments averages to 1 846 kr/month, not 1 000.
  Monthly and `Variable` costs keep the window average, which beats any single
  recent month for them.
- [x] **8.7 Suggestion inbox** ✓ 2026-08-15 — `TagSuggestionsView`, first section
  of the **Att göra** tab. **Grouped by proposed tag**, largest group first: one
  import can produce dozens of matches for the same payee, and "18 transaktioner
  ser ut som Livsmedel — godkänn alla" is one decision instead of eighteen.
  Per-group and global confirm, expand to review individually.
  - Bulk confirm goes through `db::confirm_tag_suggestions`, **not** a loop over
    `db::tag_transaction` — that wrapper reloads the aggregate, checks for a
    missing rule and re-runs a full rule evaluation *per call*, which would be
    quadratic over a batch. A suggestion already implies its rule exists.
  - The matches are re-derived **server-side** from `tag_id`, never sent from
    the client, so a stale inbox cannot tag the wrong transaction.
  - "Hoppa över" is **local-only and deliberately not persisted** — the
    transaction stays untagged, so the suggestion returns on reload. The button
    is worded as *skip*, not *reject*, to match. A CQRS test
    (`skipping_is_not_persisted`) guards against it quietly becoming persistent.
  > **Not visible on the current production data:** the live snapshot has
  > **0 untagged transactions**, so there are no suggestions to show and the Att
  > göra tab reads "Inget att göra". The inbox only appears after importing new
  > transactions. Verified with
  > `cargo run -p api --example verify_snapshot` (now also reports
  > untagged / auto-applies / suggestions, grouped by tag).
- [x] **8.8 Buffers actually accumulate** ✓ 2026-08-15 — **unblocked by Phase
  5.1–5.3.** `an_annual_bill_is_covered_by_the_accumulated_buffer` proves the
  full cycle: assign 1 000 kr/month, the bill lands in December, 11 000 carried
  + 1 000 assigned − 12 000 spent = **0**, not an 11 000 kr overspend.
  `TagSummary::buffer_target()` gives the target; carryover accumulates towards
  it. Still to do: show buffer progress against that target in the UI
  (**6.11**), which is now purely presentational.

---

### Phase 5 — Envelope carryover ◐ 5.1–5.3 done, 5.4 deliberately deferred

**Decided 2026-08-15.** The pivot was two decisions, not one, and separating them
is what unblocked it:

| | What it changes | Status |
| --- | --- | --- |
| **A. Category carryover** | Money left in a category stays in that category | ✅ **Done** |
| **B. Cash-based RTA** | "Att fördela" comes from bank balances, not forecast income | ⏸ **Deferred** |

**A is what makes the billing buffer work. B is the philosophical pivot, and you
do not need it to get the buffer.**

- [x] **5.1 `carryover_from` + the envelope identity** ✓ —
  `Budget::carryover_into(period)` accumulates `budgeted − actual` from the
  chosen start month. `BudgetItemViewModel` gains `carried_over` and
  `available`.
  > **Derived, not stored.** Storing `carried_over` per `ActualItem` would need
  > an event per item per period on an already-8179-event log, and would go
  > stale the moment a past month was edited. Deriving it means editing March
  > flows forward into April automatically. Cost is one pass over the periods
  > per projection.
  > `Budget::item_period_totals` is shared with the projection so carryover and
  > the displayed numbers cannot drift apart.
- [x] **5.2 Opt-in and dated** ✓ — `carryover_from: Option<PeriodId>`, set by a
  `CarryoverConfigured` event, `None` by default. Existing budgets are
  **completely unaffected** until switched on, and it can be switched back off.
  Dated because the log holds two years from before the budget was kept
  properly, where most `ActualItem`s have `budgeted_amount == 0` — accumulating
  across all of it would compound spending-with-no-budget into nonsense.
- [x] **5.3 Overspend policy: the category carries the debt** ✓ — an overspent
  category starts the next month negative and must be topped up. Chosen over
  YNAB's "deduct from next month's RTA" because the consequence stays attached
  to the category that caused it.
- [x] **5.x UI** ✓ — `CarryoverSettings` in Inställningar (on/off + start month),
  and an "Tillgängligt" badge per budget row, red when negative, with the full
  `carried + budgeted − spent` sum in its tooltip. The badge only appears when
  carryover is on, so it never sits next to `remaining_budget` looking like a
  duplicate.

> ### ⚠️ It will do nothing on your current history — and that is expected
>
> `cargo run -p api --example preview_carryover -- <snap.json> 2026-1 2026-3`
> shows **"Carried in" = 0 kr for every item**, because "Remaining" is also 0 kr
> for every item in every historical month. Those budgets were produced by
> **auto-budget, which sets budgeted = actual**, so nothing was ever left over
> to carry.
>
> Carryover only produces numbers once you budget a *forward-looking* amount
> that differs from what you spend — which is precisely the dog-insurance case
> (assign 1 000 kr/month, spend nothing for eleven months). The mechanism is
> proven by 6 CQRS tests including the full twelve-month cycle; it is aimed at
> the workflow going forward, not at reinterpreting the past.

- [ ] **5.4 Cash-based global Ready-to-Assign — deferred, and here is why**
  measured against the real data:
  - `BankAccount.balance` is **0.00 for all 14 accounts** — nothing ever
    populates it. Real balances are only recoverable from the last transaction
    per account (3 active accounts, ~28 708 kr).
  - That data is **four months stale** (latest transaction 2026-04-07).
  - The account registry holds **14 entries for 11 distinct accounts** —
    `9159-482.485-3` and `91594824853` are the same account in two formats.
    Latent today because only 3 accounts carry transactions, but anything
    summing over `budget.accounts` would double-count.

  Forecast budgeting degrades *gracefully* when imports lag; cash-based
  budgeting degrades *silently and wrongly*. Prerequisites before revisiting:
  populate `BankAccount.balance` on import (cheap, and unblocks 6.7), then 6.4
  reconciliation and 6.7 accounts view. A cash RTA you cannot trust is worse
  than a forecast RTA you understand.

---

<details>
<summary><strong>Phase 5 — the original analysis (2026-08-14), kept for the reasoning</strong></summary>

> **Superseded by the section above.** Written before carryover was built, so it
> describes the then-current state in the present tense. Kept because the
> reasoning still explains *why* carryover matters; the task list it contained
> has been removed to avoid contradicting the real status.

The app computed, per period and independently:

```
remaining_budget = budgeted_amount − actual_amount
```

Every period started from zero, with no link between a category's balance in
month N and month N+1. That single omission was why:

- **Sinking funds / the billing buffer could not work.** `buffer_target` and
  `required_monthly_contribution` were already modelled, but there was nothing
  for the contributions to *accumulate into*.
- **Overspending silently disappeared.** Blowing the food budget in March had no
  consequence in April.
- **Underspending was not rewarded.** Money saved in a category evaporated
  instead of building a cushion.

The fix is the YNAB identity, now implemented in `Budget::carryover_into`:

```
Available(cat, month) = Available(cat, month−1) + Assigned(cat, month) − Activity(cat, month)
```

The original notes also assumed `BankAccount.balance` "is populated by import".
**That turned out to be false** — it is 0.00 for all 14 accounts, which is a
large part of why 5.4 is deferred.

</details>

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
- ◐ **6.6 Reports** — `RunningDeficitView` was **revived in Phase 7.2** and now
  lives in the Rapporter tab. Still open: spend-by-tag over time, and anything
  beyond the running net table.
- [ ] **6.7 Accounts & net worth view** — `BankAccount` exists but its `balance`
  is **0.00 for all 14 accounts; nothing ever populates it**. Real balances are
  only recoverable from the last transaction per account. So this splits in two:
  **(a)** populate `BankAccount.balance` on import — small, and the prerequisite
  for everything else here; **(b)** a screen showing accounts and net worth.
  Both are prerequisites for 5.4. The account registry also holds 14 entries for
  11 distinct accounts (see Known issues).
- [ ] **6.8 Debt tracking** — mortgage principal is currently just "Savings".
  Real debt modelling (balance, rate, payoff projection) is standard in
  mainstream tools and directly relevant to Swedish amortisation rules.
- [ ] **6.9 Age of Money / buffer months** — YNAB Rule 4. The strongest single
  indicator of household financial stability ("am I living on last month's
  income?"). Cheap to compute once 5.4 lands.
- [ ] **6.11 Buffer progress UI** — now purely presentational and **unblocked**:
  `TagSummary::buffer_target()` gives the target and carryover accumulates
  towards it, so a "1 000 / 12 000 kr saved towards Hund" bar is a rendering job.
  Makes the annual-bill case visible rather than merely correct. Was 5.5 in the
  original notes.
- [ ] **6.10 Credit card handling** — deferred. YNAB's hardest mechanic (card
  spending moves budgeted money into a payment category). Lower priority in a
  Swedish context where debit dominates.

**Already aligned with YNAB — no work needed:** Rule 1 *Give Every Dollar a Job*
(the "Att fördela" badge, Sprint 1.3), Rule 3 *Roll With the Punches* (the
"Ta från..." reallocation, Sprint 2.2), and the strong tagging/auto-rule engine,
which is genuinely better than YNAB's payee matching for Swedish bank exports.

---

### Phase 7 — UI restructure: from one page to a period workspace ◐ 7.1/7.2 shipped

**Problem (as it was — 7.1 and 7.2 have since fixed this).**
`BudgetOverview` rendered **twelve stacked sections** in a single scroll: header, past-period banner,
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

- ~~**Running deficit / surplus table** (`RunningDeficitView`)~~ — **revived**
  in Phase 7.2; it now lives in the Rapporter tab. No longer mothballed.
- ~~**Billing buffer**~~ — **unblocked and working** as of Phase 5.1–5.3 +
  8.8: contributions now accumulate across months and cover the bill when it
  lands. Only the progress *display* remains (6.11).
- **Credit card payment categories** (6.10) — genuinely deferred; low value in a
  debit-dominant market.
- **5.4 cash-based Ready-to-Assign** — deferred with measured reasons; see
  Phase 5.

---

## Concept review — how this compares to YNAB and mainstream budgeting

*Audited 2026-08-14 against `api/src/models/`; scores updated 2026-08-15 as
Phases 5, 7 and 8 landed.*

**The four YNAB rules, scored:**

| Rule | Status | Evidence |
| --- | --- | --- |
| 1. Give Every Dollar a Job | ✅ **Good** | "Att fördela" badge, over-assignment warning, assign nudge |
| 2. Embrace Your True Expenses | ✅ **Works** *(was ❌)* | Carryover accumulates contributions; `CostKind::Recurring` periodises an annual bill to 1/12 per month. Progress display still to do (6.11) |
| 3. Roll With the Punches | ✅ **Good** | "Ta från…" reallocation via `BudgetedFundsReallocated` |
| 4. Age Your Money | ❌ **Not modelled** | → 6.9, and it needs 5.4 first |

**The one structural difference that remains.** YNAB is an *envelope* system on
two axes: category balances persist **and** you budget money that physically
exists. Rusty Budgets now does the first — carryover makes category balances
persist — but still builds the budget from *forecast* income rather than account
balances. That second axis is 5.4, deliberately deferred: the account data is
not currently trustworthy enough to base a budget on (balances unpopulated, four
months stale, duplicate registry entries).

So the app is no longer a pure budget-vs-actual variance tracker, and the buffer
concept from `CLAUDE.md` works. What is left is whether "Att fördela" should
mean *money you have* rather than *money you expect* — a real decision, not a
missing feature.

**Where Rusty Budgets is genuinely better than mainstream tools:** the
tag + auto-`MatchRule` engine with Swedish-localised tokenisation, and the
transfer-pair resolution that correctly models a savings contribution as a
budget event on the *spending* side with the receipt side ignored. Neither YNAB
nor the Swedish incumbents handle bank-export categorisation this well.
