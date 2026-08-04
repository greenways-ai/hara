function html(value) {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

/**
 * Dependency-free Hara syntax highlighter. Returns HTML with each token
 * wrapped in a span (`comment`, `string`, `keyword`, `paren-N` cycling with
 * nesting depth, `unmatched`). When `evalRange` is supplied, characters in
 * that range additionally carry the `eval-target` class.
 *
 * Extracted from website/app.js `highlightHara`; the eval-range lookup that
 * app.js reads from module state is an explicit option here.
 */
export function highlightHara(source, { evalRange = null } = {}) {
  let output = "";
  let depth = 0;
  let string = false;
  let comment = false;
  let escaped = false;
  const target = (index) => evalRange && index >= evalRange.start && index < evalRange.end ? " eval-target" : "";
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (comment) { output += `<span class="comment${target(index)}">${html(character)}</span>`; if (character === "\n") comment = false; continue; }
    if (string) {
      output += `<span class="string${target(index)}">${html(character)}</span>`;
      if (!escaped && character === '"') string = false;
      escaped = !escaped && character === "\\";
      continue;
    }
    if (character === ";") { comment = true; output += `<span class="comment${target(index)}">;</span>`; continue; }
    if (character === '"') { string = true; escaped = false; output += `<span class="string${target(index)}">"</span>`; continue; }
    if ("([{".includes(character)) { output += `<span class="paren-${depth % 6}${target(index)}">${character}</span>`; depth += 1; continue; }
    if (")]}".includes(character)) {
      depth -= 1;
      output += `<span class="${depth < 0 ? "unmatched" : `paren-${depth % 6}`}${target(index)}">${character}</span>`;
      continue;
    }
    if (character === ":") {
      const match = source.slice(index).match(/^:[A-Za-z*+!?._/-]+/);
      if (match) { output += `<span class="keyword${target(index)}">${html(match[0])}</span>`; index += match[0].length - 1; continue; }
    }
    output += target(index) ? `<span class="eval-target">${html(character)}</span>` : html(character);
  }
  return output;
}
