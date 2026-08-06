import { test, expect } from "@playwright/test";

test("descriptor-selected browser worker compiles, proves, and verifies Noir", async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto("/rust/web/noir-browser.html");
  await expect
    .poll(() => page.evaluate(() => Boolean(window.noirSmoke)), { timeout: 60_000 })
    .toBe(true);
  const result = await page.evaluate(() => window.noirSmoke);
  expect(result).toEqual({
    identity: "hara/ledger.noir",
    namespace: "ledger.noir",
    artifact: "hara/ledger.noir/v1",
    proof: "hara.noir.proof/v1",
    verified: true
  });
  await page.evaluate(() => window.noirContext.close());
});
