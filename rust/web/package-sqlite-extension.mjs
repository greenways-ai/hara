import { cp, mkdir, rm } from "node:fs/promises";
import { resolve } from "node:path";

const web = import.meta.dirname;
const repository = resolve(web, "../..");
const source = process.env.HARA_SQLITE_SOURCE
  ? resolve(process.env.HARA_SQLITE_SOURCE)
  : resolve(repository, "rust/extensions/std-db-sqlite");
const output = process.env.HARA_SQLITE_OUTPUT
  ? resolve(process.env.HARA_SQLITE_OUTPUT)
  : resolve(source, "target/package/std/db/provider/sqlite");

await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
await cp(resolve(source, "hara.extension.edn"), resolve(output, "hara.extension.edn"));
await cp(resolve(source, "package.json"), resolve(output, "package.json"));
await cp(resolve(web, "dist-sqlite-node"), resolve(output, "node"), { recursive: true });
await cp(resolve(web, "dist-sqlite-browser"), resolve(output, "browser"), { recursive: true });

console.log(output);
