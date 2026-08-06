import assert from "node:assert/strict";
import test from "node:test";
import sqlite3InitModule from "../extensions/std-db-sqlite/node_modules/@sqlite.org/sqlite-wasm/node.mjs";
import { createSqliteProvider } from "./packages/db-sqlite/index.mjs";

test("SQLite WASM executes parameterized SQL through the std.db provider core", async () => {
  const sqlite = createSqliteProvider(sqlite3InitModule);
  const version = await sqlite.call("node", "version", []);
  assert.equal(version.engine, "sqlite");
  assert.match(version.version, /^3\./);

  const opened = await sqlite.call("node", "open", [new Map()]);
  assert.equal(opened.engine, "sqlite");
  assert.equal(opened.storage, "memory");

  await sqlite.call("node", "exec", [
    opened.id,
    "create table items (id integer primary key, name text not null)",
    []
  ]);
  const inserted = await sqlite.call("node", "exec", [
    opened.id,
    "insert into items (name) values (?)",
    ["wombat"]
  ]);
  assert.equal(inserted.affected, 1);

  const result = await sqlite.call("node", "query", [
    opened.id,
    "select id, name from items where name = ?",
    ["wombat"]
  ]);
  assert.deepEqual(result.columns, ["id", "name"]);
  assert.deepEqual(result.rows, [[1, "wombat"]]);

  assert.equal(await sqlite.call("node", "close", [opened.id]), true);
  await assert.rejects(
    sqlite.call("node", "query", [opened.id, "select 1", []]),
    /db\/sqlite-connection-missing/
  );
});
