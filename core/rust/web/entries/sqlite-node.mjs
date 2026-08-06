import sqlite3InitModule from "@sqlite.org/sqlite-wasm";
import { serveNodeProvider } from "@hara-lang/hta/provider/node";
import { createSqliteProvider } from "@hara-lang/db-sqlite";

const sqlite = createSqliteProvider(sqlite3InitModule);
serveNodeProvider(
  (operation, args) => sqlite.call("node", operation, args),
  { errorCode: "db/sqlite-error" }
);
