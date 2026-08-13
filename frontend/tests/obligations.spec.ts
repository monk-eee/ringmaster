import { test, expect } from "@playwright/test";

// ADR-0012: asserts DOM structure only, never specific row counts or
// obligation content — the shared development Postgres volume accumulates
// obligations across sessions and agents, so exact content is not stable.
test("obligations page renders a table backed by the backend API", async ({ page }) => {
  await page.goto("/");

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
