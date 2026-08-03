const COMMIT = /^[0-9a-f]{40}$/;
const DIGEST = /^[0-9a-f]{64}$/;

type RepositoryKind = "identity" | "packages";

export function repositoryRequest(kind: RepositoryKind, ref: string, env: Env): Request {
  if (ref !== "main" && !COMMIT.test(ref)) throw new Error("ref must be main or a 40-character commit");
  const repository = kind === "identity" ? env.IDENTITY_REPOSITORY : env.PACKAGES_REPOSITORY;
  const path = kind === "identity" ? "identity.edn" : "registry.edn";
  return new Request(`https://api.github.com/repos/${repository}/contents/${path}?ref=${ref}`, {
    headers: { accept: "application/vnd.github.raw+json", "user-agent": "hara-platform" },
  });
}

export function objectKey(pathname: string): string | null {
  const match = /^\/objects\/sha256\/([0-9a-f]{64})$/.exec(pathname);
  return match && DIGEST.test(match[1]) ? `sha256/${match[1]}` : null;
}

function edn(body: BodyInit | null, init: ResponseInit = {}): Response {
  const headers = new Headers(init.headers);
  headers.set("content-type", "application/edn; charset=utf-8");
  headers.set("x-content-type-options", "nosniff");
  return new Response(body, { ...init, headers });
}

function problem(status: number, code: string, message: string): Response {
  return edn(`{:error/code :${code} :error/message ${JSON.stringify(message)}}\n`, {
    status,
    headers: { "cache-control": "no-store" },
  });
}

async function gitDocument(kind: RepositoryKind, request: Request, env: Env): Promise<Response> {
  const ref = new URL(request.url).searchParams.get("ref") ?? "main";
  let upstream: Request;
  try {
    upstream = repositoryRequest(kind, ref, env);
  } catch (error) {
    return problem(400, "invalid-request", (error as Error).message);
  }
  const response = await fetch(upstream);
  if (!response.ok) {
    console.error(JSON.stringify({ event: "git-read-failed", kind, ref, status: response.status }));
    return problem(502, "upstream-unavailable", "authoritative Git document unavailable");
  }
  return edn(response.body, {
    headers: {
      "cache-control": ref === "main"
        ? "public, max-age=60"
        : "public, max-age=31536000, immutable",
      "x-hara-authority": "git",
    },
  });
}

async function storedObject(request: Request, bucket: R2Bucket): Promise<Response> {
  const key = objectKey(new URL(request.url).pathname);
  if (key === null) return problem(404, "not-found", "unknown object path");
  const object = await bucket.get(key);
  if (object === null) return problem(404, "not-found", "object not found");
  const headers = new Headers();
  object.writeHttpMetadata(headers);
  headers.set("etag", object.httpEtag);
  headers.set("cache-control", "public, max-age=31536000, immutable");
  headers.set("x-content-type-options", "nosniff");
  return new Response(object.body, { headers });
}

function discovery(): Response {
  return edn(
    '{:tap/name "hara" :tap/identity "https://id.hara-lang.org" :tap/registry "https://packages.hara-lang.org"}\n',
    { headers: { "cache-control": "public, max-age=3600" } },
  );
}

export async function route(request: Request, env: Env): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") {
    return problem(405, "method-not-allowed", "public service endpoints are read-only");
  }
  const url = new URL(request.url);
  if (url.pathname === "/.well-known/hara-tap.edn") return discovery();
  if (url.hostname === "id.hara-lang.org" && url.pathname === "/v1/identity") {
    return gitDocument("identity", request, env);
  }
  if (url.hostname === "packages.hara-lang.org" && url.pathname === "/v1/registry") {
    return gitDocument("packages", request, env);
  }
  if (url.hostname === "packages.hara-lang.org") return storedObject(request, env.OBJECTS);
  return problem(404, "not-found", "unknown Hara platform endpoint");
}
