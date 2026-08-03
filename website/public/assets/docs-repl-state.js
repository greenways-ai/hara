const normalizePart = (value, fallback) => {
  const normalized = String(value ?? "")
    .trim()
    .replace(/^\/+|\/+$/g, "")
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .toLowerCase();
  return normalized || fallback;
};

export function describeDocsSession({ scope = "isolated", groupName = "", pagePath = "/", sequence = 1 } = {}) {
  const requestedScope = String(scope).trim().toLowerCase();
  const normalizedGroup = String(groupName ?? "").trim();
  const page = normalizePart(pagePath, "home");
  const base = page === "home" ? "docs-home" : page.startsWith("docs-") ? page : `docs-${page}`;

  if (requestedScope === "global") {
    const id = `${base}-global`;
    return { scope: "global", label: "global", id, filesystem: `memory:${id}`, sharedWith: "all global runners on this page" };
  }

  if (requestedScope === "group" && normalizedGroup) {
    const group = normalizePart(normalizedGroup, "group");
    const id = `${base}-group-${group}`;
    return {
      scope: "group",
      groupName: normalizedGroup,
      label: `group ${normalizedGroup}`,
      id,
      filesystem: `memory:${id}`,
      sharedWith: `group ${normalizedGroup} on this page`
    };
  }

  const id = `${base}-${Math.max(1, Number(sequence) || 1)}`;
  return { scope: "isolated", label: "isolated", id, filesystem: `memory:${id}`, sharedWith: "this runner only" };
}

export function createDocsSessionRegistry(kernelPromise) {
  const sessions = new Map();
  return {
    get(descriptor) {
      const existing = sessions.get(descriptor.id);
      if (existing) return existing;
      const pending = Promise.resolve(kernelPromise).then((kernel) =>
        kernel.createSession(descriptor.id, { filesystem: descriptor.filesystem }));
      sessions.set(descriptor.id, pending);
      pending.catch(() => {
        if (sessions.get(descriptor.id) === pending) sessions.delete(descriptor.id);
      });
      return pending;
    }
  };
}
