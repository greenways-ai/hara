SHELL := /usr/bin/env bash

.DEFAULT_GOAL := help

MAVEN ?= mvn
CARGO ?= cargo
NPM ?= npm
MKDOCS ?= mkdocs
ARGS ?=

TRUFFLE_JAR := java/target/hara-truffle.jar
TRUFFLE_NATIVE := target/hara-truffle
RUST_MANIFEST := rust/Cargo.toml
RUST_RAW_MANIFEST := rust/raw/Cargo.toml
RUST_DEBUG := rust/target/debug/hara
RUST_RELEASE := rust/target/release/hara
WASM_RAW := rust/raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm
WEB_DIR := rust/web
CHROME_DIR := extensions/hara-chrome
DOCS_CONFIG := docs/mkdocs.yml

.PHONY: help all build-all check-all test-all clean \
        java java-offline java-headless java-build java-test java-conformance \
        rust rust-offline rust-headless rust-build rust-build-release rust-release \
        rust-test rust-raw-test rust-layout lib-test \
        fabric fabric-build fabric-test fabric-benchmark \
        native-image native-image-run native-image-offline \
        wasm wasm-build wasm-web wasm-web-build \
        web-install hta-test studio-build studio-test \
        chrome-install chrome-build chrome-test \
        docs-build www-build \
        runtime-benchmark truffle-benchmark parity-benchmark

help: ## Show the available runtime targets
	@echo 'Hara runtimes'
	@echo
	@echo '  make java [ARGS="..."]          JVM Truffle REPL'
	@echo '  make java-offline               JVM Truffle REPL without RESP'
	@echo '  make java-headless              JVM Truffle RESP server'
	@echo '  make rust [ARGS="..."]          Rust native REPL'
	@echo '  make rust-offline               Rust native REPL without RESP'
	@echo '  make rust-headless              Rust native RESP server'
	@echo '  make fabric [ARGS="..."]        Rust Fabric coordination service'
	@echo '  make native-image-run           GraalVM native-image REPL'
	@echo '  make native-image-offline       GraalVM native-image REPL without RESP'
	@echo '  make wasm                       Build the raw WASM module (no terminal REPL)'
	@echo '  make wasm-web                   Build browser WASM package'
	@echo
	@echo 'Build and verification'
	@echo
	@echo '  make build-all                  Build JVM, Rust release, and raw WASM'
	@echo '  make check-all                  Run core layout and runtime test suites'
	@echo '  make java-test                  Run the Truffle test suite'
	@echo '  make java-conformance           Run JVM L0 conformance'
	@echo '  make rust-test                  Run the native Rust test suite'
	@echo '  make rust-raw-test              Run raw WASM crate tests'
	@echo '  make rust-layout                Check Rust module layout'
	@echo '  make lib-test                   Run portable .hal library tests'
	@echo '  make fabric-test                Run focused Fabric and RESP tests'
	@echo '  make hta-test                   Run HTA browser-loader Node tests'
	@echo '  make studio-test                Build raw WASM and run Studio tests'
	@echo '  make chrome-build               Build the Chrome extension'
	@echo '  make chrome-test                Test the Chrome extension'
	@echo '  make docs-build                 Build documentation strictly'
	@echo '  make fabric-benchmark [ARGS="..."]'
	@echo '                                  Run the agent-workroom benchmark'
	@echo '  make runtime-benchmark [ARGS="..."]'
	@echo '                                  Run the cross-runtime benchmark'
	@echo '  make clean                      Remove Maven and Cargo build output'
	@echo
	@echo 'Examples'
	@echo
	@echo '  make java ARGS="eval '\''(+ 19 23)'\''"'
	@echo '  make rust ARGS="run lib/examples/hello.hal"'
	@echo '  make rust ARGS="remote 127.0.0.1:1311"'
	@echo '  make fabric ARGS="--data target/fabric --shards 4"'
	@echo '  make fabric-benchmark ARGS="--rooms 8 --tasks 1000 --clients 4"'

all: help

java-build: $(TRUFFLE_JAR) ## Build the JVM Truffle runtime

$(TRUFFLE_JAR): java/pom.xml
	$(MAVEN) -f java/pom.xml -Ptruffle -DskipTests package

java: java-build ## Run the JVM Truffle runtime
	./hara $(ARGS)

java-offline: java-build ## Run the JVM Truffle REPL without RESP
	./hara --offline $(ARGS)

java-headless: java-build ## Run the JVM Truffle RESP server
	./hara headless $(ARGS)

java-test: ## Run the JVM Truffle test suite
	$(MAVEN) -f java/pom.xml -Ptruffle test

