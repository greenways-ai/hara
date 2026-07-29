const PAIRS = { "(": ")", "[": "]", "{": "}" };
const CLOSERS = new Set(Object.values(PAIRS));

function formsIn(source) {
  const forms = [];
  const stack = [];
  let inString = false;
  let inComment = false;
  let escaped = false;
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (inComment) {
      if (character === "\n") inComment = false;
      continue;
    }
    if (inString) {
      if (!escaped && character === '"') inString = false;
      escaped = !escaped && character === "\\";
      continue;
    }
    if (character === ";") { inComment = true; continue; }
    if (character === '"') { inString = true; escaped = false; continue; }
    if (Object.hasOwn(PAIRS, character)) stack.push({ opener: character, start: index });
    if (CLOSERS.has(character) && stack.length && PAIRS[stack.at(-1).opener] === character) {
      const form = stack.pop();
      forms.push({ start: form.start, end: index + 1 });
    }
  }
  return forms;
}

/** Select the editor's explicit selection or the innermost complete form at the caret. */
export function editorFormAt(source, selectionStart, selectionEnd = selectionStart) {
  if (selectionEnd > selectionStart) {
    return {
      start: selectionStart,
      end: selectionEnd,
      source: source.slice(selectionStart, selectionEnd)
    };
  }
  const caret = selectionStart;
  const forms = formsIn(source);
  if (/\s/.test(source[caret] ?? "")) {
    const previous = forms
      .filter((form) => form.end <= caret)
      .sort((left, right) => right.end - left.end || (left.end - left.start) - (right.end - right.start))[0];
    if (previous) return { ...previous, source: source.slice(previous.start, previous.end) };
  }
  const enclosing = forms
    .filter((form) => form.start <= caret && caret <= form.end)
    .sort((left, right) => (left.end - left.start) - (right.end - right.start))[0];
  if (enclosing) return { ...enclosing, source: source.slice(enclosing.start, enclosing.end) };
  const previous = forms
    .filter((form) => form.end <= caret)
    .sort((left, right) => right.end - left.end)[0];
  if (previous) return { ...previous, source: source.slice(previous.start, previous.end) };
  return null;
}

export function studioDocumentId({ projectId = "document", space, path }) {
  if (!space || !path) throw new Error("INVALID_DOCUMENT_ID");
  return `${projectId}:${space}:${path}`;
}

export function isAnonymousDocument(source) {
  return /^\s*(?:;[^\n]*\n\s*)*\(ns\+/.test(source);
}
