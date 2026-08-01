NGINX_SRC ?=
RUNTIME_DIR := $(CURDIR)/../target/release
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)
HARA_VERSION := $(shell grep -m1 '^version = ' $(CURDIR)/../Cargo.toml | cut -d'"' -f2)

ifeq ($(UNAME_M),arm64)
MACOS_TARGET := aarch64-apple-darwin
else ifeq ($(UNAME_M),x86_64)
MACOS_TARGET := x86_64-apple-darwin
else
MACOS_TARGET := $(UNAME_M)-apple-darwin
endif

PACKAGE_ROOT := $(abspath $(CURDIR)/../../target/dist)
PACKAGE_NAME := hara-rust-v$(HARA_VERSION)-$(MACOS_TARGET)
PACKAGE_DIR := $(PACKAGE_ROOT)/$(PACKAGE_NAME)
PACKAGE_ARCHIVE := $(PACKAGE_ROOT)/$(PACKAGE_NAME)-with-hoplite-nginx.tar.gz

ifeq ($(UNAME_S),Darwin)
OPENSSL_PREFIX ?= $(shell brew --prefix openssl@3)
PCRE2_PREFIX ?= $(shell brew --prefix pcre2)
NGINX_CC_OPT ?= -I$(OPENSSL_PREFIX)/include -I$(PCRE2_PREFIX)/include
NGINX_LD_OPT ?= -L$(OPENSSL_PREFIX)/lib -L$(PCRE2_PREFIX)/lib -Wl,-rpath,$(OPENSSL_PREFIX)/lib -Wl,-rpath,$(PCRE2_PREFIX)/lib
endif

.PHONY: runtime cli test nginx docker check-macos macos macos-package osx

runtime:
	cargo build --manifest-path runtime/Cargo.toml --release

cli:
	cargo build --manifest-path ../Cargo.toml --release --features hoplite \
		--bin hara --bin hoplite

test:
	cargo test --manifest-path runtime/Cargo.toml

nginx: runtime
	@test -n "$(NGINX_SRC)" || (echo "Set NGINX_SRC to an unpacked Nginx source tree" && exit 1)
	cd $(NGINX_SRC) && \
		HOPLITE_RUNTIME_DIR=$(RUNTIME_DIR) \
		./configure --prefix=$(CURDIR)/../../target/hoplite/nginx \
			--with-http_ssl_module --with-threads \
			$(if $(NGINX_CC_OPT),--with-cc-opt="$(NGINX_CC_OPT)") \
			$(if $(NGINX_LD_OPT),--with-ld-opt="$(NGINX_LD_OPT)") \
			--add-module=$(CURDIR)/nginx && \
		$(MAKE) && $(MAKE) install

docker:
	docker build -f docker/Dockerfile -t hoplite ..

check-macos:
	@test "$(UNAME_S)" = "Darwin" || \
		(echo "The macos target must run on macOS" && exit 1)

macos-package: check-macos nginx cli
	mkdir -p $(PACKAGE_DIR)
	install -m 755 ../target/release/hara $(PACKAGE_DIR)/hara
	install -m 755 ../target/release/hoplite $(PACKAGE_DIR)/hoplite
	install -m 755 ../../target/hoplite/nginx/sbin/nginx $(PACKAGE_DIR)/nginx
	tar -czf $(PACKAGE_ARCHIVE) -C $(PACKAGE_DIR) hara hoplite nginx
	shasum -a 256 $(PACKAGE_ARCHIVE)
	@echo
	@echo "macOS package ready:"
	@echo "  $(PACKAGE_ARCHIVE)"
	@echo
	@echo "Runtime dependencies:"
	@echo "  brew install openssl@3 pcre2"
	@echo
	@echo "Extract the package and keep hara, hoplite, and nginx together:"
	@echo "  tar -xzf $(PACKAGE_ARCHIVE) -C /your/install/directory"
	@echo
	@echo "Try it:"
	@echo "  $(PACKAGE_DIR)/hara eval '(+ 19 23)'"
	@echo "  cd /path/to/hoplite/project"
	@echo "  $(PACKAGE_DIR)/hoplite serve"

macos: macos-package

osx: macos
