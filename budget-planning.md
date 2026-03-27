# Budget Planning

## My Notes

**Next up: Phase 4 — Budget Item Creation Workflow (API)**

Phases 1–3 are complete. The tagging workflow works end-to-end, including transfer pair resolution. Start Phase 4 by implementing `get_unbudgeted_tags` and `get_average_monthly_expenditure_per_tag`.

## Ambition Elevation

So, thinking over how I want to work with the budget, I have come to some conclusions.
I think the flow I want to use is Setup -> Import -> Tag and Create Rules -> Create Budget Items -> Day-to-Day Budget Work.

## Setup

Setup is nothing complicated, it is just creating a budget with name and decide on currency and 1st day of month and stuff like that. There are defaults in place now, so nothing needs to be done now, however future things to do are:

- [ ] Decide on 1st day of month scheme for Budget
- [ ] Decide on default currency

## Import

The Import needs some reworking. I came to the conclusion now that I need to analyze expenditure over time to be able to create a good budget.
A lot of Budget Items fluctuate over time and are also not necessarily monthly expenditures, things like water is once a year, electricity is once a month but the consumption and prices fluctuate over time.
A concrete example is that I felt our budget was under control - but power consumption was way above what I expected and the price was way above what I expected - so a lot of money went into that hole. Instead the monthly budget should be gearing towards building up a billing buffer so that the monthly budget can "take" the hit of a spike in bills.
This would mean that for yearly expenditures, money should be put aside for that future cost (like dog insurance, once a year). The yearly cost of electricity should be used to set up a billing buffer that is used to cover the monthly budget - in the summer time the cost of electricity is lower and that just means that the buffer builds up faster.
One should be able to basically front-load the budget so that when the buffer is where it should be, less money is put aside every month.

So the Import at this stage should be of all transactions since 1st of january 2024, for instance, and for the most important bank accounts.

Thinking about it, I think the actual import is probably where it needs to be right now.

The rub comes in the next step!

## Tagging and Creating Rules

After importing the transactions, they should be tagged and rules should be created.
Tagging should be done by showing the user a transaction at a time and asking the user to tag it. When tagging it, the user should be able to create a new tag or select an existing tag. We have a system to automatically create rules, I want to both visualize the effect of the rules and enable editing of the rules. After a rule is created, it should be run on the remaining non-processed transactions before moving on to the next transaction.

Periodicity decision: **periodicity lives on the Tag**, not on the transaction or budget item. A tag like "Electricity" is inherently monthly; "Dog Insurance" is annual. BudgetItem has a `periodicity` field as an override only.

### Transfer Pair Model

Savings contributions (spending account → savings account) are handled separately from regular tagging:
- The outgoing (spending) side is tagged with a savings tag — this IS the budget event.
- The incoming (savings receipt) side is ignored — it is just confirmation of where the money went.
- True float transfers (spending → bills account pre-funding) ignore both sides.

This is implemented via `resolve_transfer_pair(budget_id, outgoing_tx_id, incoming_tx_id, tag_id: Option<Uuid>)`.

- [x] Decide on how and where to put periodicity for transactions / tags
- [x] Move through transactions in a one-by-one fashion (batches of 10)
- [x] Tag a transaction (either new or existing tags)
- [x] Create a rule based on the transaction information
- [x] Show all other transactions that match this rule
- [x] Edit a rule
- [x] Run a rule on remaining transactions
- [x] Move to next transaction
- [x] Handle internal transfer pairs (savings vs float resolution)

## Creating Budget Items

After we have tagged all the transactions, we can then move on to creating budgeting items. This is similar to what we are doing now.
A Budget Item can be associated with several tags. This means that we could have a Budget Item called "Transport" that has the tags "Car", "Buss pass" and "Train pass".

- [ ] Using a suggested monthly income, create budget items until the entire income is "spent" AND all tags are associated with a BudgetItem
- [ ] Display an average monthly expenditure for each tag not yet budgeted
- [ ] A Budget Item can be Income, Expense, Savings
- [ ] A Budget Item can have one or more tags associated with it

## Considerations

What about the time factor. Perhaps a tag is removed - it should not be removed historically.

## Cascade Plan

### Current State (as of end of session 2026-03-27)

