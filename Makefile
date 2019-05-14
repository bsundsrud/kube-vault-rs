BUILD_DIR := target/release
EXE := kube-vault
TARGET := $(shell uname -p)-$(shell uname -s)
PREFIX := /usr/local
EXE_PATH := $(BUILD_DIR)/$(EXE)
VERSION := $(shell cat kube-vault/Cargo.toml | grep version | sed -e 's/.*version\s*=\s*"\(.*\)"/\1/')
.PHONY: build release release-static install dist test lint
ifeq ($(shell uname -s), Linux)
  CARGO_ARGS = --target x86_64-unknown-linux-musl
  BUILD_DIR = target/x86_64-unknown-linux-musl/release
else
  CARGO_ARGS = 
endif

release:
	cargo build --release $(CARGO_ARGS)
install:
	install $(EXE_PATH) $(PREFIX)/bin

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy

dist:
	tar czf "kube-vault-$(TARGET)-$(VERSION).tgz" -C $(BUILD_DIR) $(EXE)
