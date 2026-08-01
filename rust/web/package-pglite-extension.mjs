import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { packageDbProvider } from "./package-db-provider.mjs";

const web = dirname(fileURLToPath(import.meta.url));
const repository = resolve(web, "../..");
const source = process.env.HARA_PGLITE_SOURCE
  ? resolve(process.env.HARA_PGLITE_SOURCE)
  : resolve(repository, "rust/extensions/std-db-pglite");
const output = process.env.HARA_PGLITE_OUTPUT
  ? resolve(process.env.HARA_PGLITE_OUTPUT)
  : resolve(source, "target/package/std/db/provider/pglite");

const packaged = await packageDbProvider({
  source,
  output,
  nodeBuild: resolve(web, "dist-pglite-node"),
  browserBuild: resolve(web, "dist-pglite-browser")
});
console.log(`${packaged.output} (${packaged.assets.length} assets)`);
