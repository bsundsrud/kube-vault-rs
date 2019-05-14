BUILD_DIR := target/x86_64-unknown-linux-musl/release
EXE := kube-vault
TARGET := linux
PREFIX := /usr/local
EXE_PATH := $(BUILD_DIR)/$(EXE)
VERSION := $(shell cat kube-vault/Cargo.toml | grep version | sed -e 's/.*version\s*=\s*"\(.*\)"/\1/')
.PHONY: build release release-static install dist test lint

release:
	cargo build --release

release-musl:
	cargo build --release --target x86_64-unknown-linux-musl

install:
	install $(EXE_PATH) $(PREFIX)/bin

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy

dist:
	tar czf kube-vault-$(TARGET)-$(VERSION).tgz -C $(BUILD_DIR) $(EXE)
