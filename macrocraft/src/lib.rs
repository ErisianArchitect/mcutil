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

#[proc_macro]
pub fn nbt(input: TokenStream) -> TokenStream {
	macro_rules! mac {
		($($($name:literal)?$(($name_expr:expr))? : $value:expr),+) => {
			mac!{{$($($name)?$($name_expr)? : $value,)+}}
		};
		({$($($name:literal)?$(($name_expr:expr))? : $value:expr),+}) => {
			$(println!("{}", $value);)+
		};
		([$($prefix:ident;)?$($value:expr),*]) => {

		};
		($($value:expr),+) => {

		};
		($array_prefix:ident; $($value:expr),*) => {

		};
		(@array;B; $($value:expr),*) => {

		};
		(@array;I;) => {
	
		};
		(@array;L;) => {
	
		};
	}
	mac!{{
		"test" : 1234,
		"one" : "Hello, world!"
	}}
	input
}