java-conformance: ## Run JVM L0 conformance
	$(MAVEN) -f java/pom.xml -Ptruffle -Dtest=hara.truffle.HaraL0ConformanceTest test

rust-build: ## Build the Rust native runtime
	$(CARGO) build --manifest-path $(RUST_MANIFEST) --bin hara

rust-build-release: ## Build the optimized Rust native runtime
	$(CARGO) build --release --manifest-path $(RUST_MANIFEST) --bin hara

rust: rust-build ## Run the Rust native runtime
	$(RUST_DEBUG) $(ARGS)

rust-offline: rust-build ## Run the Rust native REPL without RESP
	$(RUST_DEBUG) --offline $(ARGS)

rust-headless: rust-build ## Run the Rust native RESP server
	$(RUST_DEBUG) headless $(ARGS)

rust-release: rust-build-release ## Run optimized Rust native
	$(RUST_RELEASE) $(ARGS)

rust-test: ## Run the Rust native test suite
	$(CARGO) test --manifest-path $(RUST_MANIFEST)

rust-raw-test: ## Run raw WASM crate tests
	$(CARGO) test --manifest-path $(RUST_RAW_MANIFEST)

rust-layout: ## Check the Rust module layout
	bash rust/scripts/check-layout.sh

lib-test: ## Run portable .hal library tests
	scripts/run-lib-tests

fabric-build: rust-build ## Build the Rust Fabric service

fabric: fabric-build ## Run the Rust Fabric coordination service
	$(RUST_DEBUG) $(ARGS) fabric

fabric-test: ## Run focused Fabric service and RESP protocol tests
	$(CARGO) test --manifest-path $(RUST_MANIFEST) --lib service::tests
	$(CARGO) test --manifest-path $(RUST_MANIFEST) --lib resp::tests::fabric_protocol

fabric-benchmark: ## Run the Fabric agent-workroom benchmark
	scripts/run-fabric-benchmark $(ARGS)

native-image: $(TRUFFLE_NATIVE) ## Build the GraalVM native-image runtime

$(TRUFFLE_NATIVE): java/pom.xml scripts/build-truffle-native
	scripts/build-truffle-native

native-image-run: native-image ## Run the GraalVM native-image runtime
	$(TRUFFLE_NATIVE) $(ARGS)

native-image-offline: native-image ## Run native-image without RESP
	$(TRUFFLE_NATIVE) --offline $(ARGS)

wasm: wasm-build ## Build the raw WASM runtime

wasm-build: ## Build the raw WASM module
	scripts/build-hara-wasm-raw

wasm-web: wasm-web-build ## Build the browser WASM runtime

wasm-web-build: ## Build the browser WASM package
	scripts/build-hara-wasm-web

web-install: ## Install browser-loader and Studio dependencies
	$(NPM) --prefix $(WEB_DIR) ci

hta-test: ## Run HTA browser-loader Node tests
	$(NPM) --prefix $(WEB_DIR) run test:hta

studio-build: ## Build the shared Studio runtime artifacts
	scripts/build-studio-runtime

studio-test: wasm-build ## Run Studio Node tests against current raw WASM
	$(NPM) --prefix $(WEB_DIR) run test:studio

chrome-install: ## Install Chrome extension dependencies
	$(NPM) --prefix $(CHROME_DIR) ci

chrome-build: ## Build the Chrome extension
	$(NPM) --prefix $(CHROME_DIR) run build

chrome-test: ## Test the Chrome extension
	$(NPM) --prefix $(CHROME_DIR) test

docs-build: ## Build documentation with strict link and nav checks
	$(MKDOCS) build --strict -f $(DOCS_CONFIG)

www-build: ## Build the complete website and runtime payload
	scripts/build-www

runtime-benchmark: ## Run the cross-runtime benchmark suite
	scripts/run-runtime-benchmarks $(ARGS)

truffle-benchmark: ## Run the JVM Truffle benchmark
	scripts/run-truffle-benchmark $(ARGS)

parity-benchmark: ## Benchmark Rust WASM and Truffle parity
	scripts/run-wasm-truffle-parity-benchmark $(ARGS)

build-all: java-build rust-build-release wasm-build ## Build all portable runtime artifacts

test-all: java-test rust-test ## Run Java and Rust test suites

check-all: rust-layout java-test rust-test rust-raw-test lib-test hta-test studio-test ## Run core repository checks

clean: ## Remove generated build output
	$(MAVEN) -f java/pom.xml clean
	$(CARGO) clean --manifest-path $(RUST_MANIFEST)
	$(CARGO) clean --manifest-path $(RUST_RAW_MANIFEST)
