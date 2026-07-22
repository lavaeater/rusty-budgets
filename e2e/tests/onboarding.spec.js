// @ts-check
const { test, expect } = require("@playwright/test");

// The core onboarding flow against a fresh, empty database:
//   fresh DB (no default budget)  ->  "Ingen budget hittad" + create form
//   create a budget               ->  the budget overview (name + tools)
//
// Runs serially so the second test observes the budget the first one created
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

  test("creating a budget lands on the overview", async ({ page }) => {
    await page.goto("/");

    const name = "E2E Budget";
    await page.locator("#name").fill(name);
    await page.getByRole("button", { name: "Skapa budget" }).click();

    // After creation the overview renders the budget name as the heading and
    // exposes the "Verktyg" (tools) menu.
    await expect(page.getByRole("heading", { name })).toBeVisible();
    await expect(page.getByText("Verktyg")).toBeVisible();
  });
});
