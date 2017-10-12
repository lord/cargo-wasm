<p align="center">
  <img src="https://raw.githubusercontent.com/lord/img/master/logo-cargowasm.png" alt="cargo wasm: Easy Rust to Webassembly" width="226">
  <br>
  <!-- <a href="https://travis-ci.org/lord/backtalk"><img src="https://travis-ci.org/lord/backtalk.svg?branch=master" alt="Build Status"></a> -->
  <a href="https://crates.io/crates/cargo-wasm"><img src="https://img.shields.io/crates/v/cargo-wasm.svg" alt="Crate Info"></a>
  <a href="https://docs.rs/cargo-wasm"><img src="https://img.shields.io/badge/docs.rs-visit-brightgreen.svg" alt="Documentation"></a>
</p>

```sh
# Install
cargo install cargo-wasm

# Usage
cargo wasm <insert cargo subcommand here>
```

`cargo wasm` is a quick hack to ensure Emscripten is set up properly when running cargo commands. Running `cargo wasm build` automatically installs Emscripten if `emcc` is not already present, sets up the correct environment variables for the compiler, and then runs `cargo build --target=wasm32-unknown-emscripten`.

