# Development Guide

This document describes the development workflow, tooling, and CI/CD setup for the Jet project.

## Prerequisites

### Required Tools

1. **Rust Toolchain**
   - Stable Rust (for building, testing, formatting, and linting)

2. **LLVM 21**
   - Required for building the project
   - See [installation instructions](#installing-llvm)

3. **Cargo Tools**
   - `cargo-nextest` - Modern test runner

### Installation

Run the following command to install required development tools:

```bash
make install-tools
```

This will install `cargo-nextest` for running tests.

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

Format code with rustfmt:

```bash
# Format all code
make fmt

# Check formatting without modifying files
make fmt-check
```

### Linting

Run clippy for linting:

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
| `make test-cargo` | Run tests with built-in cargo test |
| `make doctest` | Run only doctests |
| `make check` | Run cargo check |
| `make fmt` | Format code with rustfmt |
| `make fmt-check` | Check code formatting |
| `make clippy` | Run clippy linting |
| `make clippy-fix` | Run clippy with auto-fixes |
| `make ci` | Run all CI checks locally |
| `make commit-check` | Full pre-commit check |
| `make install-tools` | Install cargo-nextest |
| `make install-llvm` | Install LLVM 21 |

## Continuous Integration

The CI pipeline runs on GitHub Actions as a single sequential job with the following steps:

### CI Workflow Steps

1. **Cache LLVM 21** - Checks for cached LLVM installation
2. **Restore LLVM from cache** - Restores if cache hit (conditional)
3. **Install LLVM 21** - Uses `scripts/install-llvm.sh` if not cached (conditional)
4. **Log LLVM shared libraries** - Verification step
5. **Set LLVM environment variables** - Configures LLVM_SYS_211_PREFIX
6. **Cache Rust build artifacts** - Uses Swatinem/rust-cache
7. **Install Rust stable** - With rustfmt and clippy components
8. **Apply formatting fixes** - `cargo fmt --all`
9. **Commit formatting fixes** - Automatically commits and pushes formatting changes (conditional)
10. **Run clippy** - `cargo clippy --all-targets --all-features -- -D warnings`
11. **Install cargo-nextest** - Test runner
12. **Check all targets** - `cargo check --all-targets --all-features`
13. **Build** - `cargo build --verbose --all-features`
14. **Run tests with nextest** - `cargo nextest run --all-features --no-fail-fast`
15. **Run doctests** - `cargo test --doc --all-features`

### CI Features

- **Sequential execution**: Steps run in order, failing fast on errors
- **Smart caching**: Uses `Swatinem/rust-cache` for optimized Rust artifact caching and LLVM binary caching
- **LLVM Setup**: Automatically installs and caches LLVM 21 using project scripts
- **Strict Mode**: Warnings are treated as errors (`-D warnings`)
- **Auto-formatting**: Automatically applies formatting fixes and pushes them back to the branch on push/PR

## Configuration Files

### `.nextest.toml`
Configures cargo-nextest test runner behavior with sensible defaults.

### `rust-toolchain.toml`
Specifies stable Rust as the default toolchain for all operations.

### `.github/workflows/ci.yml`
GitHub Actions workflow configuration.

## Best Practices

1. **Always run `make commit-check` before pushing**
   - Ensures your code passes all CI checks locally
   - Saves CI minutes and time

2. **Test with nextest**
   - Faster test execution
   - Better output and reporting
   - Remember to run doctests separately (`make doctest`)

3. **Keep dependencies up to date**
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
