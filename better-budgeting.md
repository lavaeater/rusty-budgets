# Better Budgeting - Code Review & Feature Recommendations

## Tommies New Thoughts

I want to rewrite how we make the budget. 

So, transactions should be tagged when being imported. A transaction can have one and only one tag. 
A budget item is simply the aggregation of a collection of tags - for the actual outcome of what happens on that budget item.

Why the change? Well, instead of connecting transactions to budget items, we simply tag transactions, no more hard connections between transactions and budget items.

This would make it easier to move transactions around, create new tags as you go and also one could make different slices of views for tags. So tags could in effect become almost item-descriptors,
so we could have a "house mortgage" tag, "house rent" tag, "transport" tag, "fuel" tag, we could almost have one tag for every transaction (inefficient).

I also realize that we need to be able to split a transaction into multiple tags, somehow - so a mortgage payment is part interest and part principal, and we need to be able to split that into two tags, i.e. mortgage interest and mortgage principal. Paying off the principal is a form of savings and needs to be tracked.

This all makes our budgetitems more free from the transactions. I still like the BudgetPeriods, but the budget should probably always be considered yearly... so when creating budget items, all items should have a periodicity, monthly, yearly, etc.

Gaah, this isn't it either.

Budget
    - BudgetPeriod
        - BudgetItem
            - ActualItem

Also, we really really need to be able to handle internal transactions. The system doesn't need to, right now, be able to handle other banks formats for exported transaction data, but it should be able to handle internal transactions between owned bank accounts, cancelling them out or... perhaps we mark one account as savings - and if money is transferred to it from another account, it's a deposit, if money is transferred from it to another account, it's a withdrawal.

So, I feel what I have is really good, but not good enough. I want the focus of the software to be working with the budget - importing transactions and modifying budgeted amounts to make sure the budget stays balanced. Suggestions?

Let me explore the current domain model to give you grounded suggestions.

Let me grab a few more pieces to fully understand the current flow.

I've read through the full domain model. Here's my analysis and a concrete proposal.

---

~~# The Missing Concept: `TransactionAllocation`

Your instinct is right — there's a rigid coupling at the heart of the current model. Look at this:

```@/home/tommie/projects/rusty-budgets/api/src/models/bank_transaction.rs:26
    pub actual_id: Option<Uuid>,
```

A transaction points at **exactly one** `ActualItem` (or nothing). This single `Option<Uuid>` is the bottleneck behind every frustration you described: can't split, can't re-tag, can't move things around easily.

## What's Actually Needed

An **allocation layer** between transactions and budget line items:

```
BankTransaction ──1:N──▶ TransactionAllocation ──N:1──▶ ActualItem
```

### `TransactionAllocation`

```rust
pub struct TransactionAllocation {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub actual_id: Uuid,
    pub amount: Money,          // portion of the transaction
    pub tag: String,            // e.g. "mortgage-interest", "mortgage-principal"
}
```

**This single concept solves three of your problems at once:**

1. **Splitting** — A 10,000 kr mortgage payment becomes two allocations: 7,000 kr → "mortgage-interest" (Expense), 3,000 kr → "mortgage-principal" (Savings). The sum of allocations must equal the transaction amount.

2. **Flexible tagging** — Tags live on allocations, not on transactions or items. You tag *how money was spent*, not *where it came from*. A `BudgetItem` then defines which tags it aggregates (e.g. a "Housing" budget item covers tags `["mortgage-interest", "home-insurance", "electricity"]`).

3. **Easy re-categorization** — Moving a transaction = deleting one allocation and creating another. No mutation of the transaction itself. The event log stays clean.

### `BudgetItem` Gets Tag Aggregation + Periodicity

```rust
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub budgeting_type: BudgetingType,
    pub tags: Vec<String>,              // which allocation tags roll up here
    pub periodicity: Periodicity,       // Monthly, Quarterly, Annual
}

pub enum Periodicity {
    Monthly,
    Quarterly,
    Annual,
}
```

