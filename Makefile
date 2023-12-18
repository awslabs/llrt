TARGET_linux_x86_64 = x86_64-unknown-linux-musl
TARGET_linux_arm64 = aarch64-unknown-linux-musl
TARGET_darwin_x86_64 = x86_64-apple-darwin
TARGET_darwin_arm64 = aarch64-apple-darwin
TOOLCHAIN = +nightly
BUILD_ARG = $(TOOLCHAIN) build -r
BUILD_DIR = ./target/release
BUNDLE_DIR = bundle
ZSTD_LIB_ARGS = UNAME=Linux ZSTD_LIB_COMPRESSION=0 ZSTD_LIB_DICTBUILDER=0
CC_ARM = zig cc -target aarch64-linux-musl -flto
CC_X86 = zig cc -target x86_64-linux-musl -flto
AR = zig ar

TS_SOURCES = $(wildcard src/js/*.ts) $(wildcard src/js/@llrt/*.ts) $(wildcard tests/*.ts)
STD_JS_FILE = $(BUNDLE_DIR)/@llrt/std.js

RELEASE_ARCH_NAME_x86 = x86_64
RELEASE_ARCH_NAME_arm64 = arm64

RELEASE_TARGETS = arm64 x86
RELEASE_ZIPS = $(addprefix llrt-lambda-,$(RELEASE_TARGETS))

ifeq ($(OS),Windows_NT)
    DETECTED_OS := Windows
	ARCH := x86
else
    DETECTED_OS := $(shell uname | tr A-Z a-z)
	ARCH := $(shell uname -m)
endif

CURRENT_TARGET ?= $(TARGET_$(DETECTED_OS)_$(ARCH))

lambda: | libs $(RELEASE_ZIPS)

release: clean-js | lambda llrt-linux-x86.zip llrt-linux-arm64.zip llrt-macos-x86.zip llrt-macos-arm64.zip

release-linux: clean-js | lambda llrt-linux-x86.zip llrt-linux-arm64.zip
release-osx: clean-js | llrt-macos-x86.zip llrt-macos-arm64.zip

llrt-macos-x86.zip: js
	cargo $(BUILD_ARG) --target $(TARGET_darwin_x86_64)
	zip -j $@ target/$(TARGET_darwin_x86_64)/release/llrt

llrt-macos-arm64.zip: js
	cargo $(BUILD_ARG) --target $(TARGET_darwin_arm64)
	zip -j $@ target/$(TARGET_darwin_arm64)/release/llrt

llrt-linux-x86.zip: js
	cargo $(BUILD_ARG) --target $(TARGET_linux_x86_64)
	zip -j $@ target/$(TARGET_linux_x86_64)/release/llrt

llrt-linux-arm64.zip: js
	cargo $(BUILD_ARG) --target $(TARGET_linux_arm64)
	zip -j $@ target/$(TARGET_linux_arm64)/release/llrt

define release_template
release-${1}: js
	cargo $$(BUILD_ARG) --target $$(TARGET_linux_$$(RELEASE_ARCH_NAME_${1})) --features lambda -vv
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
	rustup toolchain install nightly --target $(TARGET_linux_x86_64)
	rustup toolchain install nightly --target $(TARGET_linux_arm64)
	rustup component add rust-src --toolchain nightly --target $(TARGET_linux_arm64)
	rustup component add rust-src --toolchain nightly --target $(TARGET_linux_x86_64)

toolchain:
	rustup target add $(CURRENT_TARGET)
	rustup toolchain install nightly --target $(CURRENT_TARGET)
	rustup component add rust-src --toolchain nightly --target $(CURRENT_TARGET)

clean-js:
	rm -rf ./bundle

clean: clean-js
	rm -rf ./target

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
	cargo run -vv

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

flame:
#cargo build --profile=flame
	time target/flame/llrt
	rm -rf flamegraph.svg out.stacks
	sudo dtrace -c 'target/flame/llrt' -o out.stacks -n 'profile-997 /execname == "llrt"/ { @[ustack(1000)] = count(); }'
	cat out.stacks | inferno-collapse-dtrace | inferno-flamegraph > flamegraph.svg
#cargo flamegraph

run-cli: export RUST_LOG = llrt=trace
run-cli: js
	cargo run

test: export JS_MINIFY = 0
test: js 
	cargo run -- test -d bundle

test-ci: export JS_MINIFY = 0
test-ci: toolchain js
	cargo run -r --target $(CURRENT_TARGET) -- test -d bundle

libs: lib/zstd.h

lib/zstd.h: | lib/arm64/libzstd.a lib/x86/libzstd.a
	cp zstd/lib/zstd.h $@

lib/arm64/libzstd.a: 
	mkdir -p $(dir $@)
	rm -f zstd/lib/-.o
	cd zstd/lib && make clean && make -j lib-nomt CC="$(CC_ARM)" AR="$(AR)" $(ZSTD_LIB_ARGS)
	cp zstd/lib/libzstd.a $@

lib/x86/libzstd.a:
	mkdir -p $(dir $@)
	rm -f zstd/lib/-.o
	cd zstd/lib && make clean && make -j lib-nomt CC="$(CC_X86)" AR="$(AR)" $(ZSTD_LIB_ARGS)
	cp zstd/lib/libzstd.a $@ 

bench:
	cargo build -r
	hyperfine -N --warm64up=100 "node fixtures/hello.js" "deno run fixtures/hello.js" "bun fixtures/hello.js" "$(BUILD_DIR)/llrt fixtures/hello.js" "qjs fixtures/hello.js"

deploy:
	cd example/infrastructure && yarn deploy --require-approval never

.PHONY: toolchain clean-js release-linux release-osx lambda stdlib test test-ci run js run-release build release clean flame deploy