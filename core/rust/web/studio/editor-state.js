const PAIRS = { "(": ")", "[": "]", "{": "}" };
const CLOSERS = new Set(Object.values(PAIRS));

/** Return complete top-level source forms, retaining their editor ranges.
 * Comments and whitespace belong to neither adjacent form. An incomplete
 * final form is retained so the evaluator can report its syntax error. */
export function editorTopLevelForms(source) {
  const forms = [];
  let cursor = 0;
  while (cursor < source.length) {
    cursor = skipTrivia(source, cursor);
    if (cursor >= source.length) break;
    const start = cursor;
    cursor = scanForm(source, cursor);
    forms.push({ start, end: cursor, source: source.slice(start, cursor) });
  }
  return forms;
}

function skipTrivia(source, cursor) {
  while (cursor < source.length) {
    if (/\s/.test(source[cursor])) { cursor += 1; continue; }
    if (source[cursor] !== ";") return cursor;
    while (cursor < source.length && source[cursor] !== "\n") cursor += 1;
  }
  return cursor;
}

function scanForm(source, start) {
  const first = source[start];
  if (Object.hasOwn(PAIRS, first)) return scanCollection(source, start);
  if (first === '"') return scanString(source, start);
  if (first === "'" || first === "`" || first === "~" || first === "^" || first === "@") {
    return scanForm(source, skipTrivia(source, start + 1));
  }
  if (first === "#" && Object.hasOwn(PAIRS, source[start + 1])) return scanCollection(source, start + 1);
  let cursor = start;
  while (cursor < source.length && !/\s/.test(source[cursor]) && !Object.hasOwn(PAIRS, source[cursor]) && !CLOSERS.has(source[cursor])) {
    cursor += 1;
  }
  return cursor === start ? start + 1 : cursor;
}

function scanString(source, start) {
  let cursor = start + 1;
  let escaped = false;
  while (cursor < source.length) {
    const character = source[cursor++];
    if (!escaped && character === '"') break;
    escaped = !escaped && character === "\\";
  }
  return cursor;
}

function scanCollection(source, start) {
  const stack = [PAIRS[source[start]]];
  let cursor = start + 1;
  let inString = false;
  let inComment = false;
  let escaped = false;
  while (cursor < source.length && stack.length) {
    const character = source[cursor++];
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
    if (Object.hasOwn(PAIRS, character)) { stack.push(PAIRS[character]); continue; }
    if (character === stack.at(-1)) stack.pop();
  }
  return cursor;
}

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

/** True when strings and collection delimiters are balanced enough to hand
 * the whole document to the evaluator without flashing an incomplete error. */
export function editorSourceComplete(source) {
  const stack = [];
  let inString = false;
  let inComment = false;
  let escaped = false;
  for (const character of source) {
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
    if (Object.hasOwn(PAIRS, character)) stack.push(PAIRS[character]);
    else if (CLOSERS.has(character)) {
      if (stack.pop() !== character) return false;
    }
  }
  return !inString && stack.length === 0;
}
