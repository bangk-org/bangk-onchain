// File: bangk-ico/build.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 15 July 2024 @ 14:41:32
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::unwrap_used)]

use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let path_keys = Path::new(&out_dir).join("keys.rs");

    match env::var("BANGK_MODE").unwrap_or_default().as_str() {
        "MAINNET" => {
            write_mainnet_key(&path_keys);
        }
        "DEVNET" => {
            // println!("cargo:rustc-cfg=feature=\"debug-msg\"");
            write_devnet_key(&path_keys);
        }
        "TESTING" => {
            write_testing(&path_keys);
        }
        _ => {
            println!(
                "cargo:warning=Compiling bangk with unrecognized mode '{:?}': using TESTING",
                env::var("BANGK_MODE")
            );
            write_testing(&path_keys);
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=BANGK_MODE");
}

fn write_testing(path_keys: &Path) {
    write_testing_key(path_keys);
    println!("cargo:rustc-cfg=feature=\"debug-msg\"");
}

fn write_testing_key(dest_path: &Path) {
    fs::write(
        dest_path,
        "
/// Time to wait before executing a queued operation.
pub const TIMELOCK_DELAY: i64 = 5; // 5 seconds
/// Key used to initialize the program
pub const INIT_KEY: Pubkey = solana_program::pubkey!(\"HH9PXuEgE36MgMDq9hhY4gLGh4CEMUKPqLoW8UrjaiX3\");
",
    )
    .unwrap();
}

fn write_devnet_key(dest_path: &Path) {
    fs::write(
        dest_path,
        "
/// Time to wait before executing a queued operation.
pub const TIMELOCK_DELAY: i64 = 60*60; // 1 hour
/// Key used to initialize the program
pub const INIT_KEY: Pubkey = solana_program::pubkey!(\"8ryyq5XpbGe9z8vBmDpTnPG2VZPRwoXnFw3ugwQLXuLA\");
",
    )
    .unwrap();
}

fn write_mainnet_key(dest_path: &Path) {
    fs::write(
        dest_path,
        "
/// Time to wait before executing a queued operation.
pub const TIMELOCK_DELAY: i64 = 48 * 60 * 60; // 48 hours
/// Key used to initialize the program
pub const INIT_KEY: Pubkey = solana_program::pubkey!(\"8ryyq5XpbGe9z8vBmDpTnPG2VZPRwoXnFw3ugwQLXuLA\");
",
    )
    .unwrap();
}
