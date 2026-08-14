import { test, expect } from "@playwright/test";

// ADR-0014: asserts real client-side rendering and interaction (tab
// switching, filtering), never specific row counts or content — the shared
// development Postgres volume accumulates data across sessions and agents.
// ADR-0022: Daily Brief is the default landing tab -- "start with
// Attention, not Work" (VISION.md).
test("daily brief tab renders a ranked list by default", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("tab", { name: "Daily Brief" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("h1")).toHaveText("Daily Brief");
  await expect(page.locator("p.error")).toHaveCount(0);
});

test("obligations tab renders a table backed by the backend API", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("tab", { name: "Obligations" }).click();
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
    await expect(headerCells).toHaveText(["Type", "Statement", "Validation state", "Confidence", "Evidence", "Actions"]);
  }
});

test("filtering obligations by status only shows matching rows", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("tab", { name: "Obligations" }).click();

  // Wait for the async fetch to populate the table before reading filter options.
  await expect(page.locator("table tbody tr").first()).toBeVisible();

  const statusOptions = await page.locator("select").first().locator("option").allTextContents();
  const concreteStatus = statusOptions.find((option) => option !== "All");
  test.skip(!concreteStatus, "no obligations exist yet to filter by status");

  await page.locator("select").first().selectOption(concreteStatus!);

  const badges = page.locator("table tbody .badge");
  const badgeCount = await badges.count();
  for (let index = 0; index < badgeCount; index += 1) {
    // Badge text carries a leading status icon (ADR-0030), so match the status
    // name within the text rather than asserting an exact string.
    await expect(badges.nth(index)).toContainText(concreteStatus!);
  }
});

// ADR-0019: tolerant of whether RINGMASTER_EMBEDDING_URL is configured in
// this environment -- asserts search reaches a final rendered state
// (results, empty state, or a surfaced backend error), never a crash.
test("search tab submits a query and renders a final state", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("tab", { name: "Search" }).click();
  await expect(page.getByRole("tab", { name: "Search" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("h1")).toHaveText("Search");

  await page.getByPlaceholder("Search meeting evidence…").fill("transition plan");
  await page.getByRole("button", { name: "Search" }).click();

  await expect(page.getByRole("button", { name: "Search" })).toBeEnabled({ timeout: 15000 });
  const hasError = (await page.locator("p.error").count()) > 0;
  const hasEmptyState = (await page.locator("p.empty-state").count()) > 0;
  const hasResultRows = (await page.locator("table tbody tr").count()) > 0;
  expect(hasError || hasEmptyState || hasResultRows).toBe(true);
});

// ADR-0033: proves the client-side traversal trail can grow across two real
// edges and return to the root, entirely on top of the existing one-hop
// GET /api/nodes/:id primitive (no multi-hop backend route involved). Node
// type/text are kept short (<14 chars) and stamped with Date.now() so they
// (a) survive the radial view's label truncation and (b) stay unique against
// the shared development database and any concurrent agent session.
test("graph trail: traversing two edges and returning to the root (ADR-0033)", async ({ page }) => {
  const stamp = Date.now() % 100000;
  const nodeType = `trailtest${stamp}`;
  const nodeA = `Rt${stamp}`;
  const nodeB = `Md${stamp}`;
  const nodeC = `Lf${stamp}`;

  await page.goto("/");
  await page.getByRole("tab", { name: "Graph" }).click();
  await expect(page.getByRole("tab", { name: "Graph" })).toHaveAttribute("aria-selected", "true");

  async function createNode(text: string) {
    await page.getByPlaceholder("Node type (e.g. person, risk)").fill(nodeType);
    await page.getByPlaceholder("Canonical text (e.g. a name)").fill(text);
    await page.getByRole("button", { name: "Create node" }).click();
    await expect(page.locator(".node-detail h3")).toContainText(text);
  }

  async function selectNode(text: string) {
    await page.locator(".node-list-button", { hasText: text }).click();
    await expect(page.locator(".node-detail h3")).toContainText(text);
  }

  async function link(fromText: string, toText: string, verb: string) {
    await selectNode(fromText);
    await page.locator(".node-detail select").selectOption({ label: `${nodeType}: ${toText}` });
    await page.getByPlaceholder("Relationship (e.g. made, owns)").fill(verb);
    await page.getByRole("button", { name: "Add relationship" }).click();
    // The form resets its fields on success (so the submit button goes back to
    // disabled by design, not because the request is stuck) -- the real proof
    // of success is the new neighbor appearing in the radial view.
    await expect(page.locator("svg.relationship-view g", { hasText: toText })).toBeVisible();
  }

  // Three fresh nodes, linked A -> B -> C, so the trail has two real edges to cross.
  await createNode(nodeA);
  await createNode(nodeB);
  await createNode(nodeC);
  await link(nodeA, nodeB, "leads_to");
  await link(nodeB, nodeC, "leads_to");

  // Start a fresh trail at the root: selecting from the node list always
  // begins a new trail (ADR-0033), so this is exactly one item.
  await selectNode(nodeA);
  await expect(page.locator(".graph-trail-path .graph-trail-item")).toHaveCount(1);
  await expect(page.locator(".graph-trail-back")).toBeDisabled();

  // Hop 1: click B's neighbor circle from A's radial view.
  await page.locator("svg.relationship-view g", { hasText: nodeB }).locator("circle.relationship-node-clickable").click();
  await expect(page.locator(".node-detail h3")).toContainText(nodeB);
  await expect(page.locator(".graph-trail-path .graph-trail-item")).toHaveCount(2);
  await expect(page.locator(".why-here")).toContainText(nodeA);
  await expect(page.locator(".why-here")).toContainText("leads_to");

  // Hop 2: from B, click specifically C's circle (B also neighbors A, so scope by C's label).
  await page.locator("svg.relationship-view g", { hasText: nodeC }).locator("circle.relationship-node-clickable").click();
  await expect(page.locator(".node-detail h3")).toContainText(nodeC);
  await expect(page.locator(".graph-trail-path .graph-trail-item")).toHaveCount(3);

  // The breadcrumb is a readable path: every node and the traversed verb appear in order.
  const trailText = await page.locator(".graph-trail-path").innerText();
  expect(trailText).toContain(nodeA);
  expect(trailText).toContain(nodeB);
  expect(trailText).toContain(nodeC);
  expect(trailText).toContain("leads_to");

  // Returning to the root via the dedicated Back control, one edge at a time.
  await page.locator(".graph-trail-back").click();
  await expect(page.locator(".node-detail h3")).toContainText(nodeB);
  await expect(page.locator(".graph-trail-path .graph-trail-item")).toHaveCount(2);

  await page.locator(".graph-trail-back").click();
  await expect(page.locator(".node-detail h3")).toContainText(nodeA);
  await expect(page.locator(".graph-trail-path .graph-trail-item")).toHaveCount(1);
  await expect(page.locator(".graph-trail-back")).toBeDisabled();
});
