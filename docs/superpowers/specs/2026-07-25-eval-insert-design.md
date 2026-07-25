# hara-mode eval-and-insert design

Date: 2026-07-25

## Goal

Add an Emacs command that evaluates the preceding Hara form and inserts the
result directly into the current buffer at point.

## User-facing behavior

- **Command:** `hara-eval-last-sexp-and-insert`
- **Keybinding:** `C-c C-i` in `hara-mode`
- **Insertion point:** at the current cursor position (the user selected this
  option).
- **Result form:** the raw string returned by the Hara runtime's `EVAL`
  response.
- **Error handling:** evaluation errors are shown via `hara--show-error`,
  matching `hara-eval-last-sexp`.

## Implementation

1. Compute the form bounds with the existing helper
   `hara--last-sexp-bounds`.
2. Submit the form text via the existing async request helper
   `hara--request` with the `"EVAL"` command.
3. In the success callback, capture the original buffer and insert the result
   string at the saved insertion point.
4. In the error callback, call `hara--show-error`.
5. Add the keybinding to `hara-mode-map`.
6. Add an ERT test that mocks the connection and asserts the result is
   inserted at point.
7. Update `apps/hara-emacs/README.md`.

## Why async

The existing eval commands are all async via `hara--request`. A synchronous
request would block Emacs during evaluation and be inconsistent with the rest
of the mode.

## Files changed

- `apps/hara-emacs/hara-mode.el`
- `apps/hara-emacs/test/hara-mode-test.el`
- `apps/hara-emacs/README.md`
