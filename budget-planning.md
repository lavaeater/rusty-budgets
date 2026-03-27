# Budget Planning

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
Tagging should be done by showing the user a transaction at a time and asking the user to tag it. When tagging it, the user should be able to create a new tag or select an existing tag. We have a system to automatically create rules, I want to both visualize the effect of the rules and enable editing of the rules. After a rule is created, it should be run on the remaining non-processed transactions before moving on to the next transaction. OH, and I think we should connect the concept of periodicity to the transaction right here as well - but should it perhaps be on  the TAG and nothing else? This is where I need guidance. 

### Visualizing Rules

This is basically just a "show all other transactions that match this rule" when we tag a transaction.

### Editing Rules

The rules are basically just a vector of strings that match the transaction description. The editing part of it should be text fields to edit every specific string in that vector, or remove it altogether by simply pressing a button. A rule tags a transaction, nothing else, if it matches. 

- [ ] Decide on  how and where to put periodicity for transactions / tags
- [ ] Move through transactions in a one-by-one fashion
- [ ] Tag a transaction (either new or existing tags)
- [ ] Create a rule based on the transaction information
- [ ] Show all other transactions that match this rule
- [ ] Edit a rule
- [ ] Run a rule on remaining transactions
- [ ] Move to next transaction

## Creating Budget Items

After we have tagged all the transactions, we can then move on to creating budgeting items. This is similar to what we are doing now.
A Budget Item can be associated with several tags. This means that we could have a Budget Item called "Transport" that has the tags "Car", "Buss pass" and "Train pass". 

- [ ] Decide on how and where to put periodicity for budgeting items - see transactions above
- [ ] Using a suggested monthly income, create budget items until the entire income is "spent" AND all tags are associated with a BudgetItem
- [ ] Display an average monthly expenditure for each tag not yet budgeted
- [ ] A Budget Item can be Income, Expense, Savings
- [ ] A Budget Item can have one or more tags associated with it

## Considerations

What about the time factor. Perhaps a tag is removed - it should not be removed historically. 

## Cascade Plan

### Current State Assessment

What already exists in the codebase:
- `BudgetItem` has `tags: Vec<String>` and `periodicity: Periodicity` (Monthly/Quarterly/Annual)
- `BankTransaction` has no tag field — tags are only on BudgetItems/ActualItems
- `MatchRule` exists with `transaction_key` (tokenized description) and `item_key` (tokenized item name)
- `create_rule` and `evaluate_rules` are wired up — rules auto-allocate matching transactions
- `TransactionAllocation` has a `tag: String` field but it is not used in a structured way
- Import is functional; no dedicated "tagging workflow" UI or API exists yet

### Decision Required: Periodicity Placement

**Recommendation: Put periodicity on the Tag entity, not on the transaction or budget item.**

Rationale: A tag like "Electricity" is inherently a monthly expense, while "Dog Insurance" is annual. This is a property of the category, not of a specific transaction or budget item. A `BudgetItem` can then inherit or aggregate from its associated tags. The existing `periodicity` on `BudgetItem` can remain as an override/override field.

Action needed: Confirm or override this recommendation before implementing the Tag model.

### Phase 1 — Tag Model

- [ ] Create a `Tag` struct with `id: Uuid`, `name: String`, `periodicity: Periodicity`
- [ ] Add `tags: Vec<Tag>` to the `Budget` struct (replacing ad-hoc string tags)
- [ ] Add events: `TagCreated`, `TagModified` (soft-delete only — never hard-delete tags)
- [ ] Add DB/API functions: `create_tag`, `get_tags`, `modify_tag`
- [ ] Replace `tags: Vec<String>` on `BudgetItem` with `tag_ids: Vec<Uuid>` (no data migration needed — greenfield data)

### Phase 2 — Transaction Tagging Workflow (API)

- [ ] Add `tag_id: Option<Uuid>` to `BankTransaction` to record which tag a transaction was assigned
- [ ] Add API fn `get_next_untagged_transaction(budget_id)` — returns first transaction with no tag and not ignored
- [ ] Add API fn `tag_transaction(budget_id, tx_id, tag_id)` — tags the transaction, auto-creates a `MatchRule`, runs `evaluate_rules` on remaining untagged transactions
- [ ] Add API fn `preview_rule_matches(budget_id, tx_id)` — returns all other transactions that would be matched by the auto-generated rule for this transaction
- [ ] Add API fn `update_rule(budget_id, rule_id, transaction_key: Vec<String>)` — allows editing the string tokens of an existing rule

### Phase 3 — Transaction Tagging Workflow (UI)

- [ ] Build a "Tag Transactions" page/view: shows one untagged transaction at a time
- [ ] UI: select existing tag or type a new tag name (with periodicity picker) to create one inline
- [ ] UI: after tagging, display matching transactions panel ("X other transactions match this rule")
- [ ] UI: allow editing each token in the rule's `transaction_key` vector inline, with a remove button per token
- [ ] UI: "Skip" and "Ignore" buttons per transaction (ignored transactions are already supported in the model)
- [ ] UI: progress indicator — N transactions remaining to tag

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
