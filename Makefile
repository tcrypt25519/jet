LLVM_VERSION := 21
LLVM_PREFIX := $(shell bash scripts/detect-llvm.sh)

export LLVM_SYS_$(LLVM_VERSION)1_PREFIX=$(LLVM_PREFIX)
export RUST_BACKTRACE=1

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
test: ## Run the tests
	cargo test

.PHONY: check
check: ## Run Cargo check
	cargo check

.PHONY: clippy
clippy: ## Run clippy
	cargo clippy --all-targets --all-features -- -D warnings

.PHONY: commit-check
commit-check: check build test clippy ## Full check to run before commits

.DEFAULT_GOAL := help
.PHONY: help
help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
