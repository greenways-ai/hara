import { PGlite } from "@electric-sql/pglite";
import { serveNodeProvider } from "@hara-lang/hta/provider/node";
import { createPgliteProvider } from "@hara-lang/db-pglite";

const pglite = createPgliteProvider(PGlite);
serveNodeProvider(
  (operation, args) => pglite.call("node", operation, args),
  { errorCode: "db/pglite-error" }
);
