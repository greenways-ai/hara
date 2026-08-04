import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const page = readFileSync(new URL("../public/index.html", import.meta.url), "utf8");

test("registry API landing page exposes the Hara social card", () => {
  assert.match(page, /property="og:image" content="https:\/\/api\.hara-lang\.org\/og-hara-api\.jpg"/);
  assert.match(page, /property="og:image:width" content="3840"/);
  assert.match(page, /property="og:image:height" content="2016"/);
  assert.match(page, /name="twitter:card" content="summary_large_image"/);
});
