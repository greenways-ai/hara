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
