EXE := kube-vault
TARGET := $(shell uname -p)-$(shell uname -s)
PREFIX := /usr/local
VERSION := $(shell cat kube-vault/Cargo.toml | grep version | sed -e 's/.*version\s*=\s*"\(.*\)"/\1/')

BUILD_DIR := target/release
CARGO_ARGS :=
ifeq ($(shell uname -s), Linux)
  CARGO_ARGS := $(CARGO_ARGS) --target x86_64-unknown-linux-musl
  BUILD_DIR := target/x86_64-unknown-linux-musl/release
endif
EXE_PATH := $(BUILD_DIR)/$(EXE)

.PHONY: build release install dist test lint clean dist-clean

release:
	cargo build --release $(CARGO_ARGS)

install:
	install $(EXE_PATH) $(PREFIX)/bin

build:
	cargo build

test: build
	cargo test --verbose --all

lint:
	cargo fmt --all -- --check
	cargo clippy -- -D 'clippy::all'

dist-clean:
	rm -rf dist

dist: dist-clean
	mkdir -p dist
	tar czf "dist/kube-vault-$(TARGET)-$(VERSION).tgz" -C $(BUILD_DIR) $(EXE)

clean: dist-clean
	cargo clean
