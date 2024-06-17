// File: bangk-ico/build.rs
// Project: bangk-solana
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.fr>
// -----
// Last modified: Thursday 13 June 2024 @ 17:31:13
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::unwrap_used)]
#![allow(clippy::print_stdout)]

use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let path_keys = Path::new(&out_dir).join("keys.rs");
    let path_id = Path::new(&out_dir).join("program_id.rs");

    match env::var("BANGK_MODE").unwrap_or_default().as_str() {
        "MAINNET" => {
            write_mainnet_key(&path_keys);
            write_mainnet_id(&path_id);
        }
        "DEVNET" => {
            // println!("cargo:rustc-cfg=feature=\"debug-msg\"");
            write_devnet_key(&path_keys);
            write_devnet_id(&path_id);
        }
        "TESTING" => {
            write_testing(&path_keys, &path_id);
        }
        _ => {
            println!(
                "cargo:warning=Compiling bangk with unrecognized mode '{:?}': using TESTING",
                env::var("BANGK_MODE")
            );
            write_testing(&path_keys, &path_id);
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=BANGK_MODE");
}

fn write_testing(path_keys: &Path, path_id: &Path) {
    write_testing_key(path_keys);
    write_testing_id(path_id);
    println!("cargo:rustc-cfg=feature=\"debug-msg\"");
}

fn write_testing_key(dest_path: &Path) {
    fs::write(
        dest_path,
        "
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
/// Key used to initialize the program
pub const BANGK: Pubkey = solana_program::pubkey!(\"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\");
",
    )
    .unwrap();
}

fn write_mainnet_key(dest_path: &Path) {
    fs::write(
        dest_path,
        "
/// Key used to initialize the program
pub const BANGK: Pubkey = solana_program::pubkey!(\"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\");
",
    )
    .unwrap();
}

fn write_testing_id(dest_path: &Path) {
    fs::write(
        dest_path,
        "
solana_program::declare_id!(\"CCj7VqoXZxn2ZGs8TPKGW27AWu5Wsf7qhLx4Xo2YUsBb\");
",
    )
    .unwrap();
}

fn write_devnet_id(dest_path: &Path) {
    fs::write(
        dest_path,
        "
solana_program::declare_id!(\"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\");
",
    )
    .unwrap();
}

fn write_mainnet_id(dest_path: &Path) {
    fs::write(
        dest_path,
        "
solana_program::declare_id!(\"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\");
",
    )
    .unwrap();
}
