const escapeHtml = (value) => value
  .replaceAll("&", "&amp;")
  .replaceAll("<", "&lt;")
  .replaceAll(">", "&gt;");

export default function remarkHaraEval() {
  return (tree) => {
    const visit = (node) => {
      if (node?.type === "code" && /(?:^|\s)eval(?:\s|$)/.test(node.meta ?? "")) {
        const source = node.value ?? "";
        node.type = "html";
        node.value = `<section class="hara-eval-source" data-hara-eval data-hara-source="${encodeURIComponent(source)}"><pre><code>${escapeHtml(source)}</code></pre></section>`;
        delete node.lang;
        delete node.meta;
      }
      for (const child of node?.children ?? []) visit(child);
    };
    visit(tree);
  };
}
