const STORAGE_KEY = "hara-theme";
const VALID = new Set(["system", "light", "dark"]);

function isHaraDomain(hostname) {
  return hostname === "hara-lang.org" || hostname.endsWith(".hara-lang.org");
}

function readCookie() {
  const entry = document.cookie.split("; ").find((part) => part.startsWith(`${STORAGE_KEY}=`));
  return entry ? decodeURIComponent(entry.split("=").slice(1).join("=")) : null;
}

export function getThemePreference() {
  let value = null;
  try { value = localStorage.getItem(STORAGE_KEY); } catch {}
  value ||= readCookie();
  return VALID.has(value) ? value : "system";
}

export function resolvedTheme(preference = getThemePreference()) {
  return preference === "system"
    ? (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
    : preference;
}

export function applyTheme(preference = getThemePreference(), notify = false) {
  const safe = VALID.has(preference) ? preference : "system";
  const resolved = resolvedTheme(safe);
  document.documentElement.dataset.theme = resolved;
  document.documentElement.dataset.themePreference = safe;
  if (notify) dispatchEvent(new CustomEvent("hara:theme-change", { detail: { preference: safe, resolved } }));
  return resolved;
}

export function setThemePreference(preference) {
  const safe = VALID.has(preference) ? preference : "system";
  try { localStorage.setItem(STORAGE_KEY, safe); } catch {}
  const domain = isHaraDomain(location.hostname) ? "; Domain=hara-lang.org; Secure" : "";
  document.cookie = `${STORAGE_KEY}=${encodeURIComponent(safe)}; Path=/; Max-Age=31536000; SameSite=Lax${domain}`;
  return applyTheme(safe, true);
}

export function cycleTheme() {
  const order = ["system", "light", "dark"];
  const current = getThemePreference();
  const next = order[(order.indexOf(current) + 1) % order.length];
  setThemePreference(next);
  return next;
}

if (typeof window !== "undefined") {
  applyTheme();
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if (getThemePreference() === "system") applyTheme("system", true);
  });
}
