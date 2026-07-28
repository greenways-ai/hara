export async function workspaceBundle(repository, workspaceId) {
  const workspace = await repository.get(workspaceId);
  if (!workspace) throw new Error(`UNKNOWN_WORKSPACE ${workspaceId}`);
  const files = await repository.files(workspaceId);
  return {
    version: 1,
    workspace: {
      id: workspace.id,
      name: workspace.name,
      template: workspace.template
    },
    files: Object.fromEntries([...files].sort(([left], [right]) => left.localeCompare(right))),
    publishedAt: new Date().toISOString()
  };
}

export class GistPublisher {
  constructor({ request }) {
    this.request = request;
  }

  async publish(bundle, { public: visibility = true, previous = null } = {}) {
    const payload = {
      description: `Hara workspace: ${bundle.workspace.name}`,
      public: visibility,
      files: Object.fromEntries(Object.entries(bundle.files).map(([path, content]) => [
        path.replace(/^\//, "").replaceAll("/", "__"),
        { content }
      ]))
    };
    return previous?.id
      ? this.request(`/gists/${previous.id}`, { method: "PATCH", body: payload })
      : this.request("/gists", { method: "POST", body: payload });
  }
}

export class GreenwaysPublisher {
  constructor({ request }) {
    this.request = request;
  }

  publish(bundle, { public: visibility = true, previous = null } = {}) {
    return this.request(previous?.id ? `/works/${previous.id}` : "/works", {
      method: previous?.id ? "PUT" : "POST",
      body: { ...bundle, visibility: visibility ? "public" : "draft", githubProfile: true }
    });
  }
}

export function downloadWorkspace(bundle) {
  const blob = new Blob([JSON.stringify(bundle, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `${bundle.workspace.id}.hara-workspace.json`;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}
