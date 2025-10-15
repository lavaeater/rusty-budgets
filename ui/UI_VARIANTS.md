# Budget UI Variants

This document describes the three available UI variants for the budgeting application. Each variant offers a different user experience optimized for different workflows and preferences.

## Overview

- **`budget`** - Original one-page layout (default)
- **`budget_a`** - Dashboard-focused with overview cards
- **`budget_b`** - Workflow-oriented with sidebar

## Variant Details

### Original (`budget`)

**Layout**: Single-page, vertical scroll
**Best for**: Users who want everything visible at once

**Features**:
- Header with budget name and period
- Tabbed interface for Income/Expenses/Savings
- All budget items displayed under their category
- Transaction list at the bottom of the page
- Simple, straightforward layout

**When to use**: 
- You prefer a traditional, linear layout
- You want to see all information by scrolling
- You're comfortable with the existing interface

---

### Variant A (`budget_a`)

**Layout**: Dashboard-style with prominent metrics
**Best for**: Users who want quick overview and visual feedback

**Features**:
- **Dashboard cards** showing key metrics for each category (Income, Expenses, Savings)
  - Budgeted amount
  - Actual amount
  - Remaining budget
- **Prominent unassigned badge** in header showing transaction count
- **Visual hierarchy** - most important info up front
- **Conditional transaction section**:
  - Highlighted red section when there are unassigned transactions
  - Success message when all transactions are handled
- Enhanced transaction cards with better visual separation

**When to use**:
- You want to see budget health at a glance
- You prefer visual dashboards over lists
- You want unassigned transactions to be visually prominent
- You like color-coded feedback (green for success, red for attention needed)

**Styling**: Modern card-based design with gradients and shadows

---

### Variant B (`budget_b`)

**Layout**: Split-screen with sidebar workflow
**Best for**: Users who process transactions in batches

**Features**:
- **Persistent sidebar** (400px) showing unassigned transactions
- **Click-to-expand** transaction items in sidebar
- **Workflow-optimized** for processing transactions one by one
- **Main content area** with full budget management
- **Compact transaction display** - only shows actions when selected
- **Responsive design** - sidebar moves to top on mobile

**Workflow**:
1. Unassigned transactions appear in left sidebar
2. Click a transaction to expand its options
3. Choose to connect to existing item or create new
4. Transaction disappears from sidebar when handled
5. Sidebar auto-hides when all transactions are processed

**When to use**:
- You regularly import and process many transactions
- You prefer a step-by-step workflow
- You want transactions always visible while managing budget
- You like having a dedicated "inbox" for unprocessed items

**Styling**: Clean, professional split-screen with focus on workflow efficiency

---

## How to Switch Variants

### In Desktop/Web/Mobile Apps

Edit the `src/views/home.rs` file in your app:

#### Use Original (Default)
```rust
use dioxus::prelude::*;
use ui::BudgetHero;

#[component]
pub fn Home() -> Element {
    rsx! {
        BudgetHero {}
    }
}
```

#### Use Variant A (Dashboard)
```rust
use dioxus::prelude::*;
use ui::budget_a::BudgetHero;

#[component]
pub fn Home() -> Element {
    rsx! {
        BudgetHero {}
    }
}
```

#### Use Variant B (Workflow)
```rust
use dioxus::prelude::*;
use ui::budget_b::BudgetHero;

#[component]
pub fn Home() -> Element {
    rsx! {
        BudgetHero {}
    }
}
```

### Testing Multiple Variants

You can create a variant selector component to test all three:

```rust
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let mut variant = use_signal(|| "original");
    
    rsx! {
        div {
            div { style: "padding: 10px; background: #f0f0f0;",
                button { onclick: move |_| variant.set("original"), "Original" }
                button { onclick: move |_| variant.set("variant_a"), "Variant A" }
                button { onclick: move |_| variant.set("variant_b"), "Variant B" }
            }
            
            match variant() {
                "variant_a" => rsx! { ui::budget_a::BudgetHero {} },
                "variant_b" => rsx! { ui::budget_b::BudgetHero {} },
                _ => rsx! { ui::budget::BudgetHero {} },
            }
        }
    }
}
```

## CSS Files

Each variant has its own CSS file:

- **Original**: `ui/assets/styling/budget-hero.css`
- **Variant A**: `ui/assets/styling/budget-hero-a.css`
- **Variant B**: `ui/assets/styling/budget-hero-b.css`

The CSS is automatically loaded by each variant's BudgetHero component.

## Customization

Each variant can be customized by:

1. **Modifying the CSS** - Change colors, spacing, fonts in the respective CSS file
2. **Editing component files** - Modify behavior in `ui/src/budget_*/` directories
3. **Creating new variants** - Copy a variant directory and customize

## Component Structure

Each variant contains the same set of components:

```
budget_*/
├── budget_hero.rs          # Main component
├── budget_tabs.rs          # Tab navigation
├── budgeting_type_card.rs  # Category display
├── budget_item_view.rs     # Individual item
├── transactions_view.rs    # Transaction list
├── item_selector.rs        # Dropdown selector
├── new_budget_item.rs      # Create new item
└── ...
```

## Recommendations

- **Start with Original** if you're new to the app
- **Try Variant A** if you want better visual feedback
- **Use Variant B** if you process many transactions regularly

## Future Enhancements

Potential improvements for each variant:

**Variant A**:
- Add charts/graphs to dashboard cards
- Implement budget vs actual progress bars
- Add quick actions to dashboard cards

**Variant B**:
- Add keyboard shortcuts for transaction processing
- Implement batch operations (select multiple transactions)
- Add transaction filtering in sidebar
- Save transaction processing position

**All Variants**:
- Dark mode support
- User preference persistence
- Customizable layouts
- Export/print views
