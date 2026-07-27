const PAIRS = { "(": ")", "[": "]", "{": "}" };
const CLOSERS = new Set(Object.values(PAIRS));

function replace(editor, start, end, text, selection = "end") {
  editor.setRangeText(text, start, end, selection);
  editor.dispatchEvent(new Event("input", { bubbles: true }));
}

function lineStart(source, offset) {
  return source.lastIndexOf("\n", offset - 1) + 1;
}

function indentation(source, offset) {
  return source.slice(lineStart(source, offset), offset).match(/^\s*/)?.[0] ?? "";
}

/**
 * A small, dependency-free structural editing layer for a textarea.
 * It follows the core Paredit invariants: delimiters are kept balanced and
 * an existing closing delimiter is never accidentally duplicated.
 */
export function applyParedit(editor, key) {
  const { value, selectionStart: start, selectionEnd: end } = editor;
  if (Object.hasOwn(PAIRS, key)) {
    const close = PAIRS[key];
    if (start !== end) {
      replace(editor, start, end, `${key}${value.slice(start, end)}${close}`, "select");
      editor.setSelectionRange(start + 1, end + 1);
    } else {
      replace(editor, start, end, `${key}${close}`);
      editor.setSelectionRange(start + 1, start + 1);
    }
    return true;
  }

  if (CLOSERS.has(key)) {
    if (start === end && value[start] === key) editor.setSelectionRange(start + 1, start + 1);
    else replace(editor, start, end, key);
    return true;
  }

  if (key === "Backspace" && start === end && start > 0 && PAIRS[value[start - 1]] === value[start]) {
    replace(editor, start - 1, start + 1, "");
    return true;
  }

  if (key === "Enter") {
    const before = value.slice(0, start);
    const previous = before.trimEnd().at(-1);
    const indent = indentation(value, start) + (Object.hasOwn(PAIRS, previous) ? "  " : "");
    replace(editor, start, end, `\n${indent}`);
    return true;
  }
  return false;
}

export function insertIndent(editor, unindent = false) {
  const { value, selectionStart: start, selectionEnd: end } = editor;
  if (!unindent) {
    replace(editor, start, end, "  ");
    return;
  }
  const from = lineStart(value, start);
  const to = value.indexOf("\n", end) === -1 ? value.length : value.indexOf("\n", end);
  const selected = value.slice(from, to);
  const next = selected.replace(/^ {1,2}/gm, "");
  replace(editor, from, to, next, "select");
  editor.setSelectionRange(from, from + next.length);
}

function nestingBefore(source, limit) {
  const stack = [];
  let inString = false;
  let inComment = false;
  let escaped = false;
  for (let index = 0; index < limit; index += 1) {
    const character = source[index];
    if (inComment) { if (character === "\n") inComment = false; continue; }
    if (inString) {
      if (!escaped && character === '"') inString = false;
      escaped = !escaped && character === "\\";
      continue;
    }
    if (character === ";") { inComment = true; continue; }
    if (character === '"') { inString = true; escaped = false; continue; }
    if (Object.hasOwn(PAIRS, character)) stack.push(character);
    else if (CLOSERS.has(character) && PAIRS[stack.at(-1)] === character) stack.pop();
  }
  return stack.length;
}

