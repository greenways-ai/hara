# Hara authentication worker

This Worker keeps GitHub user tokens out of the Studio. It implements the
GitHub App web flow with PKCE, stores the resulting token in an encrypted
`HttpOnly` cookie, and exposes only the current profile and the Gist routes
needed by Hara publishing.

Create a GitHub App with the callback URL
`https://auth.hara-lang.org/github/callback` and grant **Gists: read and
write** as an account permission. Then configure the Worker:

```sh
npx wrangler secret put GITHUB_CLIENT_ID --config auth/wrangler.jsonc
npx wrangler secret put GITHUB_CLIENT_SECRET --config auth/wrangler.jsonc
npx wrangler secret put SESSION_SECRET --config auth/wrangler.jsonc
npx wrangler deploy --config auth/wrangler.jsonc
```

`SESSION_SECRET` should be at least 32 random bytes. Configure
`auth.hara-lang.org` as a Worker custom domain, then set the website's
`hara-auth-api` meta tag to `https://auth.hara-lang.org`.

For local development, put the same three values in `auth/.dev.vars`; that
file is ignored and must never be committed.
