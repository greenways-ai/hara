const escapeHtml = (value) => value
  .replaceAll("&", "&amp;")
  .replaceAll("<", "&lt;")
  .replaceAll(">", "&gt;");

const escapeAttribute = (value) => escapeHtml(String(value)).replaceAll('"', "&quot;");

export function parseHaraEvalScope(meta = "") {
  const group = String(meta).match(/(?:^|\s)group(?:\s*=\s*|\s*:\s*|\s+)(?:"([^"]+)"|'([^']+)'|([^\s]+))/i);
  const groupName = group?.[1] ?? group?.[2] ?? group?.[3] ?? "";
  if (groupName.trim()) return { scope: "group", groupName: groupName.trim() };
  if (/(?:^|\s)global(?:\s|$)/i.test(meta)) return { scope: "global", groupName: "" };
  return { scope: "isolated", groupName: "" };
}

export default function remarkHaraEval() {
  return (tree) => {
    const visit = (node) => {
      if (node?.type === "code" && /(?:^|\s)eval(?:\s|$)/.test(node.meta ?? "")) {
        const source = node.value ?? "";
        const { scope, groupName } = parseHaraEvalScope(node.meta);
        const groupAttribute = groupName
          ? ` data-hara-group="${escapeAttribute(groupName)}"`
          : "";
        node.type = "html";
        node.value = `<section class="hara-eval-source" data-hara-eval data-hara-source="${encodeURIComponent(source)}" data-hara-scope="${scope}"${groupAttribute}><pre><code>${escapeHtml(source)}</code></pre></section>`;
        delete node.lang;
        delete node.meta;
      }
      for (const child of node?.children ?? []) visit(child);
    };
    visit(tree);
  };
}
