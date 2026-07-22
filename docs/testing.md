# Testing plan for Rusty Budgets

This document is the plan for building a comprehensive, layered test suite for
Rusty Budgets. It mirrors the approach we landed in the sibling `oxidian`
project (and the official Dioxus 0.7/0.8 testing guide,
<https://dioxuslabs.com/learn/0.7/guides/testing/web>, plus its reference
examples in `packages/playwright-tests/{web,fullstack}` of the Dioxus repo),
adapted to Rusty Budgets' architecture: a **fullstack** Dioxus app with
`#[server]` functions over a **CQRS + event-sourced** domain backed by JoyDB.

The single most important architectural fact for testing: **the whole domain is
event-sourced through one aggregate (`Budget`) and driven by an in-memory
runtime (`JoyDbBudgetRuntime::new_in_memory()`)**. That makes almost all of our
business logic testable natively with `cargo test`, *without a browser, a
server, or the network* — the biggest lever we have. We should saturate that
layer before spending effort higher up the pyramid.

## Goals

1. **Test our own code** — the domain logic (aggregate replay, rule matching,
   tokenizer, money math, projections) and our components/views in `ui`, not the
   Dioxus framework or the vendored `dioxus-primitives` wrappers.
2. **Regression safety** — event sourcing means a bug usually shows up as a
   *wrong projection after replay*. Command→event→replay→assert-projection tests
   are the natural regression gate and should exist for every command.
3. **Runs fast in CI** — the domain and component layers run under `cargo test`
   on the native target with no external dependencies. E2E (Playwright vs a real
   `dx` web build) is the small, slow, high-value top.

## The testing pyramid for Rusty Budgets

```
        ┌───────────────────────────────────────────────┐
   E2E  │ Playwright vs `dx run --package web`           │  few, slow, high-value
        │ real browser, real server fns, real projection │  whole-flow regression
        ├───────────────────────────────────────────────┤
 Comp.  │ dioxus-ssr render + pretty_assertions          │  render a view with
        │ + VirtualDom driving for hooks/interaction     │  fixed props, assert HTML
        ├───────────────────────────────────────────────┤
 CQRS   │ JoyDbBudgetRuntime::new_in_memory()            │  command → event →
        │ command → replay → assert projection           │  replay → assert
        ├───────────────────────────────────────────────┤
 Unit   │ plain `#[test]` on pure functions              │  many, fast, exhaustive
        │ money / tokenizer / dates / periods / …        │  edge-case coverage
        └───────────────────────────────────────────────┘
```

The guiding rule, same as oxidian: **push logic down**. Anything expressible as
a pure function gets a unit test (cheap, exhaustive) so the CQRS layer only
proves event wiring, the component layer only proves render wiring, and E2E only
proves the whole thing is plumbed together.

---

## Layer 1 — Unit tests (pure logic)

Plain `#[cfg(test)] mod tests` next to the code, run with `cargo test -p api`.
This layer is partly built and should be saturated first.

### Already covered (keep, extend)
- `api/src/models/money.rs` (16) — the money math is well covered; keep it that
  way, it's the code most likely to silently corrupt data.
- `api/src/models/budget_period_id.rs` (39) — period arithmetic.
- `api/src/models/{budgeting_type,transaction_allocation,tag,user,match_rule,`
  `actual_item,month_begins_on}.rs` — small, keep.
- `api/src/holidays.rs` (5), `api/src/time_delta.rs` (4).

