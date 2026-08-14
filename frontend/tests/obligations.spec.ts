import { test, expect } from "@playwright/test";

// ADR-0014: asserts real client-side rendering and interaction (tab
// switching, filtering), never specific row counts or content — the shared
// development Postgres volume accumulates data across sessions and agents.
test("obligations tab renders a table backed by the backend API", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("tab", { name: "Obligations" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("h1")).toHaveText("Obligations");
  await expect(page.locator("p.error")).toHaveCount(0);

  const headerCells = page.locator("table thead th");
  await expect(headerCells).toHaveText(["Obligation", "Status", "Updated"]);

  const rows = page.locator("table tbody tr");
  const rowCount = await rows.count();
  if (rowCount > 0) {
    const firstRowCells = rows.first().locator("td");
    await expect(firstRowCells).toHaveCount(3);
    for (const cell of await firstRowCells.all()) {
      await expect(cell).not.toHaveText("");
    }
  }
});

test("switching to the Candidates tab renders a different, real client-side view", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("tab", { name: "Candidates" }).click();

  await expect(page.getByRole("tab", { name: "Candidates" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("h1")).toHaveText("Candidates");

  const headerCells = page.locator("table thead th");
  const headerCount = await headerCells.count();
  if (headerCount > 0) {
    await expect(headerCells).toHaveText(["Type", "Statement", "Validation state", "Confidence", "Evidence"]);
  }
});

test("filtering obligations by status only shows matching rows", async ({ page }) => {
  await page.goto("/");

  // Wait for the async fetch to populate the table before reading filter options.
  await expect(page.locator("table tbody tr").first()).toBeVisible();

  const statusOptions = await page.locator("select").first().locator("option").allTextContents();
  const concreteStatus = statusOptions.find((option) => option !== "All");
  test.skip(!concreteStatus, "no obligations exist yet to filter by status");

  await page.locator("select").first().selectOption(concreteStatus!);

  const badges = page.locator("table tbody .badge");
  const badgeCount = await badges.count();
  for (let index = 0; index < badgeCount; index += 1) {
    await expect(badges.nth(index)).toHaveText(concreteStatus!);
  }
});
