# Hara registry API

This Netlify service is a read-only interface for the official Hara package
registry. It reads public immutable content from the canonical GitHub
repositories; it is not a package publisher, identity authority, Git mirror,
or artifact verifier.

## Endpoints

```text
GET /v1/registry?kind=registry&ref=<main|40-character-commit>
GET /v1/identity?kind=identity&ref=<main|40-character-commit>
```

The response carries the upstream Git blob SHA and `advisory-read-only`
marker. Clients must verify the locked Git commit, policy, and archive digest
independently; this API is only a cache/discovery source.

## Deploy

Create `api-hara-lang-org` in the Statstrade Netlify team, assign
`api.hara-lang.org`, then configure the GitHub repository secrets
`NETLIFY_AUTH_TOKEN` and `NETLIFY_API_HARA_LANG_ORG_SITE_ID`. GitHub Actions
deploys previews for pull requests and production from `main`; developers do
not deploy through the Netlify CLI. Do not configure a GitHub write token,
publisher signing key, release credential, or Netlify write endpoint.
Public GitHub content reads are intentional. If rate limits require a token,
use a read-only token scoped only to the two public Hara repositories.
