// @ts-check
const { test, expect } = require("@playwright/test");

// The core onboarding flow against a fresh, empty database:
//   fresh DB (no default budget)  ->  "Ingen budget hittad" + create form
//   create a budget               ->  the period workspace (name + tab bar)
//
// Runs serially so the later tests observe the budget the first one created
// (the whole suite shares one throwaway DB — see playwright.config.js).
test.describe.serial("onboarding", () => {
  test("a fresh database shows the create-budget screen", async ({ page }) => {
    await page.goto("/");

    // The app boots into the NoDefaultBudget state on an empty DB.
    await expect(page.getByText("Ingen budget hittad")).toBeVisible();
    // The name field and the create button are present.
    await expect(page.locator("#name")).toBeVisible();
    await expect(page.getByRole("button", { name: "Skapa budget" })).toBeVisible();
  });

  test("creating a budget lands on the workspace", async ({ page }) => {
    await page.goto("/");

    const name = "E2E Budget";
    await page.locator("#name").fill(name);
    await page.getByRole("button", { name: "Skapa budget" }).click();

    // After creation the workspace renders the budget name as the heading and
    // the task tab bar, with Översikt selected by default.
    await expect(page.getByRole("heading", { name })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Översikt" })).toHaveAttribute(
      "data-state",
      "active",
    );
  });

  test("the tab bar exposes every task view", async ({ page }) => {
    await page.goto("/");

    for (const label of [
      "Översikt",
      "Budget",
      "Transaktioner",
      "Att göra",
      "Rapporter",
      "Inställningar",
    ]) {
      await expect(page.getByRole("tab", { name: label })).toBeVisible();
    }
  });

  test("switching tabs updates the URL and the panel", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("tab", { name: "Inställningar" }).click();

    // Tab switches push a canonical, linkable address.
    await expect(page).toHaveURL(/\/budget\/\d{4}-\d{1,2}\/installningar$/);
    // "Verktyg" (the auto-budget tools) now lives in the settings tab.
    await expect(page.getByRole("heading", { name: "Verktyg" })).toBeVisible();
  });

  test("the back button returns to the previous tab", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("tab", { name: "Inställningar" }).click();
    await expect(page).toHaveURL(/installningar$/);
    await page.getByRole("tab", { name: "Rapporter" }).click();
    await expect(page).toHaveURL(/rapporter$/);

    await page.goBack();

    // Proves the URL -> workspace sync, not just workspace -> URL.
    await expect(page).toHaveURL(/installningar$/);
    await expect(
      page.getByRole("tab", { name: "Inställningar" }),
    ).toHaveAttribute("data-state", "active");
  });

  test("a tab URL is directly linkable", async ({ page }) => {
    // Learn the current period from a normal navigation ...
    await page.goto("/");
    await page.getByRole("tab", { name: "Rapporter" }).click();
    await expect(page).toHaveURL(/rapporter$/);
    const period = new URL(page.url()).pathname.split("/")[2];

    // ... then deep-link straight into a different tab.
    await page.goto(`/budget/${period}/installningar`);

    await expect(
      page.getByRole("tab", { name: "Inställningar" }),
    ).toHaveAttribute("data-state", "active");
    await expect(page.getByRole("heading", { name: "Verktyg" })).toBeVisible();
  });

  // Regression guard: `budget-hero.css` carries the container/header/card rules
  // and was linked by the old `BudgetOverview` body. The Phase 7.2 split dropped
  // that link and the whole loaded-budget view rendered unstyled. Native SSR
  // tests cannot catch this — `document::Link` goes to the document head, which
  // `dioxus_ssr::render_element` does not capture — so it belongs here.
  test("the workspace is actually styled", async ({ page }) => {
    await page.goto("/");

    const container = page.locator(".budget-hero-a-container");
    await expect(container).toBeVisible();

    // From budget-hero.css — proves the sheet loaded and applied.
    await expect
      .poll(() =>
        container.evaluate((el) => getComputedStyle(el).maxWidth),
      )
      .toBe("1400px");

    // From workspace.css — proves the new sheet loaded too.
    const tab = page.getByRole("tab", { name: "Översikt" });
    await expect
      .poll(() => tab.evaluate((el) => getComputedStyle(el).fontWeight))
      .toBe("600");
  });
});
