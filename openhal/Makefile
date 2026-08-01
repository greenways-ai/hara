NGINX_SRC ?=
RUNTIME_DIR := $(CURDIR)/runtime/target/release

.PHONY: runtime test module docker

runtime:
	cargo build --manifest-path runtime/Cargo.toml --release

test:
	cargo test --manifest-path runtime/Cargo.toml

module: runtime
	@test -n "$(NGINX_SRC)" || (echo "Set NGINX_SRC to an unpacked Nginx source tree" && exit 1)
	cd $(NGINX_SRC) && \
		OPENHAL_RUNTIME_DIR=$(RUNTIME_DIR) \
		./configure --with-compat --add-dynamic-module=$(CURDIR)/nginx && \
		$(MAKE) modules

docker:
	docker build -f docker/Dockerfile -t openhal ..
