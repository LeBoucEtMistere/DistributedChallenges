# DistributedChallenges
Hosts the content of a workshop on solving Fly.io distributed challenges in Rust: https://fly.io/dist-sys/

The folder `distributes_challenges_solution` contains a rust project with the full solution while `distributed_challenges_template` contains the initial template to start the workshop.

## Setup

### Install Rust toolchain
Rust is installed through `rustup`, an utility which manages your rust toolchain (like `nvm` for node for instance). Install it with: 
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

This repo builds on the latest stable version of Rust, `Rust 1.68`.
When installing rustup, it will prompt you to install the latest stable Rust version. If you already have rust installed, run `rustup upgrade` to get the lastest version.

Note that as stated by the installer, you will need to export an env variable in your shell .rc file after the install process, and restart your shell to pick it up.

You can sanity check your installation by running `cargo --version`, more on cargo later.

You will also need to install the MacOS vendored LLVM compiler (it's possible you already have it, but it tends to disappear after a major MacOS upgrade): run `xcode-select install`.

### Setting up IDE

I recommend VSCode for the easiest dev experience possible. We will configure it to leverage `rust-analyzer`, the official LSP server for Rust, `clippy` the official linter for Rust and `rustfmt` the official formatter for Rust.

Note that both `rustfmt` and `clippy` ship with the default Rust toolchain install so you don't have to do anything to get them.

To get `rust-analyzer`, we will install the VSCode integration called `rust-lang.rust-analyzer`. This extension will handle itself the process of downloading the binaries for `rust-analyzer` transparently.

I then recommand to alter your VSCode config (`Cmd+,`) and add the following keys:
```json
{
    "editor.formatOnSave": true,
    "rust-analyzer.checkOnSave.command": "clippy",
}
```

Here are some other additional extensions that will make your developper experience smoother:
- `serayuzgur.crates` Augmented functionalities to manage the crates used in your project
- `tamasfe.even-better-toml` LSP for the TOML language which is often used with Rust
- `usernamehw.errorlens` Inline errors in your code, making it easier to see them in context

### Optional: installing a nightly toolchain for nicer formatting
Some nice formatting options for your code are gated behind the nightly toolchain of Rust. In particular, the option to sort imports by type (std,3p,prj for instance). To get this, we can install the latest nightly toolchain using rustup (`rustup toolchain install nightly`). Then add the following in your vscode config file:
```json
{
    "rust-analyzer.rustfmt.extraArgs": [
        "+nightly"
    ],
}
```
This will force the usage of the formatter coming with the nightly toolchain (this does not mean your project will be compiled with the nightly toolchain though, only the formatter will use it).

Then, you can specify a `.rustfmt.toml` file at the root of your project with this content:
```toml
group_imports = "StdExternalCrate"
```

### Install Maelstrom

```bash
brew install openjdk graphviz gnuplot
sudo ln -sfn /opt/homebrew/opt/openjdk/libexec/openjdk.jdk /Library/Java/JavaVirtualMachines/openjdk.jdk
curl -L https://github.com/jepsen-io/maelstrom/releases/download/v0.2.3/maelstrom.tar.bz2 > /tmp/maelstrom.tar.bz2
tar -C ~/ -xvf /tmp/maelstrom.tar.bz2 
```

### Run Maelstrom
For the echo challenge:
```bash
~/maelstrom/maelstrom test -w echo --bin ./target/debug/echo --node-count 1 --time-limit 10
```

## Cargo 101
`cargo` is the main interface with rust you will use. This is a CLI tool that exposes all commands to check, build, format, and release your code, and that handles interacting with the compiler `rustc` for you. Think of it as `poetry` for python or `npm` for node.

Here is a quick cheatsheet of useful commands:
```bash
cargo build # build a debug binary/lib
cargo run # build and run a debug binary/lib
cargo {build|run} --release # same but for release binary/lib

cargo test # run all tests of your project
cargo doc # build a static website serving the documentation of your project

cargo check # check correctness of your code, i.e. does it compile ?
cargo fmt # run the formatter on your code
cargo clippy # run the linter on your code, note that this is a superset of cargo check

cargo add <crate_name> # add a crate to your project, i.e. a dependency
cargo remove <crate_name> # remove a crate from your project
```