Now `ActualItem.actual_amount` becomes a **computed value** — the sum of all `TransactionAllocation`s whose tag matches the item's tag set, in the relevant period. You can still store a cached value for display, but the source of truth is the allocations.

### `ActualItem` Simplifies

```rust
pub struct ActualItem {
    pub id: Uuid,
    pub budget_item_id: Uuid,
    pub period_id: PeriodId,
    pub budgeted_amount: Money,
    // actual_amount is now DERIVED from allocations
    // item_name, budgeting_type are DERIVED from BudgetItem
}
```

No more denormalized `item_name` and `budgeting_type` on `ActualItem` that you have to keep in sync (the `update_actuals_from_item` dance in `@/home/tommie/projects/rusty-budgets/api/src/models/budget_period.rs:29-36` goes away).

---

## Internal Transfers

Add an `AccountType` to `BankAccount`:

```rust
pub enum AccountType {
    Checking,
    Savings,
    Loan,
}

pub struct BankAccount {
    pub id: Uuid,
    pub account_number: String,
    pub account_type: AccountType,
    // ...
}
```

**Transfer detection**: When two transactions in the same budget have matching amounts (one positive, one negative), same date, and are on different owned accounts → flag as a potential internal transfer. The user confirms, and both transactions get a special `"internal-transfer"` tag allocation. These allocations can then be excluded from expense/income totals, or — if the target is a Savings account — counted as savings.

This doesn't require a new model, just a detection query and a reserved tag convention.

---

## How the Import Workflow Changes

**Current**: Import → hash-dedup → insert transaction → match rules fire → hard-link `actual_id`.

**Proposed**: Import → hash-dedup → insert transaction → match rules fire → **create allocations** (instead of setting `actual_id`). Unallocated transactions show up in a triage view. The user can:
- Accept auto-allocation
- Split a transaction (create multiple allocations)
- Tag with a new tag (which auto-creates a match rule for next time)
- Flag as internal transfer

The budget view then shows each `ActualItem` with its budgeted amount vs. the sum of its allocations. **The focus stays exactly where you want it**: working with the budget, importing, categorizing, balancing.

---

## Migration Path

This is evolutionary, not revolutionary:

1. **Add** `TransactionAllocation` model and events (`AllocationCreated`, `AllocationDeleted`)
2. **Add** `tags` and `periodicity` to `BudgetItem`
3. **Migrate** existing `actual_id` links: for each transaction with `actual_id = Some(x)`, create an allocation `{ transaction_id, actual_id: x, amount: tx.amount, tag: actual.item_name }`
4. **Deprecate** `BankTransaction.actual_id` — keep it temporarily for backward compat, eventually remove
5. **Derive** `actual_amount` from allocations instead of manual tracking

Your CQRS/event-sourcing architecture makes this migration safe — old events stay valid, new events add the allocation concept.

---

## Summary

| Problem | Solution |
|---|---|
| Can't split transactions | `TransactionAllocation` with partial amounts |
| Rigid transaction↔item coupling | Allocations as an indirection layer |
| No tagging | Tags on allocations, aggregated by `BudgetItem.tags` |
| Internal transfers | `AccountType` + transfer detection + reserved tag |
| No periodicity | `Periodicity` enum on `BudgetItem` |
| Denormalized `ActualItem` fields | Derive from `BudgetItem` + compute from allocations |

The `TransactionAllocation` is the single missing concept that unlocks all of your listed pain points. Want me to start implementing any of this?~~

Yes, going back we now need to add tagging to the allocations and to BudgetItems - and also we need periodicity fort the BudgetItems and then we need to fix the Internal Transfers as well


## Executive Summary

This document contains a comprehensive code review of the rusty-budgets application, covering both the API crate (CQRS backend) and UI crate (Dioxus frontend). The application demonstrates solid architectural foundations with event sourcing and CQRS patterns, but lacks several critical features for practical budgeting workflows.

---

## Architecture Review

### API Crate - Strengths

