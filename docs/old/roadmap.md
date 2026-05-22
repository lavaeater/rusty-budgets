# Current Roadmap

Issues with current auto_budget_period

The current implementation (db.rs:312) only sets budgeted_amount = actual_amount for actuals that already exist and
have zero budget. It doesn't create missing actuals for budget items that have transactions in that period but no      
ActualItem yet — so it silently skips them. For old periods this is the main problem: actuals need to be created first.
                                                                                                                         
---                                                                                                                    
Recommendations

1. Fix auto_budget_period to also create missing actuals

For each BudgetItem, check if an ActualItem exists in that period — if not, create one (via add_actual) before setting
the budgeted amount. The budget has items and the period has actual_items; cross-reference by budget_item_id. The
actual amount should then be derived from the sum of transaction allocations for that item in that period.

2. Auto-budget all periods (auto_budget_all)

A new server function that iterates budget.periods, skips any period where every ActualItem already has a non-zero     
budgeted_amount (i.e. "already properly budgeted"), and calls the fixed auto_budget_period logic on the rest. "Properly
budgeted" = has at least one actual with a nonzero budgeted amount. This gives you the "don't touch months I've       
already touched" behavior.

3. Running deficit tracking

The cleanest approach given your data model:

- Compute per-period net = sum of Income actuals (actual_amount) − sum of Expense + Savings actuals (actual_amount),   
  per period
- Accumulate over time (sorted by PeriodId) → running deficit/surplus Money value
- Resolution: Add a DeficitResolved event with a source tag (e.g. a Savings or TaxReturn BudgetItem) and an amount,    
  which offsets the running total. Without resolution it just accumulates

For the UI: a simple timeline table showing each month's net and the running cumulative, with an "unresolved deficit"  
banner if the current value is negative.
                                                                                                                         
---             
Want me to start implementing? I'd suggest this order:
1. Fix auto_budget_period to create missing actuals
2. Add auto_budget_all server function
3. Add running deficit calculation to BudgetViewModel (no new events needed yet, just a computed field)

✻ Cogitated for 2m 7s                           