const GITHUB_API = "https://api.github.com";
const DEVICE_CODE_URL = "https://github.com/login/device/code";
const ACCESS_TOKEN_URL = "https://github.com/login/oauth/access_token";
const encoder = new TextEncoder();

export async function workspaceBundle(repository, workspaceId) {
  const workspace = await repository.get(workspaceId);
  if (!workspace) throw new Error(`UNKNOWN_WORKSPACE ${workspaceId}`);
  const files = await repository.files(workspaceId);
  return {
    version: 1,
    workspace: { id: workspace.id, name: workspace.name, template: workspace.template },
    files: Object.fromEntries([...files].sort(([left], [right]) => left.localeCompare(right))),
    publishedAt: new Date().toISOString()
  };
}

export class GistPublisher {
  constructor({ request = githubRequest }) { this.request = request; }

  async publish(bundle, { public: visibility = true, previous = null } = {}) {
    const payload = {
      description: `Hara workspace: ${bundle.workspace.name}`,
      public: visibility,
      files: Object.fromEntries(Object.entries(bundle.files).map(([path, content]) => [
        path.replace(/^\//, "").replaceAll("/", "__"), { content }
      ]))
    };
    return previous?.id
      ? this.request(`/gists/${previous.id}`, { method: "PATCH", body: payload })
      : this.request("/gists", { method: "POST", body: payload });
  }
}

export class GitHubDeviceAuth {
  constructor({ clientId, request = oauthRequest, sleep = delay } = {}) {
    this.clientId = clientId;
    this.request = request;
    this.sleep = sleep;
  }

  configured() { return Boolean(this.clientId); }

  async begin() {
    if (!this.configured()) throw new Error("GITHUB_LOGIN_NOT_CONFIGURED");
    return this.request(DEVICE_CODE_URL, { client_id: this.clientId });
  }

  async authorize(device, { cancelled = () => false } = {}) {
    let interval = Math.max(1, Number(device.interval) || 5) * 1000;
    const expiresAt = Date.now() + (Number(device.expires_in) || 900) * 1000;
    while (Date.now() < expiresAt) {
      if (cancelled()) throw new Error("GITHUB_LOGIN_CANCELLED");
      await this.sleep(interval);
      const result = await this.request(ACCESS_TOKEN_URL, {
        client_id: this.clientId,
        device_code: device.device_code,
        grant_type: "urn:ietf:params:oauth:grant-type:device_code"
      });
      if (result.access_token) return result;
      if (result.error === "authorization_pending") continue;
      if (result.error === "slow_down") { interval += 5000; continue; }
      throw new Error(`GITHUB_LOGIN_${String(result.error || "FAILED").toUpperCase()}`);
    }
    throw new Error("GITHUB_LOGIN_EXPIRED");
  }
}

export async function githubRequest(path, { token, method = "GET", body } = {}) {
  const response = await fetch(`${GITHUB_API}${path}`, {
    method,
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      ...(body ? { "Content-Type": "application/json" } : {})
    },
    body: body ? JSON.stringify(body) : undefined
  });
  const result = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(result.message || `GITHUB_API_${response.status}`);
  return result;
}

async function oauthRequest(url, values) {
  const response = await fetch(url, {
    method: "POST",
    headers: { Accept: "application/json", "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams(values)
  });
  const result = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(result.error || `GITHUB_OAUTH_${response.status}`);
  return result;
}

const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

export function zipWorkspace(bundle) {
  const entries = Object.entries(bundle.files).map(([path, content]) => ({
    name: path.replace(/^\/+/, ""), bytes: encoder.encode(content)
  }));
  const local = [];
  const central = [];
  let offset = 0;
  for (const entry of entries) {
    const name = encoder.encode(entry.name);
    const crc = crc32(entry.bytes);
    const header = concat([
      u32(0x04034b50), u16(20), u16(0x0800), u16(0), u16(0), u16(0), u32(crc),
      u32(entry.bytes.length), u32(entry.bytes.length), u16(name.length), u16(0), name, entry.bytes
    ]);
    local.push(header);
    central.push(concat([
      u32(0x02014b50), u16(20), u16(20), u16(0x0800), u16(0), u16(0), u16(0), u32(crc),
      u32(entry.bytes.length), u32(entry.bytes.length), u16(name.length), u16(0), u16(0), u16(0), u16(0),
      u32(0), u32(offset), name
    ]));
    offset += header.length;
  }
  const centralBytes = concat(central);
  return concat([...local, centralBytes, concat([
    u32(0x06054b50), u16(0), u16(0), u16(entries.length), u16(entries.length),
    u32(centralBytes.length), u32(offset), u16(0)
  ])]);
}

export function downloadWorkspace(bundle, { save = saveBlob } = {}) {
  const filename = `${bundle.workspace.id}.zip`;
  save(new Blob([zipWorkspace(bundle)], { type: "application/zip" }), filename);
  return filename;
}

function saveBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

function concat(parts) {
  const length = parts.reduce((sum, part) => sum + part.length, 0);
  const output = new Uint8Array(length);
  let offset = 0;
  for (const part of parts) { output.set(part, offset); offset += part.length; }
  return output;
}

function u16(value) { return Uint8Array.of(value & 255, (value >>> 8) & 255); }
function u32(value) { return Uint8Array.of(value & 255, (value >>> 8) & 255, (value >>> 16) & 255, (value >>> 24) & 255); }

function crc32(bytes) {
  let value = -1;
  for (const byte of bytes) {
    value ^= byte;
    for (let bit = 0; bit < 8; bit += 1) value = (value >>> 1) ^ (value & 1 ? 0xedb88320 : 0);
  }
  return (value ^ -1) >>> 0;
}