What is built:
- `Tag` struct with `id: Uuid`, `name: String`, `periodicity: Periodicity`, `deleted: bool`
- `tags: Vec<Tag>` on `Budget`; events `TagCreated`, `TagModified`; API: `create_tag`, `get_tags`, `modify_tag`
- `BudgetItem.tag_ids: Vec<Uuid>` (references tags by ID)
- `BankTransaction.tag_id: Option<Uuid>`, `ignored: bool`
- Full tagging workflow UI (`TagTransactionsView`) integrated in `BudgetOverview`
- Batched fetching: `get_untagged_transactions(budget_id, limit: usize)` returns at most 10
- `BudgetViewModel` has `untagged_transaction_count` (full count) and `potential_transfer_count` (full count), `potential_transfers` capped at 10
- Transfer pair resolution: `resolve_transfer_pair` API + `TransferPairCard` UI with savings/float split
- API: `tag_transaction`, `preview_rule_matches`, `update_rule`, `ignore_transaction`, `resolve_transfer_pair`

### Phase 1 — Tag Model ✅

- [x] Create a `Tag` struct with `id: Uuid`, `name: String`, `periodicity: Periodicity`
- [x] Add `tags: Vec<Tag>` to the `Budget` struct
- [x] Add events: `TagCreated`, `TagModified` (soft-delete only — never hard-delete tags)
- [x] Add DB/API functions: `create_tag`, `get_tags`, `modify_tag`
- [x] Replace `tags: Vec<String>` on `BudgetItem` with `tag_ids: Vec<Uuid>`

### Phase 2 — Transaction Tagging Workflow (API) ✅

- [x] Add `tag_id: Option<Uuid>` to `BankTransaction`
- [x] Add API fn `get_next_untagged_transaction(budget_id)`
- [x] Add API fn `get_untagged_transactions(budget_id, limit)` — batched, excludes transfer pair members
- [x] Add API fn `tag_transaction(budget_id, tx_id, tag_id)` — tags, auto-creates MatchRule, runs evaluate_tag_rules
- [x] Add API fn `preview_rule_matches(budget_id, tx_id)`
- [x] Add API fn `update_rule(budget_id, rule_id, transaction_key: Vec<String>)`
- [x] Add API fn `resolve_transfer_pair(budget_id, outgoing_tx_id, incoming_tx_id, tag_id: Option<Uuid>)`

### Phase 3 — Transaction Tagging Workflow (UI) ✅

- [x] Build `TagTransactionsView`: shows one untagged transaction at a time from a batch of 10
- [x] UI: select existing tag or type a new tag name (with periodicity picker) to create one inline
- [x] UI: after tagging, display matching transactions panel ("X other transactions match this rule")
- [x] UI: allow editing each token in the rule's `transaction_key` vector inline
- [x] UI: "Skip" and "Ignore" buttons per transaction
- [x] UI: progress indicator showing batch position and total remaining
- [x] UI: `TransferPairCard` with "Intern överföring (float)" and "Sparande →" resolution paths
- [x] `TagTransactionsView` and `TransferPairsView` integrated in `BudgetOverview`

### Phase 4 — Budget Item Creation Workflow (API)

- [ ] Add API fn `get_unbudgeted_tags(budget_id)` — returns tags not yet associated with any BudgetItem
- [ ] Add API fn `get_average_monthly_expenditure_per_tag(budget_id)` — computes average monthly spend per tag from all imported transaction history
- [ ] Add API fn `create_budget_item_with_tags(budget_id, name, budgeting_type, tag_ids, suggested_amount)` — creates item and associates tags

### Phase 5 — Budget Item Creation Workflow (UI)

- [ ] Build a "Create Budget Items" guided view:
  - User enters a suggested monthly income at the top
  - Shows running total of budgeted vs. income
  - Lists unbudgeted tags with their computed average monthly expenditure
  - User selects tags and groups them into a new BudgetItem with a name and type (Income/Expense/Savings)
- [ ] Stop condition: all tags budgeted AND total budgeted amount equals suggested income

### Phase 6 — Billing Buffer (Deferred / Future)

- [ ] Add a `buffer_target: Option<Money>` field to `BudgetItem` for items that need a rolling buffer (e.g. electricity, yearly bills)
- [ ] Add logic to compute required monthly contribution to reach buffer target based on periodicity and historical average
- [ ] Visualize buffer fill level in the day-to-day budget view
