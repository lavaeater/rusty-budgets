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
