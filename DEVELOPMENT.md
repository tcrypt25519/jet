# Development Guide

This document describes the development workflow, tooling, and CI/CD setup for the Jet project.

## Prerequisites

### Required Tools

1. **Rust Toolchains**
   - Stable Rust (for building and testing)
   - Nightly Rust (for formatting and linting)

2. **LLVM 21**
   - Required for building the project
   - See [installation instructions](#installing-llvm)

3. **Cargo Tools**
   - `cargo-nextest` - Modern test runner

### Installation

Run the following command to install all required development tools:

```bash
make install-tools
```

This will:
- Install `cargo-nextest` for running tests
- Install Rust nightly toolchain with `rustfmt` and `clippy`

Alternatively, install manually:

```bash
# Install cargo-nextest
cargo install cargo-nextest --locked

# Install nightly toolchain with components
rustup toolchain install nightly --component rustfmt,clippy
```

## Installing LLVM

The project requires LLVM 21. Use the provided script:

```bash
make install-llvm
```

Or follow the [LLVM installation guide](https://apt.llvm.org/) for your platform.

## Development Workflow

### Building

```bash
# Build the project
make build

# Or directly with cargo
cargo build
```

### Testing

We use `cargo-nextest` as the test runner for better performance and output:

```bash
# Run all tests (using nextest)
make test

# Run all tests including doctests
make test-all

# Run only doctests (nextest doesn't support these)
make doctest

# Use the built-in cargo test runner
make test-cargo
```

### Formatting

We use **nightly rustfmt** for consistent code formatting:

```bash
# Format all code
make fmt

# Check formatting without modifying files
make fmt-check
```

### Linting

We use **nightly clippy** for linting:

```bash
# Run clippy
make clippy

# Run clippy with automatic fixes
make clippy-fix
```

### Pre-commit Checks

Before committing, run the full check suite:

```bash
# Run all checks (formatting, linting, tests)
make commit-check

# Or use the CI target
make ci
```

This runs:
1. Format check (`fmt-check`)
2. Cargo check (`check`)
3. Clippy linting (`clippy`)
4. All tests including doctests (`test-all`)

## Makefile Targets

| Target | Description |
|--------|-------------|
| `make build` | Build the project |
| `make test` | Run tests with cargo-nextest |
| `make test-all` | Run all tests including doctests |
| `make doctest` | Run only doctests |
| `make check` | Run cargo check |
| `make fmt` | Format code with nightly rustfmt |
| `make fmt-check` | Check code formatting |
| `make clippy` | Run clippy linting |
| `make clippy-fix` | Run clippy with auto-fixes |
| `make ci` | Run all CI checks locally |
| `make commit-check` | Full pre-commit check |
| `make install-tools` | Install development tools |
| `make install-llvm` | Install LLVM 21 |

## Continuous Integration

The CI pipeline runs on GitHub Actions with the following jobs:

### 1. Format Check (`fmt`)
- **Toolchain**: Nightly Rust
- **Action**: Checks code formatting with `rustfmt`
- **Command**: `cargo fmt --all -- --check`

### 2. Clippy Lint (`clippy`)
- **Toolchain**: Nightly Rust
- **Action**: Runs Clippy with strict warnings
- **Command**: `cargo clippy --all-targets --all-features -- -D warnings`

### 3. Build and Test (`build-and-test`)
- **Toolchain**: Stable Rust
- **Action**: Builds the project and runs all tests
- **Test Runner**: `cargo-nextest`
- **Commands**:
  - `cargo build --verbose --all-features`
  - `cargo nextest run --all-features --no-fail-fast`
  - `cargo test --doc --all-features` (doctests)

### 4. Cargo Check (`check`)
- **Toolchain**: Stable Rust
- **Action**: Verifies the project compiles
- **Command**: `cargo check --all-targets --all-features`

### CI Features

- **Caching**: Cargo registry, git dependencies, and build artifacts are cached
- **Parallel Jobs**: All jobs run in parallel for faster feedback
- **LLVM Setup**: Automatically installs LLVM 21 on Ubuntu runners
- **Strict Mode**: Warnings are treated as errors (`-D warnings`)

## Configuration Files

### `.nextest.toml`
Configures cargo-nextest behavior:
- Default profile for local development
- CI profile with retries for flaky tests
- JUnit output for CI integration

### `rust-toolchain.toml`
Specifies the default Rust toolchain (stable) and components.

### `.github/workflows/ci.yml`
GitHub Actions workflow configuration.

## Best Practices

1. **Always run `make commit-check` before pushing**
   - Ensures your code passes all CI checks locally
   - Saves CI minutes and time

2. **Use nightly for formatting and linting**
   - The CI enforces this
   - Run `make fmt` and `make clippy` which use nightly automatically

3. **Use stable for building and testing**
   - Ensures compatibility with stable Rust
   - The default toolchain is stable

4. **Test with nextest**
   - Faster test execution
   - Better output and reporting
   - Remember to run doctests separately (`make doctest`)

5. **Keep dependencies up to date**
   - Regularly run `cargo update`
   - Check for security advisories with `cargo audit`

## Troubleshooting

### LLVM Not Found

If you get LLVM-related errors:

```bash
# Install LLVM 21
make install-llvm

# Verify LLVM is detected
bash scripts/detect-llvm.sh
```

### Nightly Toolchain Issues

If nightly commands fail:

```bash
# Update nightly toolchain
rustup update nightly

# Ensure components are installed
rustup component add --toolchain nightly rustfmt clippy
```

### cargo-nextest Not Found

```bash
# Install or update cargo-nextest
cargo install cargo-nextest --locked --force
```

## References

- [cargo-nextest Documentation](https://nexte.st/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)
- [rustfmt Configuration](https://rust-lang.github.io/rustfmt/)
- [LLVM Installation](https://apt.llvm.org/)
