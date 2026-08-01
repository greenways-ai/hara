import { serveBrowserProvider } from "@hara-lang/hta/provider/browser";
import { callSqlite } from "@hara-lang/db-sqlite";

serveBrowserProvider(
  (operation, args) => callSqlite("browser", operation, args),
  { errorCode: "db/sqlite-error" }
);
