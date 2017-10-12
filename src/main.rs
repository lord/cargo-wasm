extern crate tempdir;
extern crate regex;

use std::process::{Command, exit, Stdio};
use std::fs::File;
use std::io::prelude::*;
use std::iter::Iterator;
use std::collections::BTreeMap;
use std::ffi::OsStr;

use tempdir::TempDir;
use regex::Regex;

const EMSDK_URL: &str =
    "https://s3.amazonaws.com/mozilla-games/emscripten/releases/emsdk-portable.tar.gz";

const PREFIX: &str = "\u{1b}[1;32m[wasmnow]\u{1b}[0m";

fn check_cmake_installed() {
    if let Err(e) = Command::new("cmake").args(&["--version"]).output() {
        if let std::io::ErrorKind::NotFound = e.kind() {
            if cfg!(target_os = "linux") {
                eprintln!("{} `cmake` not found. Try installing with `sudo apt-get install cmake` and rebuilding?", PREFIX);
            } else if cfg!(target_os = "macos") {
                eprintln!("{} `cmake` not found. Try installing with `brew install cmake` and rebuilding?", PREFIX);
            } else {
                eprintln!("{} `cmake` not found. Try installing and rebuilding?", PREFIX);
            }
        } else {
            eprintln!("{} Unknown error when checking installation of `cmake`: {:?}", PREFIX, e);
        }
        exit(1);
    }
}

fn check_rustup_installed() {
    if let Err(e) = Command::new("rustup").args(&["--version"]).output() {
        if let std::io::ErrorKind::NotFound = e.kind() {
            eprintln!("{} rustup installation not found. Try installing from https://rustup.rs", PREFIX);
        } else {
            eprintln!("{} Unknown error when checking rustup installation: {:?}", PREFIX, e);
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
        .unwrap_or_else(|e| { panic!("{} failed to execute curl: {}", PREFIX, e) });
    Command::new("tar").args(&["--strip-components=1", "-xvf", "-"])
        .stdin(cmd.stdout.unwrap_or_else(|| { panic!("{} failed to get curl output.", PREFIX) }))
        .current_dir(target_dir)
        .output()
        .unwrap_or_else(|e| { panic!("{} failed to execute tar: {}", PREFIX, e) });
}

fn get_env(emsdk_path: &std::path::PathBuf) -> BTreeMap<String, String> {
    let temp_dir = TempDir::new("cargo-wasmnow").unwrap_or_else(|_| { panic!("{} failed to create temp directory", PREFIX) });
    let re = Regex::new(r#"^export ([a-zA-Z0-9_\-]+)="(.*)""#).unwrap();
    Command::new(emsdk_path).args(&["construct_env"])
        .current_dir(&temp_dir)
        .output()
        .unwrap_or_else(|e| { panic!("{} failed to execute emsdk: {}", PREFIX, e) });
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
        eprintln!("{} using emcc already in $PATH", PREFIX);
        return BTreeMap::new();
    }

    // install emsdk if necessary, and then see if we can just use the env vars to get an emcc
    check_cmake_installed();
    let mut target_dir = std::env::home_dir().unwrap_or_else(|| { panic!("{} failed to get home directory", PREFIX) });
    target_dir.push(".emsdk");
    let mut emsdk_path = target_dir.clone();
    emsdk_path.push("emsdk");
    if check_emsdk_install(&target_dir) {
        eprintln!("{} found emsdk installation at {}", PREFIX, target_dir.display());
    } else {
        eprintln!("{} emsdk not found, installing to {}...", PREFIX, target_dir.display());
        install_emsdk(&target_dir);
    }
    eprintln!("{} setting environment variables...", PREFIX);
    let env = get_env(&emsdk_path);
    let cmd_res = Command::new("emcc")
        .args(&["--version"])
        .envs(env.iter().map(|(k,v)| (OsStr::new(k), OsStr::new(v))))
        .output();
    if let Ok(_) = cmd_res {
        return env;
    }

    // well I guess that didn't work, so we'll have to actually update and activate our emsdk
    eprintln!("{} installing emcc with emsdk...", PREFIX);
    Command::new(&emsdk_path).args(&["update"])
        .current_dir(&target_dir)
        .status()
        .unwrap_or_else(|e| { panic!("{} failed to execute emsdk: {}", PREFIX, e) });
    Command::new(&emsdk_path).args(&["install", "latest"])
        .current_dir(&target_dir)
        .status()
        .unwrap_or_else(|e| { panic!("{} failed to execute emsdk: {}", PREFIX, e) });
    Command::new(&emsdk_path).args(&["activate", "latest"])
        .current_dir(&target_dir)
        .output()
        .unwrap_or_else(|e| { panic!("{} failed to execute emsdk: {}", PREFIX, e) });

    eprintln!("{} resetting environment variables...", PREFIX);
    let env = get_env(&emsdk_path);
    let cmd_res = Command::new("emcc")
        .args(&["--version"])
        .envs(env.iter().map(|(k,v)| (OsStr::new(k), OsStr::new(v))))
        .output();
    if let Ok(_) = cmd_res {
        return env;
    } else {
        eprintln!("{} failed to install emcc successfully", PREFIX);
        exit(1);
    }
}

fn main() {
    check_rustup_installed();
    let env = ensure_installed();
    let _ = Command::new("cargo")
        .args(std::env::args().skip(2))
        .envs(env.iter().map(|(k,v)| (OsStr::new(k), OsStr::new(v))))
        .status();
}
