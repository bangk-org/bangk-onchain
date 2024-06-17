// File: bangk-macro/src/lib.rs
// Project: bangk-onchain
// Creation date: Thursday 25 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 25 July 2024 @ 19:59:05
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// #![warn(missing_docs)]

mod pda;
use pda::impl_pda;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn pda(attrs: TokenStream, input: TokenStream) -> TokenStream {
    impl_pda(attrs, input)
}