**CQRS Implementation**
- ✅ Clean separation between commands and queries
- ✅ Event sourcing with `StoredEvent<A, E>` pattern
- ✅ Aggregate pattern properly implemented for `Budget`
- ✅ Snapshotting mechanism (every 10 events) for performance
- ✅ JoyDB integration for persistence with RON/JSON adapters

**Domain Model**
- ✅ Rich domain model with `Budget`, `BudgetPeriod`, `BudgetItem`, `ActualItem`
- ✅ Money type with currency support
- ✅ Period-based budgeting with `PeriodId` and `MonthBeginsOn` flexibility
- ✅ Transaction matching rules with tokenization
- ✅ Three budgeting types: Income, Expense, Savings

**Event System**
- ✅ 12 well-defined domain events covering all operations
- ✅ Events are immutable and timestamped
- ✅ User tracking on all events for audit trail

### API Crate - Issues & Concerns

**1. Static Runtime with No Mutex Protection**
```rust
// db.rs:28
pub static CLIENT: Lazy<JoyDbBudgetRuntime> = Lazy::new(|| { ... });
```
- ❌ **Critical**: The runtime is not wrapped in `Mutex` despite being mutable
- ❌ Methods like `cmd()` require `&self` but internally mutate state
- ⚠️ This could cause data races in concurrent scenarios
- **Recommendation**: Wrap in `Lazy<Mutex<JoyDbBudgetRuntime>>` or use `RwLock`

**2. Hardcoded Default User**
```rust
// db.rs:1
const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";
```
- ❌ No multi-user support
- ❌ User management is minimal
- **Recommendation**: Implement proper authentication and user context

**3. Error Handling Inconsistencies**
- ⚠️ Mix of `panic!`, `unwrap()`, and proper error handling
- ⚠️ Some server functions use `.expect()` instead of `?`
- **Recommendation**: Consistent error propagation with `Result<T, RustyError>`

**4. Missing Query Optimization**
- ❌ No caching layer for frequently accessed budgets
- ❌ Event replay happens on every load (mitigated by snapshots)
- ❌ No pagination for transactions or budget items
- **Recommendation**: Add Redis/in-memory cache for active budgets

**5. Transaction Import Limitations**
- ⚠️ Only supports Skandia Excel format
- ❌ No validation of imported data
- ❌ No duplicate detection beyond hash comparison
- **Recommendation**: Support multiple bank formats, add validation layer

**6. Rule Evaluation Performance**
```rust
// db.rs:189
pub fn evaluate_rules(user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError>
```
- ⚠️ Evaluates ALL rules on ALL unconnected transactions
- ⚠️ Called after every transaction import
- **Recommendation**: Incremental rule evaluation, index by transaction patterns

---

## UI Crate - Strengths

**Component Architecture**
- ✅ Reusable component library (Button, Input, Slider, Tabs, etc.)
- ✅ Context-based state management with `BudgetState`
- ✅ Server function integration with proper error handling
- ✅ Responsive design with CSS modules

**User Experience**
- ✅ Transaction connection workflow
- ✅ Budget item editing with slider
- ✅ Period navigation (previous/next month)
- ✅ Auto-budget feature for copying previous period
- ✅ Visual status indicators for budget items

### UI Crate - Critical Missing Features

**1. Budget Overview Dashboard**
- ❌ No high-level summary view
- ❌ Missing key metrics: total income, total expenses, savings rate
- ❌ No visual charts or graphs
- ❌ No trend analysis across periods
- **Impact**: Users can't quickly understand their financial health

**2. Search & Filtering**
- ❌ No search for transactions
- ❌ No filtering by date range, amount, or category
- ❌ No sorting options (by date, amount, description)
- ❌ Can't filter budget items by status or type
- **Impact**: Difficult to find specific transactions in large datasets

**3. Bulk Operations**
- ✅ Partial: Can select multiple transactions to move/ignore
- ❌ No bulk editing of budget amounts
- ❌ No bulk categorization
- ❌ No undo/redo functionality
- **Impact**: Time-consuming for users with many transactions

