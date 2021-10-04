<<<<<<< HEAD
# Cargo plugin for t3rn composable contracts
=======
<div align="center">
    <img src="./.images/cargo-contract.svg" alt="cargo-contract" height="170" />
>>>>>>> 8e86572b4b4ed2442de131c8e3506dee219fb0b7

[![CI Status][a1]][a2]
[![Matrix Chat][b1]][b2]
[![Discord Chat][c1]][c2]
[![Latest Release][d1]][d2]

[a1]: https://gitlab.parity.io/parity/cargo-contract/badges/master/pipeline.svg
[a2]: https://gitlab.parity.io/parity/cargo-contract/pipelines
[b1]: https://img.shields.io/badge/matrix-chat-brightgreen.svg?style=flat
[b2]: https://riot.im/app/#/room/#ink:matrix.parity.io
[c1]: https://img.shields.io/discord/722223075629727774?style=flat-square&label=discord
[c2]: https://discord.gg/ztCASQE
[d1]: https://img.shields.io/crates/v/cargo-contract.svg
[d2]: https://crates.io/crates/cargo-contract

<p align="center">

> <img src="./.images/ink-squid.svg" alt="squink, the ink! mascot" style="vertical-align: middle" align="left" height="60" />`cargo-contract` is a CLI tool which helps you develop smart contracts in Parity's <a href="https://github.com/paritytech/ink">ink!</a>.<br/>ink! is a Rust [eDSL](https://wiki.haskell.org/Embedded_domain_specific_language) which allows you to write smart contracts for blockchains built on the [Substrate](https://github.com/paritytech/substrate) framework.
</p>

<br/>

