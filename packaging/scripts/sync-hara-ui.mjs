#!/usr/bin/env node

// Hara UI is now consumed directly from the hara-www sibling repository
// (website/hara-www/vendor/hara-ui). It is a submodule of hara-lang/hara-www,
// so synchronization happens in that repo, not here. This script is kept as a
// no-op for backward compatibility.

import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const source = path.join(root, "website", "hara-www", "vendor", "hara-ui");

if (!existsSync(source)) {
  console.error(
    `hara-ui source not found at ${source}; clone hara-lang/hara-www into website/hara-www`,
  );
  process.exit(1);
}

console.log("hara-ui is managed inside hara-lang/hara-www; no sync needed.");