### Gaps to fill (high priority — pure and currently thin/untested)
| Module | Function(s) | What to assert |
| --- | --- | --- |
| `models/match_rule.rs` | `tokenize_description`, `tokenize_description_with_stopwords` | Swedish stopword + date filtering, casing/punctuation, empty/one-token descriptions, that dates and numbers are dropped — table-driven, this is the heart of auto-categorisation |
| `models/match_rule.rs` | `MatchRule` matching + `Hash`/`Eq` | a rule matches the right tokens, near-miss doesn't match; dedup semantics for the `HashSet<MatchRule>` |
| `models/money.rs` | currency-mismatch **panic** paths | assert the panic (`#[should_panic]`) so the "never mix currencies" invariant is a test, not a comment |
| `models/budget.rs` | `evaluate_tag_rules` (aggregate-pure, l.370) | given tagged + untagged txns and a rule set, returns exactly the `(tx, tag)` pairs that should auto-apply; ignored txns excluded |
| `models/budget.rs` | `potential_internal_transfers` (l.554) | same-abs-amount / different-account / within-3-days detection; boundary at exactly 3 days; same-account excluded; already-resolved excluded |
| `models/budget_period.rs` | period bounds vs `MonthBeginsOn` | a txn date maps to the right period when the month doesn't begin on the 1st |
| `models/budget_item.rs` | periodicity override vs tag periodicity | the override resolves per the CLAUDE.md rule (tag is canonical unless overridden) |
| `import/mod.rs` | `import_from_skandia_excel_sync` (l.148, already sync!) | parse a small fixture `.xlsx` from `test_data/` into the expected `BankTransaction`s: amounts, dates, descriptions, account; malformed rows handled |

### Conventions
- Unit tests live in-file under `#[cfg(test)] mod tests`.
- Prefer table-driven cases for the tokenizer and period math.
- No `dioxus` import at this layer.

---

## Layer 2 — CQRS / aggregate tests (in-memory runtime)

This is the layer that best fits this codebase and where most new coverage
should go. `api/tests/mod.rs` already has **27 tests** using
`JoyDbBudgetRuntime::new_in_memory()` — that harness is exactly right; the job
is to systematically extend it so **every command and every event** is covered.

### The pattern (already in use)
```rust
let rt = JoyDbBudgetRuntime::new_in_memory();
let budget_id = rt.create_budget(user_id, "Test", true, MonthBeginsOn::default(), Currency::SEK)?;
// issue a command …
rt.tag_transaction(budget_id, tx_id, tag_id, /* … */)?;
// reload the aggregate (forces a full replay from the event log) …
let budget = rt.load(budget_id)?;
// assert on the rebuilt state / projection …
assert_eq!(budget.items.len(), 1);
```

