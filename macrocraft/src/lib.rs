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

macro_rules! eat_tokens {($($token:tt)*)=>{};}

eat_tokens!{table_macro_name[$];
	_ => apple | banana | canteloupe;
	apple => {
		{"This is the Apple branch."},
		{"This is another element of the Apple branch."},
		{"This is a third element."}
	};
	banana => {
		{"This is the Banana branch."},
		{"The quick brown fox jumps over the lazy dog."},
		{"Patience is a virtue."}
	}
	canteloupe => {
		{"This is the Canteloupe branch."},
		{"Did I spell that right? I can't tell."},
		{"Why am I always making three elements?"}
	}
}

eat_tokens!{
	"test": "The quick brown fox jumps over the lazy dog.",
	rabbit: {
		id: "minecraft:rabbit",
		x: 3.0,
		y: 87.5,
		z: 588.0,
	},
	items: [
		${}
	],
}

#[proc_macro]
pub fn nbt(input: TokenStream) -> TokenStream {
	
	input
}
