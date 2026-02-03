# Jet: An LLVM Environment for EVM Contracts

Jet is an experimental project that aims to create a high-performance execution environment for Ethereum Virtual
Machine (EVM) contracts using LLVM. By leveraging LLVM's powerful optimization capabilities, Jet seeks to improve the
efficiency and speed of EVM contract execution.

## Project Goals

- Provide a fast and efficient EVM execution environment
- Utilize LLVM for advanced optimization of EVM bytecode
- Offer a flexible platform for EVM-based blockchain development and research
- Maintain compatibility with existing EVM contracts while exploring performance improvements

## Getting Started

### Prerequisites

- Rust (latest stable version)
- LLVM 21

### Building

```shell
# Install dependencies (handles LLVM and platform-specific packages)
make install-llvm

# Build
make build
```

### Platform-Specific Notes

**Termux:** Requires `gcc-default` and `ndk-multilib-native-static` packages (installed by `make install-llvm`)

**Ubuntu 24.04 (noble):** The LLVM 21 packages are not published for the noble apt.llvm.org repo yet, so
`make install-llvm` falls back to the official LLVM 21 binary release for Linux.

### Running jetdbg

The `jetdbg` command allows you to debug and execute EVM contracts using Jet. To run it:

```shell
cargo run --bin jetdbg
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
