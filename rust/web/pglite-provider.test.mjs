import assert from "node:assert/strict";
import test from "node:test";
import { PGlite } from "../extensions/std-db-pglite/node_modules/@electric-sql/pglite/dist/index.js";
import { createPgliteProvider } from "./packages/db-pglite/index.mjs";

test("PGlite executes parameterized PostgreSQL through the std.db provider core", async () => {
  const pglite = createPgliteProvider(PGlite);
  const opened = await pglite.call("node", "open", [new Map()]);
  assert.equal(opened.engine, "postgresql");
  assert.equal(opened.provider, "pglite");
  assert.equal(opened.storage, "memory");

  await pglite.call("node", "exec", [
    opened.id,
    "create table items (id serial primary key, name text not null)",
    []
  ]);
  const inserted = await pglite.call("node", "exec", [
    opened.id,
    "insert into items (name) values ($1)",
    ["wombat"]
  ]);
  assert.equal(inserted.affected, 1);

  const result = await pglite.call("node", "query", [
    opened.id,
    "select id, name from items where name = $1",
    ["wombat"]
  ]);
  assert.deepEqual(result.columns, ["id", "name"]);
  assert.deepEqual(result.rows, [[1, "wombat"]]);

  assert.equal(await pglite.call("node", "close", [opened.id]), true);
  await assert.rejects(
    pglite.call("node", "query", [opened.id, "select 1", []]),
    /db\/pglite-connection-missing/
  );
});
