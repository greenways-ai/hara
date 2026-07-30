import { cp, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const web = import.meta.dirname;
const repository = resolve(web, "../..");
const source = process.env.HARA_NOIR_SOURCE
  ? resolve(process.env.HARA_NOIR_SOURCE)
  : resolve(repository, "rust/extensions/ledger-noir");
const output = process.env.HARA_NOIR_OUTPUT
  ? resolve(process.env.HARA_NOIR_OUTPUT)
  : resolve(source, "target/package/ledger/noir");
const assets = resolve(output, "assets");

await rm(output, { recursive: true, force: true });
await mkdir(assets, { recursive: true });
await cp(resolve(source, "hara.extension.edn"), resolve(output, "hara.extension.edn"));
await cp(resolve(source, "package.json"), resolve(output, "package.json"));
await mkdir(resolve(output, "node"), { recursive: true });
await cp(resolve(web, "dist-node/worker.mjs"), resolve(output, "node/worker.mjs"));
await cp(resolve(source, "browser"), resolve(output, "browser"), { recursive: true });
await cp(resolve(source, "assets/noir-provider.mjs"), resolve(assets, "noir-provider.mjs"));
await cp(resolve(web, "hta.js"), resolve(assets, "hta.js"));

const compiler = resolve(web, "node_modules/@noir-lang/noir_wasm/dist/web/main.mjs");
await cp(compiler, resolve(web, "dist/noir-wasm.mjs"));
await cp(compiler, resolve(assets, "noir-wasm.mjs"));

let loader = await readFile(resolve(web, "dist/noir-loader.js"), "utf8");
const generatedAssets = await readdir(resolve(web, "dist/assets"));
for (const stem of ["main.worker", "thread.worker"]) {
  const generated = generatedAssets.find(name => name.startsWith(`${stem}-`) && name.endsWith(".js"));
  if (!generated) throw new Error(`Missing generated ${stem} asset`);
  const canonical = `${stem}.js`;
  loader = loader.replaceAll(generated, canonical);
  await cp(resolve(web, "dist/assets", generated), resolve(assets, canonical));
}
await writeFile(resolve(assets, "noir-loader.js"), loader);
for (const name of ["barretenberg.js", "barretenberg-threads.js"]) {
  await cp(resolve(web, "dist", name), resolve(assets, name));
}
console.log(output);
