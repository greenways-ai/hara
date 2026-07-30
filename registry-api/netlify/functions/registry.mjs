const REPOSITORIES = {
  identity: { repository: "hara-lang/hara-identity", path: "identity.edn" },
  registry: { repository: "hara-lang/hara-packages", path: "registry.edn" },
};

export function requestFor(event) {
  const kind = event.queryStringParameters?.kind;
  const ref = event.queryStringParameters?.ref;
  if (!REPOSITORIES[kind]) throw new Error("unknown kind");
  if (!ref || !/^(main|[0-9a-f]{40})$/.test(ref)) throw new Error("ref must be main or a 40-character commit");
  const target = REPOSITORIES[kind];
  return `https://api.github.com/repos/${target.repository}/contents/${target.path}?ref=${encodeURIComponent(ref)}`;
}

export default async (event) => {
  let url;
  try { url = requestFor(event); }
  catch (error) { return { statusCode: 400, body: JSON.stringify({ error: error.message }) }; }
  const response = await fetch(url, { headers: { Accept: "application/vnd.github+json", "User-Agent": "hara-registry-api" } });
  if (!response.ok) return { statusCode: response.status, body: JSON.stringify({ error: "upstream registry unavailable" }) };
  const source = await response.json();
  const body = Buffer.from(source.content, "base64").toString("utf8");
  return {
    statusCode: 200,
    headers: { "cache-control": event.queryStringParameters.ref === "main" ? "public, max-age=60" : "public, max-age=31536000, immutable", "content-type": "application/edn; charset=utf-8", "x-hara-upstream-sha": source.sha, "x-hara-interface": "advisory-read-only" },
    body,
  };
};
