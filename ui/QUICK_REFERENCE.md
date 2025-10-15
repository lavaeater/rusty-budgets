# Quick Reference Card

## 🚀 Switch UI Variants in 30 Seconds

### Step 1: Choose Your Variant

| Variant | Best For | Key Feature |
|---------|----------|-------------|
| **Original** | Simple needs | One-page scroll |
| **Variant A** | Visual feedback | Dashboard cards |
| **Variant B** | Batch processing | Sidebar workflow |

### Step 2: Update Import

Edit `desktop/src/views/home.rs` (or web/mobile):

```rust
// Change this line:
use ui::BudgetHero;              // Original

// To one of these:
use ui::budget_a::BudgetHero;    // Variant A
use ui::budget_b::BudgetHero;    // Variant B
```

### Step 3: Run

```bash
cd desktop  # or web/mobile
dx serve
```

That's it! ✨

---

## 📊 Feature Matrix

|  | Original | Variant A | Variant B |
|---|:---:|:---:|:---:|
| Overview cards | ❌ | ✅ | ❌ |
| Visual dashboard | ❌ | ✅ | ❌ |
| Transaction badge | ❌ | ✅ | ❌ |
| Persistent sidebar | ❌ | ❌ | ✅ |
| Click-to-expand | ❌ | ❌ | ✅ |
| One-page scroll | ✅ | ✅ | ❌ |
| Split-screen | ❌ | ❌ | ✅ |
| Mobile-friendly | ✅ | ✅ | ⚠️ |

✅ = Yes | ❌ = No | ⚠️ = Limited

---

## 🎨 Visual Quick Look

### Original
```
[Header]
[Tabs: Income | Expenses | Savings]
[Items list]
[Transactions list]
```

### Variant A
```
[Header with badge]
[📊 Card | 📊 Card | 📊 Card]
[Tabs: Income | Expenses | Savings]
[Items list]
[🔴 Transactions (highlighted)]
```

### Variant B
```
[Header]
┌──────────┬────────────────┐
│ Trans-   │ [Tabs]         │
│ actions  │ [Items list]   │
│ Sidebar  │                │
└──────────┴────────────────┘
```

---

## 💡 Decision Tree

```
Do you process many transactions daily?
├─ YES → Variant B (Workflow)
└─ NO
   └─ Do you want visual overview?
      ├─ YES → Variant A (Dashboard)
      └─ NO → Original
```

---

## 🔧 Common Customizations

### Change Colors (Variant A)
Edit `ui/assets/styling/budget-hero-a.css`:
- Line 13: Header gradient
- Line 33: Badge color
- Line 72: Card border

### Change Sidebar Width (Variant B)
Edit `ui/assets/styling/budget-hero-b.css`:
- Line 42: `.transactions-sidebar` width (default: 400px)

### Add Dark Mode
All variants support CSS variables - add to respective CSS file:
```css
:root {
  --bg-color: #1a202c;
  --text-color: #e2e8f0;
}
```

---

## 📱 Mobile Support

| Variant | Mobile Experience |
|---------|-------------------|
| Original | ✅ Excellent - designed for mobile |
| Variant A | ✅ Good - cards stack vertically |
| Variant B | ⚠️ Fair - sidebar moves to top |

---

## 🐛 Troubleshooting

### Import Error
```
error: unresolved import `ui::BudgetHero`
```
**Fix:** Use full path: `ui::budget_a::BudgetHero`

### CSS Not Loading
**Fix:** Check Asset path in `budget_hero.rs` matches file name

### Components Not Found
**Fix:** Import from module: `use crate::budget_a::{Component};`

---

## 📚 Full Documentation

- **Detailed Guide:** [UI_VARIANTS.md](./UI_VARIANTS.md)
- **Visual Layouts:** [VISUAL_COMPARISON.md](./VISUAL_COMPARISON.md)
- **Technical Details:** [IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md)
- **Code Example:** [EXAMPLE_VARIANT_SWITCHER.rs](./EXAMPLE_VARIANT_SWITCHER.rs)

---

## ⚡ Pro Tips

1. **Test all three** - Use the variant switcher example
2. **Start with Original** - Migrate when you're ready
3. **Customize freely** - Each variant is independent
4. **Mix and match** - Different platforms can use different variants
5. **No data migration** - All variants use the same backend

---

## 🎯 At a Glance

**Original:** Simple, familiar, works everywhere
**Variant A:** Visual, dashboard-style, desktop-first
**Variant B:** Efficient, workflow-focused, power users

Choose based on your workflow, not features!
