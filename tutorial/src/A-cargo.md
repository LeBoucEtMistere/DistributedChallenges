# Appendix A: Cargo Cheatsheet
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
