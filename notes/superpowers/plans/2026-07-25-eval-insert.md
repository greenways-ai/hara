# hara-mode eval-and-insert Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an Emacs command that evaluates the preceding Hara form and inserts the result at point.

**Architecture:** Extend `hara-mode.el` with a new async eval command, bind it in `hara-mode-map`, add an ERT test, and update the README.

**Tech Stack:** Emacs Lisp, ERT, Projectile (optional integration already present).

## Global Constraints

- Keep changes minimal and consistent with existing `hara-mode.el` patterns.
- Use async `hara--request`, not synchronous requests.
- Insert exactly the string returned by the runtime.
- Error handling uses existing `hara--show-error`.

---

### Task 1: Implement `hara-eval-last-sexp-and-insert`

**Files:**
- Modify: `apps/hara-emacs/hara-mode.el`

**Interfaces:**
- Consumes: `hara--last-sexp-bounds`, `hara--request`, `hara--show-error`
- Produces: `hara-eval-last-sexp-and-insert` (interactive command)

- [ ] **Step 1: Add the function after `hara-eval-last-sexp`**

```elisp
;;;###autoload
(defun hara-eval-last-sexp-and-insert ()
  "Evaluate the form preceding point and insert its result at point."
  (interactive)
  (let ((bounds (hara--last-sexp-bounds))
        (insertion-point (point))
        (buffer (current-buffer)))
    (hara--request (hara--connection) "EVAL"
                   (list (buffer-substring-no-properties (car bounds) (cdr bounds)))
                   (lambda (value)
                     (when (buffer-live-p buffer)
                       (with-current-buffer buffer
                         (save-excursion
                           (goto-char insertion-point)
                           (insert value)))))
                   (lambda (error)
                     (hara--show-error error)))))
```

- [ ] **Step 2: Bind `C-c C-i` in `hara-mode-map`**

Add to the `define-key` block:

```elisp
(define-key map (kbd "C-c C-i") #'hara-eval-last-sexp-and-insert)
```

---

### Task 2: Add ERT test

**Files:**
- Modify: `apps/hara-emacs/test/hara-mode-test.el`

**Interfaces:**
- Consumes: `hara-eval-last-sexp-and-insert`, `hara--request`
- Produces: `hara-eval-last-sexp-and-inserts-result` test

- [ ] **Step 1: Add the test**

```elisp
(ert-deftest hara-eval-last-sexp-and-inserts-result ()
  "Eval-and-insert should insert the runtime result at point."
  (with-temp-buffer
    (hara-mode)
    (insert "(+ 1 2) ")
    (goto-char (point-min))
    (search-forward "2)")
    (let* ((process (make-pipe-process :name "hara-insert-test"
                                       :command '("cat") :noquery t))
           (hara--connection
            (hara--make-connection :process process
                                   :pending (make-hash-table :test #'equal))))
      (unwind-protect
          (cl-letf (((symbol-function 'hara--request)
                     (lambda (_connection _command _arguments success _error)
                       (funcall success "3"))))
            (hara-eval-last-sexp-and-insert)
            (should (string-match-p "(+ 1 2) 3" (buffer-string))))
        (delete-process process)))))
```

---

### Task 3: Update README

**Files:**
- Modify: `apps/hara-emacs/README.md`

- [ ] **Step 1: Add the new command to the Common commands list**

Insert after the `C-c C-e` line:

```markdown
- `C-c C-i`: evaluate the preceding form and insert the result at point
```

---

### Task 4: Verify and commit

**Files:**
- `apps/hara-emacs/hara-mode.el`
- `apps/hara-emacs/test/hara-mode-test.el`
- `apps/hara-emacs/README.md`

- [ ] **Step 1: Run ERT tests**

```sh
cd apps/hara-emacs
emacs --batch -L . -L test -l test/hara-mode-test.el -f ert-run-tests-batch-and-exit
```

Expected: all tests pass.

- [ ] **Step 2: Byte-compile**

```sh
emacs --batch -L . -f batch-byte-compile hara-mode.el
```

Expected: no warnings.

- [ ] **Step 3: Commit**

```bash
git add apps/hara-emacs/hara-mode.el apps/hara-emacs/test/hara-mode-test.el apps/hara-emacs/README.md
git commit -m "Add hara-eval-last-sexp-and-insert bound to C-c C-i"
```
