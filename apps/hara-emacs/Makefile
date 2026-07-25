.PHONY: all test compile clean upgrade

all: compile test

test:
	emacs --batch -L . -L test \
	  -l test/hara-mode-test.el \
	  -f ert-run-tests-batch-and-exit

compile:
	emacs --batch -L . -f batch-byte-compile hara-mode.el

clean:
	rm -f hara-mode.elc

upgrade:
	emacs --batch \
	  --eval '(require (quote package))' \
	  --eval '(load-library "package-vc")' \
	  --eval '(package-initialize)' \
	  --eval '(when-let ((desc (cadr (assq (quote hara-mode) package-alist)))) (package-vc-upgrade desc))'