**4. Budget Templates & Recurring Items**
- ❌ No templates for common budget structures
- ❌ No recurring budget items (rent, subscriptions, etc.)
- ❌ Can't copy budget structure to new periods automatically
- ❌ No budget item groups or categories
- **Impact**: Users must manually recreate budgets each period

**5. Reporting & Analytics**
- ❌ No spending reports
- ❌ No category breakdown visualization
- ❌ No comparison between periods
- ❌ No export functionality (CSV, PDF)
- ❌ No budget vs actual variance analysis
- **Impact**: Limited insights into spending patterns

**6. Transaction Management**
- ❌ No transaction editing (description, amount, date)
- ❌ No manual transaction creation
- ❌ No transaction notes or tags
- ❌ No split transactions
- ❌ No transaction reconciliation workflow
- **Impact**: Can't handle edge cases or corrections

**7. Budget Planning Tools**
- ❌ No "what-if" scenarios
- ❌ No savings goals tracking
- ❌ No debt tracking
- ❌ No income forecasting
- ❌ No budget recommendations based on history
- **Impact**: Limited planning capabilities

**8. Mobile Responsiveness**
- ⚠️ Desktop-first design
- ❌ No mobile-optimized views
- ❌ Touch interactions not optimized
- **Impact**: Poor mobile experience

**9. Accessibility**
- ⚠️ Limited ARIA labels
- ❌ No keyboard navigation shortcuts
- ❌ No screen reader optimization
- **Impact**: Not accessible to users with disabilities

**10. Performance Issues**
```rust
// budget_hero.rs:34
let budget_resource = use_server_future(move || get_budget(None, period_id()))?;
```
- ⚠️ Full budget reload on every period change
- ❌ No optimistic updates
- ❌ No loading skeletons (just "Laddar...")
- **Impact**: Slow perceived performance

---

## Recommended Feature Priorities

### Phase 1: Essential Improvements (High Priority)

**1. Enhanced Dashboard**
```
Components needed:
- BudgetSummaryCard: Income/Expense/Savings totals
- SpendingChart: Visual breakdown by category
- PeriodComparison: Side-by-side period comparison
- QuickStats: Key metrics at a glance
```

**2. Search & Filter System**
```
Features:
- Global search bar for transactions
- Filter panel with date range, amount range, category
- Sort options for all lists
- Saved filter presets
```

**3. Bulk Operations UI**
```
Features:
- Multi-select mode for budget items
- Bulk amount adjustment
- Bulk categorization
- Action history with undo
```

**4. Transaction Details & Editing**
```
Features:
- Transaction detail modal
- Edit description, date, amount
- Add notes and tags
- Split transaction into multiple categories
```

### Phase 2: Planning & Templates (Medium Priority)

**5. Budget Templates**
```
Features:
- Save current budget as template
- Apply template to new period
- Template library (common budgets)
- Recurring item definitions
```

**6. Category Management**
```
Features:
- Custom category creation
- Category hierarchy (parent/child)
- Category icons and colors
- Budget item grouping
```

**7. Recurring Items**
```
Features:
- Define recurring budget items
- Automatic application to new periods
- Frequency options (monthly, quarterly, annual)
- Amount escalation rules
```

### Phase 3: Analytics & Insights (Medium Priority)

**8. Reporting Dashboard**
```
Features:
- Spending trends over time
- Category breakdown charts
- Budget vs actual variance reports
- Export to CSV/PDF
```

**9. Goals & Forecasting**
```
Features:
- Savings goal tracking
- Income/expense forecasting
- Budget recommendations
- Alert system for overspending
```

### Phase 4: Advanced Features (Lower Priority)

**10. Multi-Budget Support**
```
Features:
- Multiple budgets per user
- Budget switching
- Budget sharing/collaboration
- Budget comparison
```

**11. Mobile Optimization**
```
Features:
- Responsive layouts
- Touch-optimized controls
- Mobile-specific views
- Progressive Web App support
```

**12. Import/Export Enhancements**
```
Features:
- Support multiple bank formats
- CSV import/export
- QIF/OFX support
- Import mapping wizard
```

---

## Technical Debt & Refactoring Needs

### API Improvements

