TARGET_linux_x86_64 = x86_64-unknown-linux-gnu
TARGET_linux_arm64 = aarch64-unknown-linux-gnu
TARGET_darwin_x86_64 = x86_64-apple-darwin
TARGET_darwin_arm64 = aarch64-apple-darwin
RUST_VERSION = nightly
TOOLCHAIN = +$(RUST_VERSION)
BUILD_ARG = $(TOOLCHAIN) build -r
BUILD_DIR = ./target/release
BUNDLE_DIR = bundle
ZSTD_LIB_ARGS = -j lib-nomt CC="$(CURDIR)/cc -s -O3 -flto" UNAME=Linux ZSTD_LIB_COMPRESSION=0 ZSTD_LIB_DICTBUILDER=0

TS_SOURCES = $(wildcard src/js/*.ts) $(wildcard src/js/@llrt/*.ts) $(wildcard tests/*.ts)
STD_JS_FILE = $(BUNDLE_DIR)/@llrt/std.js

RELEASE_ARCH_NAME_x64 = x86_64
RELEASE_ARCH_NAME_arm64 = arm64

LAMBDA_PREFIX = llrt-lambda
RELEASE_TARGETS = arm64 x64
RELEASE_ZIPS = $(addprefix $(LAMBDA_PREFIX)-,$(RELEASE_TARGETS))

ifeq ($(OS),Windows_NT)
    DETECTED_OS := Windows
	ARCH := x64
else
    DETECTED_OS := $(shell uname | tr A-Z a-z)
	ARCH := $(shell uname -m)
endif

ifeq ($(DETECTED_OS),darwin)
	export AR = $(CURDIR)/zigar
	export CC_aarch64_unknown_linux_gnu = $(CURDIR)/zigcc
	export CCX_aarch64_unknown_linux_gnu = $(CURDIR)/zigcc
	export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = $(CURDIR)/zigcc
	export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS = -Ctarget-feature=+lse -Ctarget-cpu=neoverse-n1

	export CC_x86_64_unknown_linux_gnu = $(CURDIR)/zigcc
	export CXX_x86_64_unknown_linux_gnu = $(CURDIR)/zigcc
	export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = $(CURDIR)/zigcc
else ifeq ($(DETECTED_OS),linux)
	export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="x86_64-linux-gnu-gcc"

	export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS = -Ctarget-feature=+lse -Ctarget-cpu=neoverse-n1
	export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc"

	RUNFLAGS = RUSTFLAGS="-Ctarget-feature=+crt-static"
endif

CURRENT_TARGET ?= $(TARGET_$(DETECTED_OS)_$(ARCH))

export COMPILE_TARGET = $(CURRENT_TARGET)

lambda-all: clean-js | libs $(RELEASE_ZIPS)
release-all: clean-js | lambda-all llrt-linux-x64.zip llrt-linux-arm64.zip llrt-darwin-x64.zip llrt-darwin-arm64.zip

release-lambda: clean-js |  libs-$(ARCH) $(LAMBDA_PREFIX)-$(DETECTED_OS)-$(ARCH).zip
release: clean-js | llrt-$(DETECTED_OS)-$(ARCH).zip

release-linux: clean-js | lambda-all llrt-linux-x64.zip llrt-linux-arm64.zip
release-darwin: clean-js | llrt-darwin-x64.zip llrt-darwin-arm64.zip

llrt-darwin-x64.zip: js
	cargo $(BUILD_ARG) --target $(TARGET_darwin_x86_64)
	zip -j $@ target/$(TARGET_darwin_x86_64)/release/llrt

llrt-darwin-arm64.zip: js
	cargo $(BUILD_ARG) --target $(TARGET_darwin_arm64)
	zip -j $@ target/$(TARGET_darwin_arm64)/release/llrt

llrt-linux-x64.zip: js
	cargo $(BUILD_ARG) --target $(TARGET_linux_x86_64)
	zip -j $@ target/$(TARGET_linux_x86_64)/release/llrt

llrt-linux-arm64.zip: js
	cargo $(BUILD_ARG) --target $(TARGET_linux_arm64)
	zip -j $@ target/$(TARGET_linux_arm64)/release/llrt

define release_template
release-${1}: js
	COMPILE_TARGET=$$(TARGET_linux_$$(RELEASE_ARCH_NAME_${1})) cargo $$(BUILD_ARG) --target $$(TARGET_linux_$$(RELEASE_ARCH_NAME_${1})) --features lambda -vv
	./pack target/$$(TARGET_linux_$$(RELEASE_ARCH_NAME_${1}))/release/llrt target/$$(TARGET_linux_$$(RELEASE_ARCH_NAME_${1}))/release/bootstrap
	@rm -rf llrt-lambda-${1}.zip
	zip -j llrt-lambda-${1}.zip target/$$(TARGET_linux_$$(RELEASE_ARCH_NAME_${1}))/release/bootstrap

llrt-lambda-${1}: release-${1}
endef

$(foreach target,$(RELEASE_TARGETS),$(eval $(call release_template,$(target))))

build: js
	cargo $(BUILD_ARG) --target $(CURRENT_TARGET)

stdlib:
	rustup target add $(TARGET_linux_x86_64)
	rustup target add $(TARGET_linux_arm64)
	rustup toolchain install $(RUST_VERSION) --target $(TARGET_linux_x86_64)
	rustup toolchain install $(RUST_VERSION) --target $(TARGET_linux_arm64)
	rustup component add rust-src --toolchain $(RUST_VERSION) --target $(TARGET_linux_arm64)
	rustup component add rust-src --toolchain $(RUST_VERSION) --target $(TARGET_linux_x86_64)

toolchain:
	rustup target add $(CURRENT_TARGET)
	rustup toolchain install $(RUST_VERSION) --target $(CURRENT_TARGET)
	rustup component add rust-src --toolchain $(RUST_VERSION) --target $(CURRENT_TARGET)

clean-js:
	rm -rf ./bundle

clean: clean-js
	rm -rf ./target
	rm -rf ./lib

js: $(STD_JS_FILE)

bundle/%.js: $(TS_SOURCES)
	node build.mjs

patch:
	cargo clean -p rquickjs-sys
	cargo patch

fix:
	cargo fix --allow-dirty
	cargo clippy --fix --allow-dirty
	cargo fmt

linux-flame: js
	cargo build --profile=flame --target $(TARGET_linux_x86_64)

bloat: js
	cargo build --profile=flame --target $(TARGET_linux_x86_64)
	cargo bloat --profile=flame --crates

run: export AWS_LAMBDA_FUNCTION_NAME = n/a
run: export AWS_LAMBDA_FUNCTION_MEMORY_SIZE = 1
run: export AWS_LAMBDA_FUNCTION_VERSION = 1
run: export AWS_LAMBDA_RUNTIME_API = localhost:3000
run: export _EXIT_ITERATIONS = 1
run: export AWS_REGION=eu-north-1
run: export TABLE_NAME=quickjs-table
run: export BUCKET_NAME=llrt-demo-bucket2
run: export JS_MINIFY = 0
run: export RUST_LOG = llrt=trace
run: export _HANDLER = index.handler
run: js
	cargo run -r -vv

run-js: export _HANDLER = index.handler
run-js:
	touch build.rs
	cargo run

run-release: export _HANDLER = fixtures/local.handler
run-release: js
	cargo build
	time target/release/llrt
	time target/release/llrt
	time target/release/llrt

run-ssr: export AWS_LAMBDA_RUNTIME_API = localhost:3000
run-ssr: export TABLE_NAME=quickjs-table
run-ssr: export AWS_REGION = us-east-1
run-ssr: export _HANDLER = index.handler
run-ssr: js
	cargo build
	cd example/functions && yarn build && cd build && ../../../target/debug/llrt

flame: export CARGO_PROFILE_RELEASE_DEBUG = true
flame:
	cargo flamegraph

run-cli: export RUST_LOG = llrt=trace
run-cli: js
	cargo run

test: export JS_MINIFY = 0
test: js 
	cargo run -- test -d bundle

test-ci: export JS_MINIFY = 0
test-ci: clean-js | toolchain js
	$(RUNFLAGS) cargo $(TOOLCHAIN) -Z panic-abort-tests test --target $(CURRENT_TARGET)
	$(RUNFLAGS) cargo $(TOOLCHAIN) run -r --target $(CURRENT_TARGET) -- test -d bundle

libs-arm64: lib/arm64/libzstd.a lib/zstd.h
libs-x64: lib/x64/libzstd.a lib/zstd.h

libs: | libs-arm64 libs-x64

lib/zstd.h:
	cp zstd/lib/zstd.h $@

lib/arm64/libzstd.a: 
	mkdir -p $(dir $@)
	rm -f zstd/lib/-.o
	cd zstd/lib && make clean && COMPILE_TARGET=$(TARGET_linux_arm64) make $(ZSTD_LIB_ARGS)
	cp zstd/lib/libzstd.a $@

lib/x64/libzstd.a:
	mkdir -p $(dir $@)
	rm -f zstd/lib/-.o
	cd zstd/lib && make clean && COMPILE_TARGET=$(TARGET_linux_x86_64) make $(ZSTD_LIB_ARGS)
	cp zstd/lib/libzstd.a $@ 

bench:
	cargo build -r
	hyperfine -N --warm64up=100 "node fixtures/hello.js" "deno run fixtures/hello.js" "bun fixtures/hello.js" "$(BUILD_DIR)/llrt fixtures/hello.js" "qjs fixtures/hello.js"

deploy:
	cd example/infrastructure && yarn deploy --require-approval never

.PHONY: libs libs-arm64 libs-x64 toolchain clean-js release-linux release-darwin lambda stdlib test test-ci run js run-release build release clean flame deploy