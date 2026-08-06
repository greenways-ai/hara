(() => {
  "use strict";

  if (document.querySelector("script[data-hara-identity-client]")) return;

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

  const client = document.createElement("script");
  client.src = `${identityOrigin}/identity-client.js`;
  client.defer = true;
  client.dataset.haraIdentityClient = "";
  document.head.append(client);
})();
