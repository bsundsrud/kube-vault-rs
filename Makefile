BUILD_DIR := target/x86_64-unknown-linux-musl/release
EXE := kube-vault
PREFIX := /usr/local
EXE_PATH := $(BUILD_DIR)/$(EXE)
VERSION := $(shell cat kube-vault/Cargo.toml | grep version | sed -e 's/.*version\s*=\s*"\(.*\)"/\1/')
.PHONY: build release install dist test lint

release:
	docker run --rm -it \
	-v "$(shell pwd)":/home/rust/src \
	ekidd/rust-musl-builder cargo build --release

install:
	install $(EXE_PATH) $(PREFIX)/bin

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy

dist: release
	tar czf kube-vault-linux-$(VERSION).tar.gz -C $(BUILD_DIR) $(EXE)
