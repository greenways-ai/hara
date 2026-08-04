# Hara official service origins

This is the deployable, read-only Cloudflare edge for the official Hara tap.
It exposes only the two public origins defined by the platform specs:

- `id.hara-lang.org` serves the signed identity policy from authoritative Git.
- `packages.hara-lang.org` serves the accepted package registry from Git and
  immutable digest-addressed package objects from R2.
Artifact, HARP, extension, publishing, distribution, and mirroring remain
protocol components. They do not receive additional official domains.

The Worker has no mutation route and no GitHub write credential. Protected
GitHub workflows perform enrollment and publication finalization, write
accepted records to Git, and upload objects to R2. This preserves Git as the
authority and prevents the public delivery edge from becoming a second
registry.

Generate bindings and verify before deployment:

```shell
npm install
npm run types
npm run check
npm test
```

R2 uploads must use `sha256/<lowercase digest>` keys. Configure credentials
with Wrangler secrets or GitHub environment secrets; never add them to
`wrangler.jsonc`.

## Deploy

`.github/workflows/deploy-cloudflare.yml` (main repo) verifies and deploys the
Worker on pushes to `main` that touch `platform/cloudflare/**`, or via manual
dispatch. It expects the `cloudflare` GitHub environment to provide:

- `CLOUDFLARE_API_TOKEN` — token with Workers Scripts:Edit and R2:Edit
- `CLOUDFLARE_ACCOUNT_ID` — the account holding the `hara-lang.org` zone

One-time setup before the first deploy:

1. Create the object bucket: `npx wrangler r2 bucket create hara-objects`.
2. Detach `id.hara-lang.org` / `packages.hara-lang.org` from any existing
   hosting (GitHub Pages custom domains on the `hara-identity` /
   `hara-packages` repos, Netlify sites) so the zone records can move.
3. Run the workflow. Wrangler attaches both custom domains, provisions
   certificates, and rewrites the zone DNS records automatically; the
   `hara-lang.org` zone must live in the same Cloudflare account.
