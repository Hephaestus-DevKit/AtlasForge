import { test, expect } from "@playwright/test";

test.describe("AtlasForge Smoke", () => {
  test("app loads and shows Dashboard", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("AtlasForge", { exact: true })).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
  });

  test("navigation to Assets page", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("link", { name: "Assets" }).click();
    await expect(page).toHaveURL(/assets/);
    await expect(page.getByRole("heading", { name: "Assets" })).toBeVisible();
  });

  test("navigation to Settings page", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("link", { name: "Settings" }).click();
    await expect(page).toHaveURL(/settings/);
    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  });

  test("app shell never renders a blank body", async ({ page }) => {
    await page.goto("/");
    const body = page.locator("body");
    await expect(body).not.toBeEmpty();
  });

  test("automations exposes only implemented scheduled notifications", async ({ page }) => {
    await page.goto("/automations");
    await expect(page.getByRole("heading", { name: "Automations" })).toBeVisible();
    await page.getByRole("button", { name: "New Rule" }).click();
    await expect(page.getByLabel("Trigger")).toHaveValue("Schedule");
    await expect(page.getByLabel("Action")).toHaveValue("Notification");
    await expect(page.getByLabel("Interval (minutes)")).toHaveValue("60");
    await expect(page.getByText("Auto-apply fixes when possible")).toHaveCount(0);
  });
});
