import { serveNodeProvider } from "@hara-lang/hta/provider/node";
import { callSqlite } from "@hara-lang/db-sqlite";

serveNodeProvider(
  (operation, args) => callSqlite("node", operation, args),
  { errorCode: "db/sqlite-error" }
);
