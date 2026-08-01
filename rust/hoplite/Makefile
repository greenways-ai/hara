NGINX_VERSION := 1.30.4
NGINX_SHA256 := 4261dc90e9e47c1c4041276e9aaa3d48ebe2e664f728e14fa95ae6c67d57a08b
SOURCE_ROOT := $(abspath $(CURDIR)/../../target/hoplite/sources)
NGINX_ARCHIVE := $(SOURCE_ROOT)/nginx-$(NGINX_VERSION).tar.gz
DEFAULT_NGINX_SRC := $(SOURCE_ROOT)/nginx-$(NGINX_VERSION)
NGINX_SRC ?= $(DEFAULT_NGINX_SRC)
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
HOPLITE_ARTIFACT := $(PACKAGE_ROOT)/hoplite-v$(HARA_VERSION)-$(MACOS_TARGET)

ifeq ($(UNAME_S),Darwin)
OPENSSL_PREFIX ?= $(shell brew --prefix openssl@3)
PCRE2_PREFIX ?= $(shell brew --prefix pcre2)
NGINX_CC_OPT ?= -I$(OPENSSL_PREFIX)/include -I$(PCRE2_PREFIX)/include
NGINX_LD_OPT ?= $(PCRE2_PREFIX)/lib/libpcre2-8.a $(OPENSSL_PREFIX)/lib/libssl.a $(OPENSSL_PREFIX)/lib/libcrypto.a -Wl,-dead_strip_dylibs
endif

.PHONY: runtime cli embedded-cli test nginx nginx-source docker check-macos macos macos-package osx

runtime:
	cargo build --manifest-path runtime/Cargo.toml --release

cli:
	cargo build --manifest-path ../Cargo.toml --release --features hoplite \
		--bin hara --bin hoplite

embedded-cli: nginx
	cargo build --manifest-path ../Cargo.toml --release \
		--features hoplite-embedded-nginx --bin hoplite

test:
	cargo test --manifest-path runtime/Cargo.toml

$(NGINX_ARCHIVE):
	mkdir -p $(SOURCE_ROOT)
	curl --fail --location --retry 3 \
		https://nginx.org/download/nginx-$(NGINX_VERSION).tar.gz \
		--output $@.tmp
	echo "$(NGINX_SHA256)  $@.tmp" | shasum -a 256 --check
	mv $@.tmp $@

$(DEFAULT_NGINX_SRC)/configure: $(NGINX_ARCHIVE)
	tar -xzf $(NGINX_ARCHIVE) -C $(SOURCE_ROOT)

nginx-source: $(NGINX_SRC)/configure

nginx: runtime nginx-source
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

macos-package: check-macos embedded-cli
	mkdir -p $(PACKAGE_ROOT)
	install -m 755 ../target/release/hoplite $(HOPLITE_ARTIFACT)
	shasum -a 256 $(HOPLITE_ARTIFACT)
	@echo
	@echo "Standalone macOS executable ready:"
	@echo "  $(HOPLITE_ARTIFACT)"
	@echo
	@echo "Install it:"
	@echo "  install -m 755 $(HOPLITE_ARTIFACT) /usr/local/bin/hoplite"
	@echo "  hoplite version"
	@echo
	@echo "Install a project as a per-user launchd service:"
	@echo "  hoplite serve install /path/to/hoplite/project"
	@echo "  hoplite serve status /path/to/hoplite/project"
	@echo "  hoplite serve uninstall /path/to/hoplite/project"

macos: macos-package

osx: macos
