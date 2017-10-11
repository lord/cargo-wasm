extern crate tempdir;

use std::process::{Command, exit, Stdio};

use tempdir::TempDir;

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

fn main() {
    check_cmake_installed();
    let mut target_dir = std::env::home_dir().unwrap_or_else(|| { panic!("{} failed to get home directory", PREFIX) });
    let temp_dir = TempDir::new("cargo-wasmnow").unwrap_or_else(|_| { panic!("{} failed to create temp directory", PREFIX) });
    target_dir.push(".emsdk");
    if check_emsdk_install(&target_dir) {
        eprintln!("{} found emsdk installation at {}", PREFIX, target_dir.display());
    } else {
        eprintln!("{} emsdk not found, installing...", PREFIX);
        install_emsdk(&target_dir);
    }
    eprintln!("{} updating emsdk", PREFIX);
    Command::new("./emsdk").args(&["update"])
        .current_dir(&target_dir)
        .status()
        .unwrap_or_else(|e| { panic!("{} failed to execute emsdk: {}", PREFIX, e) });
    Command::new("./emsdk").args(&["install", "latest"])
        .current_dir(&target_dir)
        .status()
        .unwrap_or_else(|e| { panic!("{} failed to execute emsdk: {}", PREFIX, e) });
    Command::new("./emsdk").args(&["activate", "latest"])
        .current_dir(&target_dir)
        .status()
        .unwrap_or_else(|e| { panic!("{} failed to execute emsdk: {}", PREFIX, e) });
    Command::new("./emsdk").args(&["construct_env"])
        .current_dir(&temp_dir)
        .status()
        .unwrap_or_else(|e| { panic!("{} failed to execute emsdk: {}", PREFIX, e) });
}
