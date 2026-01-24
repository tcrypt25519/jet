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

To build and run Jet, you'll need the following:

- Rust (latest stable version)
- LLVM 18.0

### Setting Up the Build Environment

1. Install Rust:
    ```shell
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

2. Install LLVM 18.0:
    - On macOS (using Homebrew):
      ```shell
      brew install llvm@18
      ```
    - On Ubuntu:
      ```shell
      wget https://apt.llvm.org/llvm.sh
      chmod +x llvm.sh
      sudo ./llvm.sh 18
      ```

3. Clone the Jet repository:
    ```shell
    git clone https://github.com/tyler-smith/jet.git
    cd jet
    ```

### Building the Project

1. Set up the LLVM environment variable:
    ```shell
    export LLVM_SYS_180_PREFIX=/usr/local/opt/llvm
    ```
2. Build the project:
    ```shell
    make build
    ```

### Running jetdbg

The `jetdbg` command allows you to debug and execute EVM contracts using Jet. To run it:

```shell
cargo run --bin jetdbg
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

