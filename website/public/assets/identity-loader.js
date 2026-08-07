(() => {
  "use strict";

  if (globalThis.HaraIdentityLoader) return;

  const configured = document.querySelector('meta[name="hara-identity-origin"]')?.content?.trim();
  let identityOrigin = "";
  if (configured) {
    try { identityOrigin = new URL(configured, location.href).origin; }
    catch {}
  }

  if (!identityOrigin) {
    const testing = location.hostname === "www.testing.hara-lang.org"
      || location.hostname.endsWith(".testing.hara-lang.org");
    identityOrigin = testing
      ? "https://id.testing.hara-lang.org"
      : "https://id.hara-lang.org";
  }

  function installFallbackStyles() {
    if (document.getElementById("hara-identity-fallback-style")) return;
    const style = document.createElement("style");
    style.id = "hara-identity-fallback-style";
    style.textContent = `
      [data-hara-identity]{display:inline-flex;align-items:center}
      [data-hara-identity-fallback]{display:inline-flex;align-items:center;justify-content:center;min-height:36px;padding:.42rem .7rem;color:var(--hara-text,var(--sl-color-text,#f2f4f7));background:var(--hara-surface,var(--sl-color-bg-nav,#11151b));border:1px solid var(--hara-line,var(--sl-color-gray-5,#303640));border-radius:999px;text-decoration:none;font:620 .78rem var(--hara-font-sans,var(--hara-display,ui-sans-serif,system-ui,sans-serif));white-space:nowrap}
      [data-hara-identity-fallback]:hover{background:var(--hara-surface-raised,var(--sl-color-gray-6,#171c24));border-color:var(--hara-line-strong,var(--sl-color-gray-4,#48515e))}
      [data-hara-identity-fallback]:focus-visible{outline:2px solid var(--hara-signal,var(--hara-cyan,#41f5e4));outline-offset:2px}
    `;
    document.head.append(style);
  }

  function automaticMount() {
    if (!document.querySelector('meta[name="hara-identity-auto"]')) return null;
    return document.querySelector(".header .right-group")
      || document.querySelector("header .right-group")
      || document.querySelector("header [data-theme-toggle]")?.parentElement
      || document.querySelector("header");
  }

  function ensureRoot() {
    const existing = document.querySelector("[data-hara-identity]");
    if (existing) return existing;
    const mount = automaticMount();
    if (!mount) return null;
    const root = document.createElement("div");
    root.dataset.haraIdentity = "";
    mount.prepend(root);
    return root;
  }

  function signInUrl() {
    const url = new URL("/github/start", identityOrigin);
    url.searchParams.set("returnTo", location.href);
    return url.href;
  }

  function ensureFallback(root) {
    if (!root || root.querySelector("[data-hara-identity-account]")) return;
    let fallback = root.querySelector("[data-hara-identity-fallback]");
    if (!(fallback instanceof HTMLAnchorElement)) {
      fallback = document.createElement("a");
      fallback.dataset.haraIdentityFallback = "";
      fallback.textContent = "Sign in";
      fallback.setAttribute("aria-label", "Sign in with GitHub");
      root.replaceChildren(fallback);
    }
    fallback.href = signInUrl();
  }

  function loadClient() {
    installFallbackStyles();
    document.querySelectorAll("[data-hara-identity]").forEach(ensureFallback);
    ensureFallback(ensureRoot());

    if (document.querySelector("script[data-hara-identity-client]")) return;
    const client = document.createElement("script");
    client.src = `${identityOrigin}/v1/identity-client.js`;
    client.defer = true;
    client.dataset.haraIdentityClient = "";
    document.head.append(client);
  }

  const initialise = () => loadClient();
  document.addEventListener("astro:page-load", initialise);
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initialise, { once: true });
  } else {
    initialise();
  }

  globalThis.HaraIdentityLoader = Object.freeze({
    origin: identityOrigin,
    refresh: initialise,
  });
})();