/** Align the current line to its enclosing structural delimiter depth. */
export function structuralAlign(editor) {
  const { value, selectionStart: start, selectionEnd: end } = editor;
  const from = lineStart(value, start);
  const lineEnd = value.indexOf("\n", from) === -1 ? value.length : value.indexOf("\n", from);
  const line = value.slice(from, lineEnd);
  const content = line.trimStart();
  let depth = nestingBefore(value, from);
  if (content && CLOSERS.has(content[0])) depth = Math.max(0, depth - 1);
  const padding = " ".repeat(depth * 2);
  const oldPadding = line.slice(0, line.length - content.length);
  if (oldPadding === padding) return false;
  replace(editor, from, from + oldPadding.length, padding);
  const delta = padding.length - oldPadding.length;
  const shift = (offset) => offset <= from + oldPadding.length ? from + padding.length : offset + delta;
  editor.setSelectionRange(shift(start), shift(end));
  return true;
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

function expressionAt(source, offset, limit = source.length) {
  let start = offset;
  while (start < limit) {
    if (/\s/.test(source[start])) { start += 1; continue; }
    if (source[start] === ";") {
      const newline = source.indexOf("\n", start);
      start = newline === -1 ? limit : newline + 1;
      continue;
    }
    break;
  }
  if (start >= limit) return null;
  const opener = source[start];
  if (Object.hasOwn(PAIRS, opener)) {
    const close = PAIRS[opener];
    let depth = 0;
    let inString = false;
    let escaped = false;
    for (let index = start; index < limit; index += 1) {
      const character = source[index];
      if (inString) {
        if (!escaped && character === '"') inString = false;
        escaped = !escaped && character === "\\";
        continue;
      }
      if (character === '"') { inString = true; escaped = false; continue; }
      if (character === opener) depth += 1;
      if (character === close && --depth === 0) return { start, end: index + 1 };
    }
    return null;
  }
  if (opener === '"') {
    let escaped = false;
    for (let index = start + 1; index < limit; index += 1) {
      if (!escaped && source[index] === '"') return { start, end: index + 1 };
      escaped = !escaped && source[index] === "\\";
    }
    return null;
  }
  let end = start;
  while (end < limit && !/\s/.test(source[end]) && !"()[]{}".includes(source[end])) end += 1;
  return end > start ? { start, end } : null;
}

function enclosingForm(source, caret) {
  return formsIn(source)
    .filter((form) => form.start < caret && caret < form.end)
    .sort((left, right) => left.end - left.start - (right.end - right.start))[0] ?? null;
}

/** Move the closing delimiter of the enclosing form past its next sibling. */
export function slurpForward(editor) {
  const { value, selectionStart: caret, selectionEnd } = editor;
  const form = enclosingForm(value, caret);
  if (!form) return false;
  const sibling = expressionAt(value, form.end);
  if (!sibling) return false;
  replace(editor, form.end - 1, sibling.end, `${value.slice(form.end, sibling.end)}${value[form.end - 1]}`);
  editor.setSelectionRange(caret, selectionEnd);
  return true;
}

/** Move the opening delimiter after the first expression in the enclosing form. */
export function barfBackward(editor) {
  const { value, selectionStart: caret, selectionEnd } = editor;
  const form = enclosingForm(value, caret);
  if (!form) return false;
  const expressions = [];
  for (let offset = form.start + 1; offset < form.end - 1;) {
    const expression = expressionAt(value, offset, form.end - 1);
    if (!expression) break;
    expressions.push(expression);
    offset = expression.end;
  }
  const first = expressions[0];
  if (!first) return false;
  const gap = value.slice(first.end, form.end - 1).match(/^\s*/)?.[0] ?? "";
  replace(editor, form.start, first.end + gap.length, `${value.slice(form.start + 1, first.end)}${gap}${value[form.start]}`);
  editor.setSelectionRange(caret, selectionEnd);
  return true;
}

/** Move the closing delimiter before the final expression in the enclosing form. */
export function barfForward(editor) {
  const { value, selectionStart: caret, selectionEnd } = editor;
  const form = enclosingForm(value, caret);
  if (!form) return false;
  const expressions = [];
  for (let offset = form.start + 1; offset < form.end - 1;) {
    const expression = expressionAt(value, offset, form.end - 1);
    if (!expression) break;
    expressions.push(expression);
    offset = expression.end;
  }
  const last = expressions.at(-1);
  if (!last) return false;
  const gap = value.slice(form.start + 1, last.start).match(/\s*$/)?.[0] ?? "";
  const closeAt = last.start - gap.length;
  replace(editor, closeAt, form.end, `${value[form.end - 1]}${gap}${value.slice(last.start, form.end - 1)}`);
  editor.setSelectionRange(caret, selectionEnd);
  return true;
}

/** Delete from the caret to the enclosing form's closing delimiter. */
export function killToFormEnd(editor) {
  const { value, selectionStart: caret } = editor;
  const form = enclosingForm(value, caret);
  const end = form?.end - 1;
  if (end == null || caret >= end) return false;
  replace(editor, caret, end, "");
  return true;
}

/** Return the innermost balanced form at a caret, or the atom beneath it. */
export function localFormAt(source, caret) {
  const allForms = formsIn(source);
  // In whitespace, C-x C-e semantics select the completed expression on the
  // left rather than the containing outer collection.
  if (/\s/.test(source[caret] ?? "")) {
    const previous = allForms
      .filter((form) => form.end <= caret)
      .sort((left, right) => right.end - left.end || (left.end - left.start) - (right.end - right.start))[0];
    if (previous) return { ...previous, source: source.slice(previous.start, previous.end) };
  }
  const forms = allForms
    .filter((form) => form.start <= caret && caret <= form.end)
    .sort((left, right) => left.end - left.start - (right.end - right.start));
  if (forms.length) {
    const form = forms[0];
    return { ...form, source: source.slice(form.start, form.end) };
  }
  // A just-opened file normally leaves its caret after the final newline.
  // In that useful case, evaluate the preceding top-level form.
  const previous = allForms
    .filter((form) => form.end <= caret)
    .sort((left, right) => right.end - left.end)[0];
  if (previous) return { ...previous, source: source.slice(previous.start, previous.end) };
  const before = source.slice(0, caret).search(/[^\s()[\]{}]/) === -1 ? caret : caret;
  const start = source.lastIndexOf("\n", before - 1) + 1;
  const line = source.slice(start, source.indexOf("\n", before) === -1 ? source.length : source.indexOf("\n", before));
  const token = /[^\s()[\]{}]+/g;
  for (const match of line.matchAll(token)) {
    const tokenStart = start + match.index;
    const tokenEnd = tokenStart + match[0].length;
    if (tokenStart <= caret && caret <= tokenEnd) return { start: tokenStart, end: tokenEnd, source: match[0] };
  }
  return null;
}