[Guided Tutorial for Beginners](https://substrate.dev/substrate-contracts-workshop/#/0/building-your-contract)&nbsp;&nbsp;•&nbsp;&nbsp; 
[ink! Documentation Portal](https://paritytech.github.io/ink-docs)

<br/>
</div>

More relevant links:
* Talk to us on [Element][b2] or [Discord][c2]
* [`ink!`](https://github.com/paritytech/ink) ‒ The main ink! repository with smart contract examples
* [Canvas UI](https://paritytech.github.io/canvas-ui/#/upload) ‒ Frontend for contract deployment and interaction
* [Substrate Contracts Node](https://github.com/paritytech/substrate-contracts-node) ‒ Simple Substrate blockchain which includes smart contract functionality

<<<<<<< HEAD
A CLI tool for helping setting up and managing WebAssembly smart contracts in !ink, Solidity (_not yet! WIP._) and WebAssembly text format. Supports t3rn composable contracts.
 
**This is a fork of [`cargo-contracts`](https://github.com/paritytech/cargo-contracts). The fork extends the smart contract languages with Solidity and WASM text format. It also adds the features of composable contract builds, deployment and execution via t3rn gateways.**
=======
>>>>>>> 8e86572b4b4ed2442de131c8e3506dee219fb0b7

## Installation

* Step 1: `rustup component add rust-src`.

* Step 2: Install `binaryen` in a version >= 99:

<<<<<<< HEAD
- **Install from source**
    - `cargo build --features extrinsics`
- **Install from remote repo**
  - `cargo install --git https://github.com/MaciejBaj/cargo-contract cargo-t3rn-contract --features extrinsics --force`

**You can now use the compiler as a command line tool: `cargo t3rn-contract`**

=======
  * [Debian/Ubuntu](https://tracker.debian.org/pkg/binaryen): `apt-get install binaryen`
  * [Homebrew](https://formulae.brew.sh/formula/binaryen): `brew install binaryen`
  * [Arch Linux](https://archlinux.org/packages/community/x86_64/binaryen/): `pacman -S binaryen`
  * Windows: [binary releases are available](https://github.com/WebAssembly/binaryen/releases)
>>>>>>> 8e86572b4b4ed2442de131c8e3506dee219fb0b7

  There's only an old version in your distributions package manager? Just use a 
  [binary release](https://github.com/WebAssembly/binaryen/releases).

* Step 3: `cargo install --force cargo-contract`

### Installation using Docker Image

If you prefer to use Docker instead we have a Docker image
[available on the Docker Hub](https://hub.docker.com/r/paritytech/contracts-ci-linux):

```bash
# Pull the latest stable image.
docker pull paritytech/contracts-ci-linux:production

# Create a new contract in your current directory.
docker run --rm -it -v $(pwd):/sources paritytech/contracts-ci-linux:production \
  cargo +nightly contract new --target-dir /sources my_contract

# Build the contract. This will create the contract file under
# `my_contract/target/ink/my_contract.contract`.
docker run --rm -it -v $(pwd):/sources paritytech/contracts-ci-linux:production \
  cargo +nightly contract build --manifest-path=/sources/my_contract/Cargo.toml
```
<<<<<<< HEAD
cargo-t3rn-contract 0.3.0
Utilities to develop Wasm smart contracts.

USAGE:
    cargo contract <SUBCOMMAND>

OPTIONS:
    -h, --help       Prints help information
    -V, --version    Prints version information

NEW COMMANDS:
    composable-build       Compiles multiple smart contracts according to schedule
    composable-deploy      Upload the multiple smart contracts chains according to schedule
    call-runtime-gateway   Execute smart contract via Runtime Gateway
    call-contracts-gateway Execute smart contract via Contracts Gateway
    call-contract          Execute smart contract via regular Contract call

SUBCOMMANDS:
    new                    Setup and create a new smart contract project
    build                  Compiles the smart contract
    generate-metadata      Generate contract metadata artifacts
    test                   Test the smart contract off-chain
    deploy                 Upload the smart contract code to the chain
    instantiate            Instantiate a deployed smart contract
    help                   Prints this message or the help of the given subcommand(s)
```
=======
>>>>>>> 8e86572b4b4ed2442de131c8e3506dee219fb0b7

If you want to reproduce other steps of CI process you can use the following
[guide](https://github.com/paritytech/scripts#reproduce-ci-locally).

## Usage

You can always use `cargo contract help` to print information on available
commands and their usage.

For each command there is also a `--help` flag with info on additional parameters,
e.g. `cargo contract new --help`.

##### `cargo contract new my_contract`

Creates an initial smart contract with some scaffolding code into a new
folder `my_contract` .

<<<<<<< HEAD
`cargo install --git https://github.com/MaciejBaj/cargo-contract cargo-t3rn-contract --features extrinsics --force`
=======
The contract contains the source code for the [`Flipper`](https://github.com/paritytech/ink/blob/master/examples/flipper/lib.rs) 
contract, which is about the simplest "smart" contract you can build ‒ a `bool` which gets flipped
from `true` to `false` through the `flip()` function.
>>>>>>> 8e86572b4b4ed2442de131c8e3506dee219fb0b7

##### `cargo +nightly contract build`

Compiles the contract into optimized WebAssembly bytecode, generates metadata for it,
and bundles both together in a `<name>.contract` file, which you can use for
deploying the contract on-chain.

`cargo contract build` must be run using the `nightly` toolchain. If you have
[`rustup`](https://github.com/rust-lang/rustup) installed, the simplest way to
do so is `cargo +nightly contract build`.

<<<<<<< HEAD
The entire code within this repository is licensed under the [GPLv3](LICENSE). Please [contact Parity](https://www.parity.io/contact/) if you have questions about the licensing of this product.
=======
To avoid having to always add `+nightly` you can also set `nightly` as the default
toolchain of a directory by executing `rustup override set nightly` in it.

##### `cargo contract check`

Checks that the code builds as WebAssembly. This command does not output any `<name>.contract`
artifact to the `target/` directory.

##### `cargo contract test`

Runs test suites defined for a smart contract off-chain.

## License
>>>>>>> 8e86572b4b4ed2442de131c8e3506dee219fb0b7

The entire code within this repository is licensed under the [GPLv3](LICENSE).

Please [contact us](https://www.parity.io/contact/) if you have questions about
the licensing of our products.
