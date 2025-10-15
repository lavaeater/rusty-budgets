# UI

This crate contains all shared components for the workspace, including three distinct budget UI variants.

## 🎨 Budget UI Variants

This crate now includes **three complete UI implementations** for the budgeting application:

### Quick Start

```rust
// Use the original layout (default)
use ui::BudgetHero;

// Or use Variant A (Dashboard-focused)
use ui::budget_a::BudgetHero;

// Or use Variant B (Workflow-oriented)
use ui::budget_b::BudgetHero;
```

### The Three Variants

1. **Original (`budget`)** - One-page vertical scroll layout
2. **Variant A (`budget_a`)** - Dashboard with overview cards and visual feedback
3. **Variant B (`budget_b`)** - Split-screen with persistent transaction sidebar

### 📚 Documentation

- **[UI_VARIANTS.md](./UI_VARIANTS.md)** - Detailed feature comparison and usage guide
- **[VISUAL_COMPARISON.md](./VISUAL_COMPARISON.md)** - Visual layouts and interaction patterns
- **[IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md)** - Technical details and customization
- **[EXAMPLE_VARIANT_SWITCHER.rs](./EXAMPLE_VARIANT_SWITCHER.rs)** - Code example for testing variants

### Structure

```
ui/
├─ src/
│  ├─ lib.rs              # Entrypoint - exports all three variants
│  ├─ budget/             # Original UI (one-page layout)
│  ├─ budget_a/           # Variant A (dashboard-focused)
│  ├─ budget_b/           # Variant B (workflow-oriented)
│  ├─ components/         # Shared components (Button, Input, etc.)
│  └─ file_chooser/       # File import dialog
├─ assets/
│  └─ styling/
│     ├─ budget-hero.css   # Original styles
│     ├─ budget-hero-a.css # Variant A styles
│     └─ budget-hero-b.css # Variant B styles
```

## Shared Components

This crate also provides reusable components:
- `Button` - Styled button component
- `Input` - Form input with event handlers
- `Separator` - Visual divider
- `Tabs`, `TabList`, `TabTrigger`, `TabContent` - Tab navigation
- `Popover` components - Overlay dialogs
- And more in `src/components/`

## Dependencies

Since this crate is shared between multiple platforms, it should not pull in any platform specific dependencies. For example, if you want to use the `web_sys` crate in the web build of your app, you should not add it to this crate. Instead, you should add platform specific dependencies to the [web](../web/Cargo.toml), [desktop](../desktop/Cargo.toml), or [mobile](../mobile/Cargo.toml) crates.

## Testing Variants

See [EXAMPLE_VARIANT_SWITCHER.rs](./EXAMPLE_VARIANT_SWITCHER.rs) for a complete example of how to create a runtime switcher to test all three variants.
