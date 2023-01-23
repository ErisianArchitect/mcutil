// TODO:Remove this when the code is ready.
//      This is only used to temporarily get rid of warnings.
#![allow(unused)]

extern crate proc_macro;
use std::{ops::ControlFlow, collections::HashSet};

use proc_macro::TokenStream;
use quote::{
    quote,
    quote_spanned, ToTokens, TokenStreamExt,
};
use syn::{
    parse::{
        Parse,
        ParseStream,
        Result,
    },
    token::{
        self, Continue,
    },
    punctuated::Punctuated,
    spanned::Spanned,
    parse_macro_input,
    Expr,
    Ident,
    Type,
    Visibility,
    Block,
    Token,
    parenthesized,
};

pub(crate) enum NbtTag {
}