**1. Add Query Layer**
```rust
// Separate read models from write models
pub mod queries {
    pub struct BudgetSummaryQuery;
    pub struct TransactionSearchQuery;
    pub struct SpendingTrendsQuery;
}
```

**2. Implement Caching**
```rust
// Add cache layer for frequently accessed data
pub struct BudgetCache {
    cache: Arc<RwLock<HashMap<Uuid, Budget>>>,
    ttl: Duration,
}
```

**3. Add Validation Layer**
```rust
// Validate commands before execution
pub trait Validate {
    fn validate(&self) -> Result<(), ValidationError>;
}
```

**4. Improve Error Types**
```rust
// More specific error types
pub enum BudgetError {
    NotFound(Uuid),
    InvalidAmount(Money),
    PeriodMismatch { expected: PeriodId, actual: PeriodId },
    InsufficientFunds { available: Money, required: Money },
}
```

### UI Improvements

**1. State Management Refactor**
```rust
// Centralized state management
pub struct AppState {
    budget: Signal<BudgetViewModel>,
    filters: Signal<FilterState>,
    ui_state: Signal<UiState>,
}
```

**2. Loading States**
```rust
// Better loading UX
pub enum LoadingState<T> {
    Idle,
    Loading,
    Success(T),
    Error(String),
}
```

**3. Optimistic Updates**
```rust
// Update UI immediately, sync with server
async fn optimistic_update<T>(
    local_update: impl FnOnce(&mut T),
    server_call: impl Future<Output = Result<T>>,
) -> Result<()>
```

**4. Component Library Expansion**
```
Needed components:
- DataTable with sorting/filtering
- DateRangePicker
- AmountInput with currency formatting
- CategoryPicker
- Chart components (Bar, Line, Pie)
- Modal/Dialog improvements
```

---

## Code Quality Observations

### Positive Patterns

1. **Event Sourcing**: Clean implementation with proper versioning
2. **Type Safety**: Strong typing with `Money`, `PeriodId`, `BudgetingType`
3. **Separation of Concerns**: Clear boundaries between layers
4. **Testing**: Some test coverage exists (budget_tests.rs)

### Areas for Improvement

1. **Documentation**: Missing doc comments on most public APIs
2. **Testing**: Limited test coverage, especially for UI components
3. **Logging**: Inconsistent use of `info!`, `error!`, `tracing`
4. **Code Duplication**: Similar patterns in multiple server functions
5. **Magic Numbers**: Hardcoded values (e.g., snapshot threshold of 10)

---

## Security Considerations

**Current Issues:**
- ❌ No authentication/authorization
- ❌ No input validation on server functions
- ❌ No rate limiting
- ❌ No CSRF protection
- ❌ Hardcoded user credentials

**Recommendations:**
1. Implement JWT-based authentication
2. Add input validation middleware
3. Implement rate limiting on server functions
4. Add CSRF tokens for state-changing operations
5. Environment-based configuration for sensitive data

---

## Performance Optimization Opportunities

**API:**
1. Add database indexes for common queries
2. Implement connection pooling
3. Add request caching headers
4. Optimize event replay with incremental snapshots
5. Batch transaction imports

**UI:**
1. Implement virtual scrolling for large lists
2. Lazy load transaction details
3. Debounce search inputs
4. Use React.memo equivalent for expensive components
5. Optimize re-renders with fine-grained signals

---

## Conclusion

The rusty-budgets application has a **solid architectural foundation** with proper CQRS/Event Sourcing patterns. However, it currently lacks many **essential features** for practical budgeting use:

**Critical Gaps:**
- No dashboard/overview
- Limited transaction management
- No search/filtering
- No reporting/analytics
- No budget templates

**Recommended Next Steps:**
1. Implement Phase 1 features (Dashboard, Search, Bulk Operations)
2. Fix the static runtime mutex issue in API
3. Add comprehensive error handling
4. Improve UI loading states and performance
5. Add basic authentication

The codebase is well-structured for expansion, and the CQRS pattern will support adding these features without major refactoring.