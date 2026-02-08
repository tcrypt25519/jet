LLVM_VERSION := 21
LLVM_PREFIX := $(shell bash scripts/detect-llvm.sh)

export LLVM_SYS_$(LLVM_VERSION)1_PREFIX=$(LLVM_PREFIX)
export RUST_BACKTRACE=1

# On Termux (Android) the project lives on a FUSE mount (/storage/emulated/0)
# that does not support the executable bit, so build artifacts must be placed
# on the data partition instead.
ifneq ($(wildcard /data/data/com.termux),)
export CARGO_TARGET_DIR := /data/data/com.termux/files/home/.cargo/jet-target
endif

# On Termux (Android) the project lives on a FUSE mount (/storage/emulated/0)
# that does not support the executable bit, so build artifacts must be placed
# on the data partition instead.
ifneq ($(wildcard /data/data/com.termux),)
export CARGO_TARGET_DIR := /data/data/com.termux/files/home/.cargo/jet-target
endif

.PHONY: install-llvm
install-llvm: ## Install LLVM 21 for your platform
	@bash scripts/install-llvm.sh

.PHONY: build
build: ## Build the project
	cargo build

.PHONY: run
run: ## Run the project
	cargo run

.PHONY: test
test: ## Run the tests with cargo-nextest
	cargo nextest run --all-features

.PHONY: test-cargo
test-cargo: ## Run the tests with built-in cargo test
	cargo test --all-features

.PHONY: doctest
doctest: ## Run doc tests (nextest doesn't support these yet)
	cargo test --doc --all-features

.PHONY: test-all
test-all: test doctest ## Run all tests including doctests

.PHONY: check
check: ## Run Cargo check
	cargo check --all-targets --all-features

.PHONY: fmt
fmt: ## Format code with rustfmt
	cargo fmt --all

.PHONY: fmt-check
fmt-check: ## Check code formatting with rustfmt
	cargo fmt --all -- --check

.PHONY: clippy
clippy: ## Run clippy
	cargo clippy --all-targets --all-features -- -D warnings

.PHONY: clippy-fix
clippy-fix: ## Run clippy with automatic fixes
	cargo clippy --all-targets --all-features --fix

.PHONY: install-tools
install-tools: ## Install cargo-nextest
	@echo "Installing cargo-nextest..."
	cargo install cargo-nextest --locked
	@echo "Done!"

.PHONY: ci
ci: fmt-check check clippy test-all ## Run all CI checks locally

.PHONY: commit-check
commit-check: fmt-check check clippy test-all ## Full check to run before commits

.DEFAULT_GOAL := help
.PHONY: help
help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
