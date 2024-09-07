// File: bangk/build.rs
// Project: bangk-onchain
// Creation date: Friday 23 February 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::unwrap_used)]

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
/// Bangk public key (just a testing key)
pub const BANGK: Pubkey = pubkey!(\"HH9PXuEgE36MgMDq9hhY4gLGh4CEMUKPqLoW8UrjaiX3\");
/// Permanent delegate for bangk operations
pub const DELEGATE: Pubkey = pubkey!(\"C5ohokxYya5dEhQG9pyB6n8rc4i9F6wbMioGmsf6Eb9U\");
",
    )
    .unwrap();
}

fn write_devnet_key(dest_path: &Path) {
    fs::write(
        dest_path,
        "
/// Bangk public key
pub const BANGK: Pubkey = pubkey!(\"BangkWeKrogcm29StE8tWdMPVedjY8TdFJQA9DXnKCJw\");
/// Permanent delegate for bangk operations
pub const DELEGATE: Pubkey = pubkey!(\"BangkB4wfm3rUrhdR9WEy6jaYT4dDwE6YQkirTtd9t4L\");
",
    )
    .unwrap();
}

fn write_mainnet_key(dest_path: &Path) {
    fs::write(
        dest_path,
        "
/// Bangk public key
pub const BANGK: Pubkey = pubkey!(\"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\");
/// Permanent delegate for bangk operations
pub const DELEGATE: Pubkey = pubkey!(\"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\");
",
    )
    .unwrap();
}

fn write_testing_id(dest_path: &Path) {
    fs::write(
        dest_path,
        "
solana_program::declare_id!(\"BKPrg2BFZLMzLtujrsT7ayVewgVCGkKUwdB9e3E6Kzyp\");
",
    )
    .unwrap();
}

fn write_devnet_id(dest_path: &Path) {
    fs::write(
        dest_path,
        "
solana_program::declare_id!(\"BKPrg2BFZLMzLtujrsT7ayVewgVCGkKUwdB9e3E6Kzyp\");
",
    )
    .unwrap();
}

fn write_mainnet_id(dest_path: &Path) {
    fs::write(
        dest_path,
        "
solana_program::declare_id!(\"BKPrg2BFZLMzLtujrsT7ayVewgVCGkKUwdB9e3E6Kzyp\");
",
    )
    .unwrap();
}
