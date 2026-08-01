import sqlite3InitModule from "@sqlite.org/sqlite-wasm";
import { serveBrowserProvider } from "@hara-lang/hta/provider/browser";
import { createSqliteProvider } from "@hara-lang/db-sqlite";

const sqlite = createSqliteProvider(sqlite3InitModule);
serveBrowserProvider(
  (operation, args) => sqlite.call("browser", operation, args),
  { errorCode: "db/sqlite-error" }
);
