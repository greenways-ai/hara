.PHONY: all test compile clean upgrade \
  bin-build bin-build-truffle bin-build-native bin-build-rust \
  bin-install bin-install-truffle bin-install-native bin-install-rust \
  bin-clean

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

# ---------------------------------------------------------------------------
# Runtime binary build/install targets
# ---------------------------------------------------------------------------

HARA_ROOT := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))../..
LOCAL_BIN := $(HOME)/.local/bin
LOCAL_SHARE := $(HOME)/.local/share/hara-mode

TRUFFLE_JAR_SRC := $(HARA_ROOT)/java/target/hara-truffle.jar
TRUFFLE_JAR_DST := $(LOCAL_SHARE)/hara-truffle.jar
NATIVE_SRC := $(HARA_ROOT)/target/hara-truffle
RUST_SRC := $(HARA_ROOT)/rust/target/release/hara

bin-build: bin-build-truffle
bin-install: bin-install-truffle

bin-build-truffle:
	mvn -f "$(HARA_ROOT)/java/pom.xml" -Ptruffle -DskipTests package

bin-install-truffle: bin-build-truffle
	mkdir -p "$(LOCAL_BIN)" "$(LOCAL_SHARE)"
	install -m 644 "$(TRUFFLE_JAR_SRC)" "$(TRUFFLE_JAR_DST)"
	printf '%s\n' '#!/bin/sh' \
	  'exec java -jar "$${HARA_RUNTIME_JAR:-$${HOME}/.local/share/hara-mode/hara-truffle.jar}" "$$@"' \
	  > "$(LOCAL_BIN)/hara-truffle"
	chmod 755 "$(LOCAL_BIN)/hara-truffle"

bin-build-native:
	bash "$(HARA_ROOT)/scripts/build-truffle-native"

bin-install-native: bin-build-native
	mkdir -p "$(LOCAL_BIN)"
	install -m 755 "$(NATIVE_SRC)" "$(LOCAL_BIN)/hara-native"

bin-build-rust:
	cargo build --release --manifest-path "$(HARA_ROOT)/rust/Cargo.toml"

bin-install-rust: bin-build-rust
	mkdir -p "$(LOCAL_BIN)"
	install -m 755 "$(RUST_SRC)" "$(LOCAL_BIN)/hara-rust"

bin-clean:
	rm -f "$(TRUFFLE_JAR_DST)" "$(LOCAL_BIN)/hara-truffle" \
	      "$(LOCAL_BIN)/hara-native" "$(LOCAL_BIN)/hara-rust"
