import { PGlite } from "@electric-sql/pglite";
import { serveBrowserProvider } from "@hara-lang/hta/provider/browser";
import { createPgliteProvider } from "@hara-lang/db-pglite";

const pglite = createPgliteProvider(PGlite);
serveBrowserProvider(
  (operation, args) => pglite.call("browser", operation, args),
  { errorCode: "db/pglite-error" }
);
