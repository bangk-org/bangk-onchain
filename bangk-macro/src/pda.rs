// File: bangk-macro/src/pda.rs
// Project: bangk-onchain
// Creation date: Thursday 25 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 14 August 2024 @ 19:22:16
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use darling::{ast::NestedMeta, util::parse_expr, Error, FromMeta};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    parse::Parser, parse_macro_input, DeriveInput, Expr, ExprField, ExprLit, ExprPath, Ident,
    Lifetime, LifetimeParam, Member,
};

#[derive(Debug, FromMeta)]
struct PdaArgs {
    kind: syn::Path,
    #[darling(multiple, with = parse_expr::preserve_str_literal)]
    seed: Vec<Expr>,
}

// Get the seeds as they will be used to sign an instruction.
fn get_seed(seed: &Expr) -> TokenStream {
    match seed {
        Expr::Lit(ExprLit { lit: value, .. }) => quote! { Seed::from(#value) }.into(),
        Expr::Field(field) => quote! { Seed::from(self.#field) }.into(),
        Expr::Path(ExprPath { path, .. }) => {
            if path.segments.len() > 1 {
                panic!("seed should either be a literal, an ident or a field");
            }
            quote! { Seed::from(self.#path) }.into()
        }
        _ => panic!("seed should be either literal, an ident or a field"),
    }
}

// Make the seeds to sign a transaction, which will be used in the Pda::seeds() function.
fn make_pda_seed(seeds: &[TokenStream]) -> TokenStream {
    let res = seeds.iter().fold(quote! {}, |acc, elt| {
        let elt: proc_macro2::TokenStream = elt.clone().into();
        if acc.is_empty() {
            quote! { #elt }
        } else {
            quote! { #acc, #elt }
        }
    });

    quote! { Vec::from([#res, Seed::from(self.bump)]) }.into()
}

// Gets the data used to create the Pda::get_address()
// -> the ident of the function’s parameters,
// -> the documentation for the name of those parameters,
// -> the casts needed on those parameters,
// -> the seeds as they are passed to Pubkey::find_program_address()
fn get_address_data(
    seeds: &[Expr],
) -> (
    proc_macro2::TokenStream,
    String,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
) {
    let data: Vec<(proc_macro2::TokenStream, Option<Ident>)> =
        seeds.iter().map(get_address_seed).collect();
    let mut params = quote! {};
    let mut doc = String::new();
    let mut casts = quote! {};
    let mut seeds = quote! {};

    for (seed, ident) in data {
        if let Some(ident) = ident {
            if params.is_empty() {
                params = quote! { #ident: I, };
                doc = format!("* `{ident}`\n");
                casts = quote! {
                    let #ident: Seed = #ident.into();
                    let #ident: Vec<u8> = #ident.into();
                };
            } else {
                params = quote! { #params #ident: I, };
                doc = format!("{doc}* `{ident}`\n");
                casts = quote! {
                    #casts
                    let #ident: Seed = #ident.into();
                    let #ident: Vec<u8> = #ident.into();
                };
            }
        }

        if seeds.is_empty() {
            seeds = quote! {
                &#seed
            };
        } else {
            seeds = quote! {
                #seeds, &#seed
            };
        }
    }
    (params, doc, casts, seeds)
}

// Retrieve the seed and its ident if there’s one for each seed attribute on the macro.
fn get_address_seed(expr: &Expr) -> (proc_macro2::TokenStream, Option<Ident>) {
    match expr {
        Expr::Lit(ExprLit { lit, .. }) => (quote! {#lit.as_bytes().to_vec()}, None),
        Expr::Field(ExprField {
            member: Member::Named(ident),
            ..
        }) => (quote! { #ident }, Some(ident.clone())),
        Expr::Path(ExprPath { path, .. }) => {
            let ident = path.segments.first().unwrap().ident.clone();
            (quote! { #ident }, Some(ident.clone()))
        }
        _ => (quote! {}, None),
    }
}

fn get_address_fn_str(
    crate_name: &Ident,
    params: TokenStream2,
    casts: TokenStream2,
    seeds: TokenStream2,
) -> proc_macro2::TokenStream {
    if params.is_empty() {
        quote! {
            pub fn get_address(program_id: &Pubkey) -> (Pubkey, u8) {
                Pubkey::find_program_address(&[#seeds], program_id)
            }
        }
    } else {
        quote! {
            pub fn get_address<I>(#params program_id: &Pubkey) -> (Pubkey, u8)
            where
                I: Into<#crate_name::pda::Seed>,
            {
                use #crate_name::pda::Seed;

                #casts
                Pubkey::find_program_address(&[#seeds], program_id)
            }
        }
    }
}

fn check_address_fn_str(
    crate_name: &Ident,
    params: TokenStream2,
    casts: TokenStream2,
    seeds: TokenStream2,
) -> proc_macro2::TokenStream {
    if params.is_empty() {
        quote! {
            /// Checks that the account is the expected one
            ///
            /// # Errors
            /// If the address doesn't match the expected one.
            pub fn check_address<'b>(program_id: &Pubkey, account: &solana_program::account_info::AccountInfo<'b>) -> #crate_name::Result<()> {
                let expected = Pubkey::find_program_address(&[#seeds], program_id).0;
                if expected == *account.key {
                    Ok(())
                } else {
                    solana_program::msg!("PDA address mismatch: expected {}, got {}", expected, account.key);
                    Err(#crate_name::Error::InvalidPdaAddress)
                }
            }
        }
    } else {
        quote! {
            /// Checks that the account is the expected one
            ///
            /// # Errors
            /// If the address doesn't match the expected one.
            pub fn check_address<'b, I>(#params program_id: &Pubkey, account: &solana_program::account_info::AccountInfo<'b>) -> #crate_name::Result<()>
            where
                I: Into<#crate_name::pda::Seed>,
            {
                use #crate_name::pda::Seed;

                #casts
                let expected = Pubkey::find_program_address(&[#seeds], program_id).0;
                if expected == *account.key {
                    Ok(())
                } else {
                    solana_program::msg!("PDA address mismatch: expected {}, got {}", expected, account.key);
                    Err(#crate_name::Error::InvalidPdaAddress)
                }
            }
        }
    }
}

// Implements most of a PDA’s boilerplate.
pub fn impl_pda(attrs: TokenStream, input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let name = ast.ident.clone();

    // Parse argument tokens as a list of NestedMeta items
    let attr_args = match NestedMeta::parse_meta_list(attrs.into()) {
        Ok(v) => v,
        Err(e) => {
            // Write error to output token stream if there is one
            return proc_macro::TokenStream::from(Error::from(e).write_errors());
        }
    };

    // Parse the nested meta list as our `CachedParams` struct
    let PdaArgs { seed, kind } = match PdaArgs::from_list(&attr_args) {
        Ok(params) => params,
        Err(error) => {
            // Write error to output token stream if there is one
            return proc_macro::TokenStream::from(error.write_errors());
        }
    };

    if seed.is_empty() {
        return proc_macro::TokenStream::from(Error::missing_field("seed").write_errors());
    }

    let crate_ident = match std::env::var("CARGO_PKG_NAME").unwrap().as_str() {
        "bangk-onchain-common" => format_ident!("crate"),
        _ => format_ident!("bangk_onchain_common"),
    };
    let (get_address_params, get_address_doc, get_address_casts, get_address_seeds) =
        get_address_data(&seed);
    let seeds = seed.iter().map(get_seed).collect::<Vec<_>>();
    let pda_seeds: proc_macro2::TokenStream = make_pda_seed(&seeds).into();

    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            if let syn::Fields::Named(fields) = &mut struct_data.fields {
                fields.named.insert(
                    0,
                    syn::Field::parse_named
                        .parse2(quote! {
                            /// Type of the PDA
                            pub pda_type: PdaType
                        })
                        .unwrap(),
                );
                fields.named.insert(
                    1,
                    syn::Field::parse_named
                        .parse2(quote! {
                            /// Bump for the PDA
                            pub bump: u8
                        })
                        .unwrap(),
                );
                fields.named.insert(
                    2,
                    syn::Field::parse_named
                        .parse2(quote! {
                            /// Account where the PDA is stored, will not be serialized
                            #[borsh(skip)]
                            pub account: Option<solana_program::account_info::AccountInfo<'a>>
                        })
                        .unwrap(),
                );
            }
            ast.generics
                .params
                .push(syn::GenericParam::Lifetime(LifetimeParam::new(
                    Lifetime::new("'a", Span::mixed_site()),
                )));

            let get_address_fn = get_address_fn_str(
                &crate_ident,
                get_address_params.clone(),
                get_address_casts.clone(),
                get_address_seeds.clone(),
            );
            let check_address_fn = check_address_fn_str(
                &crate_ident,
                get_address_params,
                get_address_casts,
                get_address_seeds,
            );

            // That fugly. But it’s also the only way I managed to make it work (otherwise cargo test thinks it’s a doc test…)
            let get_address_doc = format!(
                "Get the PDA's address.\n\n All parameters that are not the program_id must implement `Into<Seed>`.\n\n # Parameters\n {get_address_doc} * `program_id` - Program owning the PDA.\n\n # Returns\n\n * Tuple of public Key of the investment record and associated bump");
            quote! {
                #[derive(Debug, borsh::BorshSerialize, borsh::BorshDeserialize, shank::ShankAccount)]
                #ast

                #[automatically_derived]
                impl<'a> BangkPda<'a> for #name<'a> {

                    const PDA_TYPE: PdaType = #kind;

                    fn get_bump(&self) -> u8 {
                        self.bump
                    }

                    fn is_valid(&self) -> bool {
                        self.pda_type == Self::PDA_TYPE
                    }

                    fn get_account(&self) -> core::result::Result<&solana_program::account_info::AccountInfo<'a>, #crate_ident::Error> {
                        self.account.as_ref().ok_or(#crate_ident::Error::MissingPDAAccount)
                    }

                    #[must_use]
                    fn seeds(&self) -> Vec<Vec<u8>> {
                        use #crate_ident::pda::Seed;
                        let seeds = #pda_seeds;
                        let mut res = Vec::new();
                        for seed in seeds {
                            let seed: Vec<u8> = seed.into();
                            res.push(seed);
                        }
                        res
                    }
                }

                #[automatically_derived]
                impl<'a> #name<'a> {
                    #[doc = #get_address_doc]
                    #[must_use]
                    #get_address_fn
                    #check_address_fn
                    /// Loads a PDA data from an account.
                    ///
                    /// # Parameters
                    /// * `account` - Account from which to read the data
                    ///
                    /// # Errors
                    /// If the given account does not contain the expected data.
                    // #[cfg(not(feature = "no-entrypoint"))]
                    pub fn from_account(account: &solana_program::account_info::AccountInfo<'a>)
                        -> core::result::Result<Self, solana_program::program_error::ProgramError> {
                        let data = account.try_borrow_data()?;
                        let mut res = Self::try_from_slice(&data)?;
                        res.account = Some(account.clone());
                        if res.pda_type != Self::PDA_TYPE {
                            return Err(#crate_ident::Error::InvalidPdaType.into());
                        }

                        Ok(res)
                    }
                }

            }
            .into()
        }
        _ => syn::Error::new(
            ast.ident.span(),
            "the PDA attribute can only be used on a struct,",
        )
        .into_compile_error()
        .into(),
    }
}
