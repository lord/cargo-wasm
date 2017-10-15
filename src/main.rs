extern crate tempdir;
extern crate term;
extern crate regex;
extern crate curl;
extern crate fantoccini_stable;
extern crate tokio_core;
extern crate futures;
extern crate rustc_serialize;

mod install;
use install::{print_prefix, ensure_installed};

use std::process::{Command, exit, Stdio};
use std::fs::File;
use std::io::prelude::*;
use std::iter::{Iterator};
use std::collections::BTreeMap;
use std::ffi::OsStr;

use tempdir::TempDir;
use regex::Regex;
use curl::easy::Easy;
use fantoccini_stable::Client as WebClient;
use futures::Future;
use rustc_serialize::json::Json;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

const HELP_STRING: &str = r#"Run cargo commands with the correct emscripten environment

Usage:
    cargo wasm [cargo subcommand] [<args>...]

If the cargo subcommand is `build`, cargo wasm will automatically add
`--target=wasm32-unknown-emscripten` to the arguments.

If the Emscripten complier is not already found in the $PATH, cargo wasm will
automatically install emsdk to ~/.emsdk, and then with it install the latest
version of emcc."#;

fn run_tests() {
    let env = ensure_installed();
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let addr = std::env::var("CARGO_WASM_WEBCLIENT_URL").unwrap_or("http://localhost:4444".to_string());
    let (c, fin) = WebClient::new(&addr, &core.handle());
    let mut c = match core.run(c) {
        Ok(v) => v,
        Err(e) => {
            print_prefix();
            eprintln!("failed to connect to WebDriver at {} successfully: {}", &addr, e);
            exit(1);
        }
    };

    {
        // we want to have a reference to c so we can use it in the and_thens below
        fn run_check(core: tokio_core::reactor::Handle, c: WebClient) {
            let core_handle = core.clone();
            let f = c.execute("return [runtimeExited, EXITSTATUS]", Vec::new())
                .then(move |res| {
                    let mut json_vals = match res {
                        Ok(Json::Array(a)) => a.into_iter(),
                        _ => panic!("failed to get web server status"),
                    };
                    let runtime_exited = json_vals.next().expect("no runtime exit status found");
                    let exit_status = json_vals.next().expect("no exit status found");
                    if let Json::Boolean(true) = runtime_exited {
                        drop(c);
                        print_prefix();
                        let stat = exit_status.as_i64().unwrap() as i32;
                        eprintln!("tests finished with status: {}", stat);
                    } else {
                        run_check(core_handle, c);
                    }
                    Ok(())
                });
            core.spawn(f);
        }

        let core_handle = core.handle();
        // now let's set up the sequence of steps we want the browser to take
        // first, go to the Wikipedia page for Foobar
        let f = c.goto("http://localhost:9292/index.html")
            .and_then(move |_| {
                run_check(core_handle, c);
                Ok(())
            });

        // and set the browser off to do those things
        core.run(f).expect("Failed to run core");
    }
    // and wait for cleanup to finish
    core.run(fin).unwrap();
}

fn main() {
    let mut args = std::env::args().skip(2).peekable();
    match args.peek().cloned() {
        None => {
            println!("cargo wasm version {}\n{}", VERSION, HELP_STRING);
            exit(0);
        },
        Some(subcommand) => {
            if subcommand == "test" {
                run_tests();
            } else {
                let env = ensure_installed();
                let mut cmd_builder = Command::new("cargo");
                cmd_builder.args(args);
                cmd_builder.envs(env.iter().map(|(k,v)| (OsStr::new(k), OsStr::new(v))));
                if subcommand == "build" {
                    cmd_builder.arg("--target=wasm32-unknown-emscripten");
                }
                let _ = cmd_builder.status();
            }
        }
    }
}
