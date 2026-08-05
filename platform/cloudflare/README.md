# Hara official service origins (superseded)

**Superseded.** `id.hara-lang.org` and `packages.hara-lang.org` are now served
by Netlify sites driven from the data repositories:

- `hara-lang/hara-identity` — branded landing page (`site/`) plus the
  `/.well-known/hara-tap.edn` and `/v1/identity` Netlify functions.
- `hara-lang/hara-packages` — branded landing page plus `/v1/registry` and
  the `/objects/sha256/*` R2 proxy functions (R2 credentials live as Netlify
  site env vars).

This Worker was the first cut of the same read-only edge and is kept for
reference. It is no longer deployed: the `deploy-cloudflare.yml` workflow was
removed so a stray push cannot re-attach the Worker custom domains and steal
the hostnames back. Do not redeploy this Worker for the two origins without
retiring the Netlify sites first.

The code remains a valid reference for the route contract:

- `id.hara-lang.org` serves the signed identity policy from authoritative Git.
- `packages.hara-lang.org` serves the accepted package registry from Git and
  immutable digest-addressed package objects from R2.

Verify the code with:

```shell
npm install
npm run types
npm run check
npm test
```

The `hara-cli` Worker (`cli.hara-lang.org`, installer endpoint) is a separate
deployment and is unaffected.
