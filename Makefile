export LLVM_SYS_180_PREFIX=/usr/local/opt/llvm
export RUST_BACKTRACE=1

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

