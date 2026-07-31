# Hara official service origins

This is the deployable, read-only Cloudflare edge for the official Hara tap.
It exposes only the three public origins defined by the platform specs:

- `id.hara-lang.org` serves the signed identity policy from authoritative Git.
- `packages.hara-lang.org` serves the accepted package registry from Git and
  immutable digest-addressed package objects from R2.
- `assets.hara-lang.org` serves immutable digest-addressed media objects from
  a separate R2 bucket.

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
