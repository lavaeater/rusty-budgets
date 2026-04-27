# Rusty Budgets — Claude Instructions

## Project Overview

A full-stack personal budgeting application written entirely in Rust, using Dioxus for the UI layer. Supports web, desktop, and mobile from a shared codebase. The intended workflow is:

**Setup → Import → Tag & Create Rules → Create Budget Items → Day-to-Day Budget Work**

## Tech Stack

- **Rust 2024 Edition**, multi-crate workspace
- **Dioxus 0.7.3** — fullstack framework (React-like, used for web/desktop/mobile)
- **JoyDB** — custom JSON/RON file-based database (no SQL)
- **CQRS + Event Sourcing** — custom framework in `api/src/cqrs/`
- **Tokio** — async runtime (server-side only)
- **Serde / Chrono / UUID v4 / Calamine** — standard utilities

## Workspace Layout

```
api/        — Domain model, CQRS events, all server functions (#[server])
ui/         — Shared Dioxus UI components
web/        — Web platform entry point
desktop/    — Desktop platform entry point
mobile/     — Mobile platform entry point
cqrs_macros/ — Procedural macros for event enum generation
```

## Architecture: CQRS + Event Sourcing

`Budget` is the single aggregate root. All mutations go through domain events stored in `api/src/events/` (one file per event type). State is rebuilt by replaying events. Never mutate aggregate state directly — always emit an event.

Server functions use Dioxus's `#[server]` macro and live in `api/src/lib.rs`. They return `BudgetViewModel` (a read-optimized projection). The DB layer (`api/src/db.rs`) is a thin wrapper around `JoyDbBudgetRuntime`.

## Key Data Model

### `Budget` (aggregate)
- `items: Vec<BudgetItem>` — budget categories
- `periods: Vec<BudgetPeriod>` — one per calendar period
- `accounts: Vec<BankAccount>`
- `tags: Vec<Tag>` — canonical tag registry
- `match_rules: HashSet<MatchRule>` — auto-categorisation rules
- `currency: Currency`, `month_begins_on: MonthBeginsOn`

### `Tag`
- `id: Uuid`, `name: String`, `periodicity: Periodicity`, `deleted: bool`
- **Periodicity lives on `Tag`, not on `BudgetItem` or `BankTransaction`.** A tag like "Electricity" is inherently monthly; "Dog Insurance" is annual. This is a property of the category.
- Tags are **never hard-deleted** — use `deleted: bool` (soft-delete only). Tags must remain historically intact.

### `BudgetItem`
- `tag_ids: Vec<Uuid>` — references tags by ID (not by name string)
- `periodicity: Periodicity` — override field; tag periodicity is the canonical source
- `budgeting_type: BudgetingType` — Income | Expense | Savings | InternalTransfer
- One BudgetItem can cover multiple tags (e.g. "Transport" = Car + Bus Pass + Train)

### `BankTransaction`
- `tag_id: Option<Uuid>` — set when transaction is tagged during the tagging workflow
- `ignored: bool` — transactions can be ignored; ignored transactions skip rule evaluation
- `actual_id: Option<Uuid>` — link to an ActualItem once categorised

### `MatchRule`
- `transaction_key: Vec<String>` — tokenised description fragments to match
- `tag_id: Option<Uuid>` — tag applied when rule matches
- Tokenisation is Swedish-localised (filters Swedish stopwords and dates)

### `Money`
- Stored as `cents: i64` (minor units). Currency mismatches panic — never mix currencies.

## Tagging Workflow (Phases 1–3 complete)

The tagging workflow is the core UX loop for new users. It is fully implemented.

- Transactions are shown in batches of 10 (`BATCH_SIZE`) fetched via `get_untagged_transactions(budget_id, limit)`. The full count is in `BudgetViewModel.untagged_transaction_count`.
- User picks an existing tag (chip UI) or creates one inline with a periodicity picker.
- On tagging: auto-creates a `MatchRule`, runs `evaluate_tag_rules` on remaining untagged transactions.
- Shows a preview count of other transactions matching the new rule (`preview_rule_matches`).
- Rule tokens are editable inline, each token individually removable (`update_rule`).
- "Skip" (local index advance) and "Ignore" (server-side, permanent) buttons per transaction.
- `TagTransactionsView` is mounted in `BudgetOverview` whenever `untagged_transaction_count > 0`.

### Transfer Pair Resolution

Potential internal transfers (same absolute amount, different accounts, within 3 days) are detected by `Budget::potential_internal_transfers()` and resolved separately from the regular tagging flow — they are excluded from `get_untagged_transactions`.

`BudgetViewModel.potential_transfers` is capped at 10 per response; `potential_transfer_count` carries the true total.

The UI (`TransferPairCard`) offers two resolution paths:
- **"Intern överföring (float)"** → `resolve_transfer_pair(..., tag_id: None)` — ignores both sides
- **"Sparande →"** → tag picker appears; `resolve_transfer_pair(..., tag_id: Some(id))` — tags the outgoing (spending) side with a savings tag, ignores the incoming (savings receipt) side

This correctly models savings contributions: the deduction from the spending account is the budget event; the deposit to the savings account is just the receipt.

## Budget Item Creation Workflow (Phase 4–5, next)

After all transactions are tagged:
- User enters a suggested monthly income
- System shows tags not yet associated with any BudgetItem, with their computed average monthly expenditure
- User groups tags into named BudgetItems with a type (Income/Expense/Savings)
- Goal: all tags budgeted AND total budgeted ≈ suggested income

## Billing Buffer Concept (Phase 6, deferred)

For items with irregular or infrequent bills (electricity, annual insurance): instead of budgeting the exact monthly cost, the budget builds a rolling buffer. The monthly contribution fills the buffer; in cheap months the buffer grows faster. This allows the monthly budget to absorb billing spikes without going over. Not yet implemented — tracked as `buffer_target: Option<Money>` on BudgetItem (future field).

## Conventions

- All IDs are `Uuid` (v4)
- Dates via `chrono::DateTime<Utc>`
- Errors via `thiserror`
- No hard-deletes anywhere in the domain — use soft-delete flags
- Server functions return `BudgetViewModel` for read operations, not raw aggregates
- Default user is `tommie.nygren@gmail.com` (single-user app for now)
- Swedish locale assumptions in tokenisation/matching