### Coverage matrix — one command/event pair per row
There are 23 event types in `api/src/events/` and ~40 server functions in
`api/src/lib.rs`. Build a test per **event** (the durable contract) asserting:
1. the command emits the expected event(s),
2. replaying the log rebuilds the expected aggregate state,
3. the derived `BudgetViewModel` projection reflects it,
4. **round-trip**: `serde_json` serialize→deserialize the `Budget` is stable
   (the existing `create_budget_test` already does this — do it everywhere,
   it's the event-store durability guarantee).

Priority events to lock down (mutation-heavy, correctness-critical):
- `transaction_added`, `transaction_tagged`, `transaction_untagged`,
  `transaction_ignored`, `transaction_connected` — the tagging workflow core.
- `rule_added`, `rule_modified`, `rule_deleted` — auto-categorisation; assert
  `apply_all_rules` / `evaluate_tag_rules` re-run gives the right tags.
- `tag_created`, `tag_modified` — and the **soft-delete invariant**: a
  "deleted" tag sets `deleted: bool` and is *never* removed from the log or the
  `tags` registry (assert the tag is still replayable/historically present).
- `item_added`, `item_modified`, `item_buffer_set` — budget item creation.
- `allocation_created`, `allocation_deleted`, `actual_added`, `actual_modified`,
  `actual_funds_adjusted`, `actual_funds_reallocated`.
- `transfer_pair_rejected` + `resolve_transfer_pair` — both resolution paths
  from CLAUDE.md: float (tag_id None → both ignored) vs savings (tag_id Some →
  outgoing tagged, incoming ignored). This is subtle and worth two explicit
  tests.

### Projection tests (`view_models/`)
`BudgetViewModel`, `BudgetItemViewModel` and friends are read-optimised
projections. Assert the caps documented in CLAUDE.md are enforced:
- `potential_transfers` capped at 10 while `potential_transfer_count` carries
  the true total.
- `untagged_transaction_count` matches the real untagged set (excluding
  ignored + transfer-pair candidates).
- `get_untagged_transactions(budget_id, limit)` respects `BATCH_SIZE` and
  excludes ignored/transfer txns.

### Conventions
- These live in `api/tests/mod.rs` (or split into
  `api/tests/{tagging,rules,items,transfers}.rs` as it grows — the file is
  already large enough to justify splitting by workflow).
- Always `rt.load()` (full replay) before asserting, never inspect intermediate
  state — replay-correctness is the whole point of event sourcing.

---

## Layer 3 — Component & view tests (dioxus-ssr + VirtualDom)

Native `cargo test` that renders real Dioxus components to a string and asserts
on the output — the guide's "component testing" approach, exactly as bootstrapped
in oxidian.

### Setup
Add to `ui/Cargo.toml` (and `api` if we SSR anything server-side) as
`[dev-dependencies]`:
```toml
[dev-dependencies]
dioxus-ssr       = "0.7.6"   # match the workspace dioxus version
pretty_assertions = "1"
futures          = "0.3"
```
Reusable harness (copy from oxidian's `app/tests/ssr_smoke.rs`):
```rust
fn render(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}
```
For a single component with props, `dioxus_ssr::render_element(rsx! { … })`.

### Which components to test (`ui/src/budget/`)
Focus on **our** view components with real logic, not the vendored primitives in
`ui/src/components/*` (accordion/dialog/select/… are upstream's to test).
Priorities — each rendered with representative props, asserting the important
structural output and that the right data reaches the DOM:
- `tag_transactions_view.rs` — the tagging loop UI; assert a batch of untagged
  txns renders, the tag-chip picker shows existing tags, the inline
  periodicity picker appears on "create tag".
- `create_budget_items_view.rs` / `new_budget_item.rs` / `item_selector.rs` —
  the budget-item creation workflow (Phase 4–5); assert unbudgeted tag summaries
  render with their computed monthly averages.
- `rules_view.rs` — editable rule tokens render, each token individually
  removable.
- `transactions_view.rs`, `budget_item_status_view.rs`,
  `budgeting_type_overview_view.rs`, `budget_hero.rs` — smoke-render with fixed
  props and assert key figures/labels appear.
- `retag_transactions_view.rs`, `tags_view.rs`.

> **Note on interaction depth:** `dioxus-ssr` renders one `rebuild_in_place`
> pass — initial state only. Effects, `document::eval`/JS, and event handlers do
> **not** run. Use this layer for *render correctness* and prop→DOM wiring. True
> click-through flows belong in Layer 4. (Same caveat we hit in oxidian.)

### Hooks
Any `use_*` hook whose logic lives in Rust can be tested by mounting a probe
component, driving the `VirtualDom`, and asserting on what the hook produced
(`VirtualDom::new_with_props`, `rebuild_in_place`, `render_immediate`,
`mark_dirty`). Hooks whose logic lives in JS (`document::eval`) can't fire
natively — those belong to Layer 4.

---

## Layer 4 — End-to-end tests (Playwright)

Mirrors oxidian's `e2e/` and `packages/playwright-tests/fullstack` from the
Dioxus repo: a Playwright project that boots the app with `dx run` and drives a
real Chromium. This proves *"change one feature, everything else stays whole"*
across the real server-function boundary.

**Key difference from oxidian:** Rusty Budgets is **fullstack**, so `dx run`
starts the server functions too, and they run against a **real JoyDB file**.
That means the auth/network mocking oxidian needed is *not* required — but test
isolation is. Strategy:

- Point the server at a **throwaway data file** per run (JoyDB writes
  `data.json`; give the E2E server its own dir / env so it never touches the
  developer's real `data.json` — note the repo ships `data.json`/`data.bak` and
  `test_data/`). Seed it from a fixture at suite start and reset between specs.
- Because the domain is event-sourced, a fixture is just a **prebuilt event
  log** (or a small import) — replaying it gives a deterministic starting state.

### Structure
Create a top-level `e2e/` (copy the scaffold from oxidian's `e2e/`):
```
e2e/
  package.json            # @playwright/test devDependency
  playwright.config.js     # webServer: dx run --package web, port 8080
  tests/
    helpers.js             # seed the throwaway DB, reset between specs
    import.spec.js
    tagging.spec.js
    rules.spec.js
    budget-items.spec.js
    transfers.spec.js
```
`playwright.config.js` — same shape as oxidian's, but the `webServer.command`
builds the **fullstack** web app:
```js
command: `dx run --package web --addr 127.0.0.1 --port ${PORT} --hot-reload false`,
cwd: "..",
timeout: 10 * 60 * 1000,  // first wasm+server build is slow on a cold target
```
Use `dx run` (not `dx serve`) so there's no rebuild/hot-patch overlay racing the
tests — the lesson from oxidian.

Locate elements by adding stable `class` / `id` / `data-testid` hooks in the RSX
(the Dioxus examples locate by `button.increment-button`, `#main`, etc.). We'll
need to add a few of these to the budget views.

### Regression-critical flows to cover
Follow the documented product workflow (**Setup → Import → Tag & Rules → Create
Items → Day-to-Day**):
- **Import:** upload/point at a Skandia `.xlsx` fixture → transactions appear;
  `untagged_transaction_count` reflects them.
- **Tagging loop:** tag a transaction with an existing chip → it leaves the
  untagged batch; create a tag inline with a periodicity → an auto `MatchRule`
  is created and the preview count of matching txns shows; "Skip" advances
  locally, "Ignore" persists across reload.
- **Rules:** edit a rule token / remove a token → re-evaluation retags the right
  transactions.
- **Transfer pairs:** a same-amount cross-account pair is offered; resolving as
  "Intern överföring (float)" ignores both; resolving as "Sparande →" with a
  savings tag tags the outgoing side and ignores the incoming — assert the
  resulting budget event is on the spending side (the CLAUDE.md model).
- **Budget-item creation:** enter a suggested income → unbudgeted tags show with
  monthly averages → group tags into a named item with a type → item persists.
- **Persistence across reload** — event log replays to the same projection.

---

## Cross-cutting: what "stays whole" means mechanically

To make regressions *fail a test* rather than get noticed in review:

1. **Event-log round-trip** — every CQRS test serialises→deserialises the
   `Budget` (durability of the store); the existing `create_budget_test` is the
   template.
2. **Projection assertions** — every mutation test asserts the `BudgetViewModel`
   the UI actually consumes, so a projection regression fails a test, not a demo.
3. **Snapshot the tokenizer** — golden cases (consider `insta`) for
   `tokenize_description`, so any change to Swedish tokenisation shows a
   reviewable diff — it silently changes auto-categorisation otherwise.
4. **The E2E suite is the integration contract** — every user-facing workflow
   step gets at least one spec; altering a feature must consciously update its
   spec.

---

## Tooling & CI

### Local commands
```sh
cargo test -p api              # unit + CQRS/aggregate (native, no browser/server)
cargo test -p ui               # component/view render tests
cargo test --workspace         # everything native
cargo clippy --workspace       # lint

cd e2e && npm test             # E2E (auto-starts `dx run --package web`)
```

### New dev-dependencies to add
- `ui`: `dioxus-ssr`, `pretty_assertions`, `futures`.
- `api`: `insta` (tokenizer/projection snapshots), optionally `pretty_assertions`.
- `e2e/`: `@playwright/test`.

### CI (GitHub Actions) — suggested jobs
1. `cargo test --workspace` + `cargo clippy --workspace` (fast, always). This
   alone covers Layers 1–3 with no external deps.
2. `playwright` job: install the `dx` CLI, `cargo build`, run `npx playwright
   test` against a throwaway data dir; upload the HTML report + traces on failure.

---

## Rollout order (recommended)

1. ✅ **Saturate the CQRS layer (Layer 2)** — one test per event in
   `api/src/events/`, each with replay + projection + round-trip assertions.
   *Done: every event type has a test; added transfer-detection boundaries,
   transfer-pair resolution semantics, and an event-store round-trip test.*
2. ✅ **Fill the pure-logic gaps (Layer 1)** — *Done for tokenizer (table-driven),
   `potential_internal_transfers` boundaries, and the money panic paths.* Still
   open: the Skandia import fixture detail and `MonthBeginsOn` period bounds.
3. ✅ **Component render harness (Layer 3)** — *Bootstrapped: `dioxus-ssr` +
   `pretty_assertions` dev-deps added to `ui`; `ui/tests/component_render.rs`
   holds the `render_element` helper and the context-provider pattern.* Still
   open: the heavy workflow views (`tag_transactions_view`, budget-item creation).
4. ✅ **Playwright scaffold (Layer 4)** — *Done & green (2 tests): `e2e/` holds
   the Playwright project with the throwaway-SQLite strategy (`DATABASE_URL` +
   empty `DATA_FILE`) wired into `webServer.command`, and an `onboarding` spec
   (fresh-DB create-budget screen → overview). It immediately caught and we fixed
   a real first-budget-creation panic in the SQL runtime.* Next: import →
   tagging → items.
5. ◐ **Snapshots + CI wiring** — *CI done: `.github/workflows/{ci,e2e}.yml`
   (fast native gate + the Playwright job).* Snapshots (`insta`) still open.
   Then broaden E2E workflow-by-workflow.

## Implementation status

- **Layer 1 (pure units) — in progress.**
  - ✅ `money` — arithmetic, display, ordering, hashing, and **all four**
    currency-mismatch panics (`+`, `-`, `*`, `+=`, `-=`) asserted with
    `#[should_panic]`.
  - ✅ `budget_period_id` (39) — period arithmetic.
  - ✅ **Tokenizer** (`models/match_rule.rs`) — table-driven coverage of
    lowercasing, ISO/slash/compact-`YYYYMMDD` date stripping, stopword +
    place-name filtering (incl. the "punctuation defeats the place-name match"
    edge), the "numbers that aren't dates survive" rule, empty/whitespace input,
    `is_date_pattern`, custom stopwords, and `MatchRule::matches_transaction`
    (all-tokens-present / near-miss / empty-key). 5 new tests.
  - ⬜ Skandia import fixture beyond the existing 296-row smoke; period-bounds vs
    `MonthBeginsOn`; `budget_item` periodicity override.
- **Layer 2 (CQRS/aggregate) — largely saturated.** `api/tests/mod.rs` now has
  **34 tests** on `JoyDbBudgetRuntime::new_in_memory()`, with at least one test
  per event type. Added this pass:
  - ✅ `potential_internal_transfers` boundaries — matching pair detected; the
    3-day window (inclusive at 3, excluded at 4); different-account requirement;
    rejected pairs excluded.
  - ✅ **Transfer-pair resolution semantics** (the CLAUDE.md savings model) —
    *float* (both sides ignored, neither tagged) and *savings* (outgoing/spending
    side tagged, incoming/receipt side ignored) asserted against the runtime.
  - ✅ **Event-store durability** — a multi-event workflow (tag/item/actual/
    transaction/rule) is replayed, serialised, deserialised, and proven
    byte-stable (`to_string(restored) == to_string(loaded)`).
  - ⬜ Still worth adding: `serde` round-trip assertions on the *remaining*
    per-event tests, and explicit projection-cap tests (`potential_transfers`
    capped at 10 with the true `potential_transfer_count`).
- **Layer 3 (component/view) — bootstrapped.** `dioxus-ssr` +
  `pretty_assertions` are dev-deps on `ui`. `ui/tests/component_render.rs` (7
  tests) establishes two reusable patterns:
  - `dioxus_ssr::render_element(rsx! { … })` for a pure-props component —
    proven on `BudgetingTypeOverviewView` (label + all three money figures reach
    the DOM; the progress-bar width is `actual/budgeted`, caps at 100%, and gets
    the `over` modifier + `over-budget`/`warning` classes when over).
  - A `StatusHarness` component that installs `BudgetState` via
    `use_context_provider`, so `use_context`-consuming views can be rendered —
    proven on `BudgetItemStatusView` (Balanced→empty, NotBudgeted→indicator,
    OverBudget→auto-adjust button). `BudgetState` was exported from
    `ui::budget` to make this reachable from the test crate.
  > A single `rebuild_in_place`/`render_element` pass renders **initial state
  > only** — effects, `document::eval` JS, and event handlers don't run, so
  > interaction (button clicks, the take-from picker) stays in Layer 4.
- **Layer 4 (Playwright E2E) — green (2 passing).** `e2e/` holds a working
  Playwright project (`npm test`). Because Rusty Budgets is *fullstack* over a
  SQL DB (welds/SQLite via `DATABASE_URL`), the harness needed a **real server +
  a throwaway database**, not the localStorage/route-mocking a client-only app
  would use. The `webServer.command` runs the `api` migration binary against a
  per-run temp SQLite file (with `DATA_FILE` pointed at an empty file so the
  binary's JoyDB→SQL copy is a no-op → a clean, empty, migrated DB), then boots
  `dx serve -p web --fullstack true --web --hot-reload false`. On first request
  the server auto-creates the default user; a budget-less DB boots into the
  "Ingen budget hittad" screen. Spec: `onboarding.spec.js` (`describe.serial`) —
  the create-budget screen renders on a fresh DB, and creating a budget lands on
  the overview (name heading + "Verktyg" tools).
  > **Bug surfaced by the onboarding spec (now fixed):** creating the *first*
  > budget for any user panicked server-side in `add_budget_to_user`
  > (`api/src/cqrs/runtime.rs`) — the freshly-initialised `user_budgets` row
  > serialised the whole `UserBudgets` struct into the `budgets` JSON column
  > instead of just its `Vec<(Uuid, bool)>`, so the immediate read-back failed
  > with *"invalid type: map, expected a sequence"* (`pg_models.rs:147`). Two
  > write sites were storing the struct-shaped value; both now serialise the bare
  > vec, matching `From<UserBudgets> for DbState<PgUserBudgets>`. The in-memory
  > JoyDB unit/CQRS tests never exercised the SQL runtime, so **only the
  > fullstack E2E could catch this** — a textbook case for the top of the pyramid.
  > A targeted SQLite integration test for the `PgRuntime` (`sqlite::memory:` +
  > `migrations::up`) is the recommended follow-up to guard it at the fast layer.
  Next: import a Skandia fixture, the tagging loop, transfer-pair resolution,
  budget-item creation.
- **CI — wired.** `.github/workflows/ci.yml` runs `cargo test --workspace` +
  `cargo clippy -D warnings` (the fast Layers 1–3 gate); `e2e.yml` installs
  `dx` + Node + Playwright and runs the E2E suite, uploading the HTML report.

## Non-goals
- Testing the vendored `dioxus-primitives` components in `ui/src/components/*`
  (upstream's responsibility) — we only test *our* views' usage of them.
- Testing the Dioxus framework or JoyDB itself.
- Multi-user / multi-currency scenarios (single-user, single-currency app for
  now — currency mixing is a *panic* we assert against, not a feature to test).
