extern crate tempdir;
extern crate term;
extern crate regex;

use std::process::{Command, exit, Stdio};
use std::fs::File;
use std::io::prelude::*;
use std::iter::{Iterator};
use std::collections::BTreeMap;
use std::ffi::OsStr;

use tempdir::TempDir;
use regex::Regex;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn print_prefix() {
    let mut t = term::stderr().unwrap();
    t.fg(term::color::GREEN).unwrap();
    write!(t, "  Setup wasm").unwrap();
}

const HELP_STRING: &str = r#"Run cargo commands with the correct emscripten environment

Usage:
    cargo wasm [cargo subcommand] [<args>...]

If the cargo subcommand is `build`, cargo wasm will automatically add
`--target=wasm32-unknown-emscripten` to the arguments.

If the Emscripten complier is not already found in the $PATH, cargo wasm will
automatically install emsdk to ~/.emsdk, and then with it install the latest
version of emcc."#;

const EMSDK_URL: &str =
    "https://s3.amazonaws.com/mozilla-games/emscripten/releases/emsdk-portable.tar.gz";

fn check_cmake_installed() {
    if let Err(e) = Command::new("cmake").args(&["--version"]).output() {
        if let std::io::ErrorKind::NotFound = e.kind() {
            if cfg!(target_os = "linux") {
                print_prefix();
                eprintln!("`cmake` not found. Try installing with `sudo apt-get install cmake` and rebuilding?");
            } else if cfg!(target_os = "macos") {
                print_prefix();
                eprintln!("`cmake` not found. Try installing with `brew install cmake` and rebuilding?");
            } else if cfg!(target_os = "windows") {
                print_prefix();
                eprintln!("`cmake` not found. Try installing from https://cmake.org/download/ and rebuilding?");
            } else {
                print_prefix();
                eprintln!("`cmake` not found. Try installing and rebuilding?");
            }
        } else {
            print_prefix();
            eprintln!("Unknown error when checking installation of `cmake`: {:?}", e);
        }
        exit(1);
    }
}

fn check_installation(cmd: &str, arg: &str, fail_msg: &str) -> bool {
    false
}

fn check_rustup_installed() {
    print_prefix();
    eprintln!("checking for wasm32 rustup target...");
    if let Err(e) = Command::new("rustup").args(&["target", "add", "wasm32-unknown-emscripten"]).output() {
        if let std::io::ErrorKind::NotFound = e.kind() {
            print_prefix();
            eprintln!("rustup installation not found. Try installing from https://rustup.rs");
        } else {
            print_prefix();
            eprintln!("Unknown error when checking rustup installation: {:?}", e);
        }
        exit(1);
    }
}

fn check_emsdk_install(target_dir: &std::path::PathBuf) -> bool {
    Command::new("./emsdk").args(&["--help"])
        .current_dir(&target_dir)
        .output()
        .is_ok()
}

fn install_emsdk(target_dir: &std::path::PathBuf) {
    let _ = std::fs::create_dir(target_dir.clone());
    let cmd = Command::new("curl")
        .args(&[EMSDK_URL])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| { panic!("failed to execute curl: {}", e) });
    Command::new("tar").args(&["--strip-components=1", "-zxvf", "-"])
        .stdin(cmd.stdout.unwrap_or_else(|| { panic!("failed to get curl output.") }))
        .current_dir(target_dir)
        .output()
        .unwrap_or_else(|e| { panic!("failed to execute tar: {}", e) });
}

fn get_env(emsdk_path: &std::path::PathBuf) -> BTreeMap<String, String> {
    print_prefix();
    let temp_dir = TempDir::new("cargo-wasmnow").unwrap_or_else(|_| { panic!("failed to create temp directory") });
    let re = Regex::new(r#"^export ([a-zA-Z0-9_\-]+)="(.*)""#).unwrap();
    Command::new(emsdk_path).args(&["construct_env"])
        .current_dir(&temp_dir)
        .output()
        .unwrap_or_else(|e| { panic!("failed to execute emsdk: {}", e) });
    let mut env_file = temp_dir.into_path();
    env_file.push("emsdk_set_env.sh");
    let mut contents = String::new();
    let _ = File::open(env_file).unwrap().read_to_string(&mut contents).unwrap();
    let env_lines: Vec<&str> = contents.split("\n").filter(|line| line.len() > 0).collect();
    let mut res = BTreeMap::new();
    for line in env_lines {
        let caps = re.captures(line).unwrap();
        res.insert(caps[1].to_string(), caps[2].to_string());
    }
    res
}

// returns env needed to use emcc
fn ensure_installed() -> BTreeMap<String, String> {
    // check if just already in path and working
    if let Ok(_) = Command::new("emcc").args(&["--version"]).output() {
        print_prefix();
        eprintln!("using emcc already in $PATH");
        return BTreeMap::new();
    }

    // install emsdk if necessary, and then see if we can just use the env vars to get an emcc
    check_cmake_installed();
    print_prefix();
    let mut target_dir = std::env::home_dir().unwrap_or_else(|| { panic!("failed to get home directory") });
    target_dir.push(".emsdk");
    let mut emsdk_path = target_dir.clone();
    emsdk_path.push("emsdk");
    if check_emsdk_install(&target_dir) {
        print_prefix();
        eprintln!("found emsdk installation at {}", target_dir.display());
    } else {
        print_prefix();
        eprintln!("emsdk not found, installing to {}...", target_dir.display());
        install_emsdk(&target_dir);
    }
    print_prefix();
    eprintln!("setting environment variables...");
    let env = get_env(&emsdk_path);
    let cmd_res = Command::new("emcc")
        .args(&["--version"])
        .envs(env.iter().map(|(k,v)| (OsStr::new(k), OsStr::new(v))))
        .output();
    if let Ok(_) = cmd_res {
        return env;
    }

    // well I guess that didn't work, so we'll have to actually update and activate our emsdk
    print_prefix();
    eprintln!("installing emcc with emsdk...");
    Command::new(&emsdk_path).args(&["update"])
        .current_dir(&target_dir)
        .status()
        .unwrap_or_else(|e| { panic!("failed to execute emsdk: {}", e) });
    Command::new(&emsdk_path).args(&["install", "latest"])
        .current_dir(&target_dir)
        .status()
        .unwrap_or_else(|e| { panic!("failed to execute emsdk: {}", e) });
    Command::new(&emsdk_path).args(&["activate", "latest"])
        .current_dir(&target_dir)
        .output()
        .unwrap_or_else(|e| { panic!("failed to execute emsdk: {}", e) });

    print_prefix();
    eprintln!("resetting environment variables...");
    let env = get_env(&emsdk_path);
    let cmd_res = Command::new("emcc")
        .args(&["--version"])
        .envs(env.iter().map(|(k,v)| (OsStr::new(k), OsStr::new(v))))
        .output();
    if let Ok(_) = cmd_res {
        return env;
    } else {
        print_prefix();
        eprintln!("failed to install emcc successfully");
        exit(1);
    }
}

fn main() {
    let mut args = std::env::args().skip(2).peekable();
    match args.peek().cloned() {
        None => {
            println!("cargo wasm version {}\n{}", VERSION, HELP_STRING);
            exit(0);
        },
        Some(subcommand) => {
            check_rustup_installed();
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
