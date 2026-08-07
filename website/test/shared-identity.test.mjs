import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const layout = await readFile(new URL("../src/layouts/SiteLayout.astro", import.meta.url), "utf8");
const config = await readFile(new URL("../astro.config.mjs", import.meta.url), "utf8");
const loader = await readFile(new URL("../public/assets/identity-loader.js", import.meta.url), "utf8");
const verifier = await readFile(new URL("../../.github/scripts/verify-shared-identity.sh", import.meta.url), "utf8");
const workflow = await readFile(new URL("../../.github/workflows/pages-www.yml", import.meta.url), "utf8");

test("backports the shared popup account mount without replacing reviewed homepage assets", () => {
  assert.match(layout, /<meta name="hara-identity-mode" content="popup" \/>/);
  assert.match(layout, /data-hara-identity/);
  assert.match(layout, /data-hara-identity-fallback/);
  assert.match(layout, /aria-label="Sign in with GitHub"/);
  assert.match(layout, /\/assets\/identity-loader\.js/);
  assert.match(layout, /\/assets\/install-copy\.js/);
  assert.match(layout, /\/assets\/install-copy\.css/);
  assert.doesNotMatch(layout, /\/auth\/github\/callback|HARA_AUTH_SESSION_SECRET|HARA_GITHUB_OAUTH_CLIENT_SECRET/);
});

test("selects the matching Identity issuer and preserves a useful fallback", () => {
  assert.match(loader, /https:\/\/id\.testing\.hara-lang\.org/);
  assert.match(loader, /https:\/\/id\.hara-lang\.org/);
  assert.match(loader, /location\.hostname === "www\.testing\.hara-lang\.org"/);
  assert.match(loader, /endsWith\("\.testing\.hara-lang\.org"\)/);
  assert.match(loader, /new URL\("\/github\/start", identityOrigin\)/);
  assert.match(loader, /url\.searchParams\.set\("returnTo", location\.href\)/);
  assert.match(loader, /data-hara-identity-fallback/);
  assert.match(loader, /\/v1\/identity-client\.js/);
  assert.match(loader, /meta\[name="hara-identity-auto"\]/);
  assert.doesNotMatch(loader, /access_token|client_secret|sessionStorage|localStorage/);
});

test("injects the same account control into Starlight Docs", () => {
  assert.match(config, /name: "hara-identity-auto"/);
  assert.match(config, /src: "\/assets\/identity-loader\.js"/);
  assert.match(config, /defer: true/);
});

test("verifies the deployed popup www and Docs boundary against Identity contract v1", () => {
  assert.match(verifier, /hara-identity-mode/);
  assert.match(verifier, /hara_identity_popup/);
  assert.match(verifier, /\/docs\/start\/orientation\//);
  assert.match(verifier, /hara-identity-auto/);
  assert.match(verifier, /\.contractVersion == 1/);
  assert.match(verifier, /\.clientVersion == 1/);
  assert.match(verifier, /Access-Control-Allow-Origin/);
  assert.match(verifier, /https:\/\/untrusted\.example/);
  assert.match(verifier, /v1\/identity-client\.js/);
});

test("deploys testing first and promotes only an explicit exact prod commit", () => {
  assert.match(workflow, /promote_production:/);
  assert.match(workflow, /tested_sha:/);
  assert.match(workflow, /remote_prod="\$\(git ls-remote origin refs\/heads\/prod/);
  assert.match(workflow, /hara-identity-mode/);
  assert.match(workflow, /Upload immutable website artifact/);
  assert.match(workflow, /actions\/upload-artifact@v4/);
  assert.match(workflow, /deploy-testing:/);
  assert.match(workflow, /Verify testing shared identity/);
  assert.match(workflow, /deploy-production:/);
  assert.match(workflow, /inputs\.promote_production == true/);
  assert.match(workflow, /environment:[\s\S]*name: hara-www-production/);
  assert.match(workflow, /Download the tested website artifact/);
  assert.match(workflow, /Verify production shared identity/);
  assert.match(workflow, /HARA_SITE_ORIGIN: https:\/\/www\.hara-lang\.org/);
  assert.match(workflow, /HARA_IDENTITY_ORIGIN: https:\/\/id\.hara-lang\.org/);
});
