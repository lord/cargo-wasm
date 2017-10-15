extern crate tempdir;
extern crate term;
extern crate regex;
extern crate curl;
extern crate fantoccini_stable;
extern crate tokio_core;
extern crate futures;
extern crate rustc_serialize;
extern crate tiny_http;

mod install;
use install::{print_prefix, ensure_installed};

mod testserver;

use std::sync::{Arc, Mutex};
use std::process::{Command, exit};
use std::iter::{Iterator};

use std::ffi::OsStr;

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
    let args = std::env::args().skip(2);
    let exit_value = Arc::new(Mutex::new(1 as i32));
    let exit_value2 = exit_value.clone();
    let env = ensure_installed();
    Command::new("cargo")
        .args(args)
        .arg("--no-run")
        .envs(env.iter().map(|(k,v)| (OsStr::new(k), OsStr::new(v))))
        .arg("--target=wasm32-unknown-emscripten")
        .status()
        .unwrap_or_else(|_| {
            exit(1);
        });
    let run_server = testserver::start_server();
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let addr = std::env::var("CARGO_WASM_WEBCLIENT_URL").unwrap_or("http://localhost:4444".to_string());
    let (c, fin) = WebClient::new(&addr, &core.handle());
    let c = match core.run(c) {
        Ok(v) => v,
        Err(e) => {
            print_prefix();
            eprintln!("failed to connect to WebDriver at {} successfully: {}", &addr, e);
            exit(1);
        }
    };

    {
        // we want to have a reference to c so we can use it in the and_thens below
        fn run_check(core: tokio_core::reactor::Handle, c: WebClient, exit_value: Arc<Mutex<i32>>) {
            let core_handle = core.clone();
            let f = c.execute("var res = [window.runtimeExited, window.EXITSTATUS, window.TEST_LOGS]; window.TEST_LOGS=[]; return res;", Vec::new())
                .then(move |res| {
                    let mut json_vals = match res {
                        Ok(Json::Array(a)) => a.into_iter(),
                        _ => panic!("failed to get web server status"),
                    };
                    let runtime_exited = json_vals.next().expect("no runtime exit status found");
                    let exit_status = json_vals.next().expect("no exit status found");
                    let logs = json_vals
                        .next().expect("no test log stream found")
                        .into_array().expect("test log stream is not array");
                    for log in logs {
                        if let Json::String(line) = log {
                            eprintln!("{}", line);
                        }
                    }
                    if let Json::Boolean(true) = runtime_exited {
                        drop(c);
                        let stat = exit_status.as_i64().expect("failed to get exit status") as i32;
                        print_prefix();
                        eprintln!("tests finished with status: {}", stat);
                        let mut exit_value = exit_value.lock().expect("failed to get exit value");
                        *exit_value = stat;
                    } else {
                        run_check(core_handle, c, exit_value);
                    }
                    Ok(())
                });
            core.spawn(f);
        }

        let core_handle = core.handle();
        // now let's set up the sequence of steps we want the browser to take
        // first, go to the Wikipedia page for Foobar
        let f = c.goto("http://localhost:7777/")
            .and_then(move |_| {
                print_prefix();
                eprintln!("Running tests...");
                run_check(core_handle, c, exit_value);
                Ok(())
            });

        // and set the browser off to do those things
        core.run(f).expect("Failed to run core");
    }
    // and wait for cleanup to finish
    core.run(fin).expect("failed to run core");
    *(run_server.lock().expect("mutex unlock failed")) = false;
    exit(exit_value2.lock().expect("mutex unlock failed").clone());
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
