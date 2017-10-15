<p align="center">
  <img src="https://raw.githubusercontent.com/lord/img/master/logo-cargowasm.png" alt="cargo wasm: Easy Rust to Webassembly" width="226">
  <br>
  <!-- <a href="https://travis-ci.org/lord/backtalk"><img src="https://travis-ci.org/lord/backtalk.svg?branch=master" alt="Build Status"></a> -->
  <a href="https://crates.io/crates/cargo-wasm"><img src="https://img.shields.io/crates/v/cargo-wasm.svg" alt="Crate Info"></a>
</p>

```sh
# Install
cargo install cargo-wasm

# Usage
cargo wasm <insert cargo subcommand here>
```

    language: python
    addons:
      firefox: "45.4.0esr"
    python:
      - "2.7"
    before_install:
      - wget https://github.com/mozilla/geckodriver/releases/download/v0.11.1/geckodriver-v0.11.1-linux64.tar.gz
      - mkdir geckodriver
      - tar -xzf geckodriver-v0.11.1-linux64.tar.gz -C geckodriver
      - export PATH=$PATH:$PWD/geckodriver

    http://www.columbia.edu/~njn2118/journal/2016/10/28.html

`cargo wasm` is a quick hack to ensure Emscripten is set up properly when running cargo commands. Running `cargo wasm build` automatically installs Emscripten if `emcc` is not already present, sets up the correct environment variables for the compiler, uses `rustup` to install the standard library for Emscripten, and then runs `cargo build --target=wasm32-unknown-emscripten`.

Automatic installation should work on macOS, Linux, and (soon) Windows. If you _ever_ encounter an error with `emcc`, please file an issue here — running `cargo wasm build` on a newly created `cargo new --bin foobar` project should _always_ work, or if not, provide helpful instructions on how to get it to work.

## Known Bugs to Fix

- [ ] doesn't work on Windows
- [ ] https://github.com/kripken/emscripten/issues/5418
  - we need to automatically
- [ ] Fails if Python 3 is default `python`
  - should run `python --version` to check version. If it's 3, check for `python2.7`, if that also fails, prompt user to install
