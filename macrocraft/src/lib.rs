// TODO:Remove this when the code is ready.
//      This is only used to temporarily get rid of warnings.
#![allow(unused)]

extern crate proc_macro;
use std::{ops::ControlFlow, collections::HashSet};

use proc_macro::{TokenStream};
use quote::{
    quote,
    quote_spanned, ToTokens,
};

use syn::{
    parse::{
        Parse,
        ParseStream,
        Result,
    },
    token::{
        self,
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
	visit_mut,
	visit,
};

// struct visitor {

// }

// impl visit_mut::VisitMut for visitor {
// 	fn visit_block_mut(&mut self, i: &mut Block) {

// 	}
// }


#[proc_macro]
pub fn nbt(input: TokenStream) -> TokenStream {
	
	input
}
