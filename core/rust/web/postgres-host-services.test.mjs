import assert from "node:assert/strict";
import test from "node:test";
import { createPostgresHostServices } from "./packages/db-postgres/index.mjs";

function provider(name) {
  return {
    calls: [],
    async call(environment, operation, args) {
      this.calls.push([environment, operation, args]);
      if (operation === "open") return { id: 7, engine: "postgresql", provider: name, mode: name === "pglite" ? "embedded" : "remote", capabilities: ["sql"] };
      if (operation === "listen") return { id: 9, channel: args[1] };
      if (operation === "close" || operation === "unlisten") return true;
      return { operation, args };
    }
  };
}

test("browser auto-selection uses PGlite and keeps handles opaque", async () => {
  const pglite = provider("pglite");
  const services = createPostgresHostServices({ pglite, environment: "browser" });
  const opened = await services["std.db.postgres/open"](new Map());
  assert.equal(opened.provider, "pglite");
  assert.equal(opened.id, 1);
  const decode = new Map([["decode", "tagged"]]);
  await services["std.db.postgres/query"](opened.id, "select 1", [], decode);
  assert.equal(pglite.calls.at(-1)[1], "query-options");
  assert.deepEqual(pglite.calls.at(-1)[2], [7, "select 1", [], decode]);
  assert.equal(await services["std.db.postgres/close"](opened.id), true);
  await assert.rejects(services["std.db.postgres/query"](opened.id, "select 1", []), /postgres\/connection-closed/);
});

test("remote options select the injected on-prem transport", async () => {
  const pglite = provider("pglite");
  const remote = provider("postgres");
  const services = createPostgresHostServices({ pglite, remote, environment: "browser" });
  const opened = await services["std.db.postgres/open"](new Map([["endpoint", "https://hestia.local/db"]]));
  assert.equal(opened.provider, "postgres");
  assert.equal(remote.calls[0][1], "open");
  assert.equal(pglite.calls.length, 0);
});

test("conflicting provider indicators and unavailable remote providers reject", async () => {
  const services = createPostgresHostServices({ pglite: provider("pglite") });
  await assert.rejects(
    services["std.db.postgres/open"](new Map([["host", "db"], ["storage", "memory"]])),
    /postgres\/config-invalid/
  );
  await assert.rejects(
    services["std.db.postgres/open"](new Map([["endpoint", "https://hestia.local/db"]])),
    /postgres\/provider-unavailable/
  );
});
