const DEFAULT_RETURN = () => `${location.origin}${location.pathname}`;

export class GitHubAuthClient {
  constructor({ baseUrl = "", fetch = globalThis.fetch?.bind(globalThis), navigate = (url) => location.assign(url) } = {}) {
    this.baseUrl = String(baseUrl).replace(/\/$/, "");
    this.fetch = fetch;
    this.navigate = navigate;
  }

  get available() { return Boolean(this.baseUrl && this.fetch); }

  async session() {
    if (!this.available) return { authenticated: false, configured: false, profile: null };
    const response = await this.fetch(`${this.baseUrl}/session`, {
      credentials: "include",
      headers: { Accept: "application/json" }
    });
    if (response.status === 401) return { authenticated: false, configured: true, profile: null };
    if (!response.ok) throw new Error(`GITHUB_AUTH_SESSION ${response.status}`);
    const value = await response.json();
    return {
      authenticated: Boolean(value.authenticated),
      configured: value.configured !== false,
      profile: value.profile ?? null
    };
  }

  signIn(returnTo = DEFAULT_RETURN()) {
    if (!this.available) throw new Error("GITHUB_AUTH_NOT_CONFIGURED");
    const url = new URL(`${this.baseUrl}/github/start`);
    url.searchParams.set("returnTo", returnTo);
    this.navigate(url.href);
  }

  async signOut() {
    if (!this.available) return;
    const response = await this.fetch(`${this.baseUrl}/logout`, {
      method: "POST",
      credentials: "include",
      headers: { "X-Hara-Request": "sign-out" }
    });
    if (!response.ok) throw new Error(`GITHUB_AUTH_LOGOUT ${response.status}`);
  }

  async request(path, { method = "GET", body } = {}) {
    if (!this.available) throw new Error("GITHUB_AUTH_NOT_CONFIGURED");
    const response = await this.fetch(`${this.baseUrl}${path}`, {
      method,
      credentials: "include",
      headers: { Accept: "application/json", "Content-Type": "application/json", "X-Hara-Request": "studio" },
      body: body === undefined ? undefined : JSON.stringify(body)
    });
    if (response.status === 401) throw new Error("GITHUB_AUTH_REQUIRED");
    if (!response.ok) throw new Error(`GITHUB_API ${response.status}`);
    return response.json();
  }
}

export function authBaseFromDocument(document = globalThis.document) {
  return document?.querySelector('meta[name="hara-auth-api"]')?.content?.trim() ?? "";
}
