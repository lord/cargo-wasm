extern crate tempdir;
extern crate term;
extern crate regex;
extern crate curl;

use std::process::{Command, exit, Stdio};
use std::fs::File;
use std::io::prelude::*;
use std::iter::{Iterator};
use std::collections::BTreeMap;
use std::ffi::OsStr;

use tempdir::TempDir;
use regex::Regex;
use curl::easy::Easy;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn print_prefix() {
    let mut t = term::stderr().unwrap();
    let _ = t.fg(term::color::GREEN);
    let _ = t.attr(term::Attr::Bold);
    write!(t, "  Setup wasm ").unwrap();
    t.reset().unwrap();
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

const EMSDK_WINDOWS_URL: &str =
    "https://s3.amazonaws.com/mozilla-games/emscripten/releases/emsdk-portable-64bit.zip";

fn check_installation(cmd: &str, args: &[&str], fail_msg: &str) {
    if let Err(e) = Command::new(cmd).args(args).output() {
        if let std::io::ErrorKind::NotFound = e.kind() {
            print_prefix();
            eprintln!("{}", fail_msg);
        } else {
            print_prefix();
            eprintln!("An unknown error occurred when checking for build dependencies.");
        }
        exit(1);
    }
}

fn check_emsdk_install(emsdk_path: &std::path::PathBuf) -> bool {
    Command::new(emsdk_path).args(&["--help"])
        .output()
        .is_ok()
}

fn install_emsdk(target_dir: &std::path::PathBuf) {
    let _ = std::fs::create_dir(target_dir.clone());
    let mut temp_path = TempDir::new("cargo-wasmnow2")
        .unwrap_or_else(|_| { panic!("failed to create temp directory") })
        .into_path();
    temp_path.push("emsdk.zip");
    let mut emsdk_data = Vec::new();
    {
        let mut easy = Easy::new();
        if cfg!(target_os = "windows") {
            easy.url(EMSDK_WINDOWS_URL).unwrap();
        } else {
            easy.url(EMSDK_URL).unwrap();
        }
       let mut transfer = easy.transfer();
       transfer.write_function(|data| {
            emsdk_data.extend_from_slice(data);
            Ok(data.len())
       }).unwrap();
       transfer.perform().unwrap();
    }
    if cfg!(target_os = "windows") {
        {
            let mut file = File::create(&temp_path).unwrap();
            file.write_all(&emsdk_data).unwrap();
            file.flush().unwrap();
        }
        Command::new("powershell.exe")
            .args(&[
                "-nologo",
                "-noprofile",
                "-command",
                &format!("& {{ Add-Type -A 'System.IO.Compression.FileSystem'; [IO.Compression.ZipFile]::ExtractToDirectory('{}', '.'); }}", &temp_path.display())
            ])
            .current_dir(target_dir)
            .status()
            .unwrap_or_else(|e| { panic!("failed to unzip file: {}", e) });
    } else {
        let mut child = Command::new("tar").args(&["--strip-components=1", "-zxvf", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .current_dir(target_dir)
            .spawn()
            .unwrap_or_else(|e| { panic!("failed to execute tar: {}", e) });

        child.stdin.as_mut().unwrap().write_all(&emsdk_data).unwrap();
        child.wait_with_output().unwrap();
    }
}

fn get_env(emsdk_path: &std::path::PathBuf) -> BTreeMap<String, String> {
    if cfg!(target_os = "windows") {
        let mut newpath = emsdk_path.clone();
        newpath.pop();
        newpath.push("emsdk_env.bat");
        Command::new(emsdk_path).output()
            .unwrap_or_else(|e| panic!("failed to get emsdk env: {}", e))

        BTreeMap::new()
    } else {
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
}

fn check_dependencies() {
    if cfg!(target_os = "macos") {
        check_installation("cmake",
            &["--version"],
            "cmake not found. Try installing with `brew install cmake` and rerunning?");
    } else if cfg!(target_os = "windows") {
        // check_installation("cmake",
        //     &["--version"],
        //     "cmake not found. Try installing with `brew install cmake` and rerunning?");
    } else {
        check_installation("gcc",
            &["--version"],
            "gcc not found. Try installing with `sudo apt-get install build-essential` and rerunning?");
        check_installation("cmake",
            &["--version"],
            "gcc not found. Try installing with `sudo apt-get install cmake` and rerunning?");
        check_installation("nodejs",
            &["--version"],
            "nodejs not found. Try installing with `sudo apt-get install nodejs` and rerunning?");
    }
}

// returns env needed to use emcc
fn ensure_installed() -> BTreeMap<String, String> {
    check_installation("rustup",
        &["target", "add", "wasm32-unknown-emscripten"],
        "rustup installation not found. Try installing from https://rustup.rs and rerunning?");

    // check if just already in path and working
    if let Ok(_) = Command::new("emcc").args(&["--version"]).output() {
        print_prefix();
        eprintln!("using emcc already in $PATH");
        return BTreeMap::new();
    }

    // install emsdk if necessary, and then see if we can just use the env vars to get an emcc
    check_dependencies();
    let mut target_dir = std::env::home_dir().unwrap_or_else(|| { panic!("failed to get home directory") });
    target_dir.push(".emsdk");
    let mut emsdk_path = target_dir.clone();
    if cfg!(target_os = "windows") {
        emsdk_path.push("emsdk.bat");
    } else {
        emsdk_path.push("emsdk");
    }
    if check_emsdk_install(&emsdk_path) {
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
