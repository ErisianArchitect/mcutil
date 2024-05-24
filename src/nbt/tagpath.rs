use std::fmt::{Display, Debug};
use std::ops::Index;
use std::slice::SliceIndex;
use std::str::FromStr;

use chumsky::prelude::*;
use chumsky::primitive::{
    Container,
    OneOf,
    NoneOf,
};
use chumsky::Error;

use thiserror::Error;

use crate::nbt::tag::*;

#[derive(PartialEq, Eq,PartialOrd, Ord, Clone, Hash, Debug)]
pub enum TagPathPart {
    AtIndex(i64),
    AtKey(String),
}

macro_rules! tag_path_part_from_impl {
    ($valname:ident : $from_type:ty; AtKey($value:expr)) => {
        impl From<$from_type> for TagPathPart {
            fn from($valname: $from_type) -> Self {
                TagPathPart::AtKey($value)
            }
        }
    };
    ($valname:ident : $from_type:ty; AtIndex($value:expr)) => {
        impl From<$from_type> for TagPathPart {
            fn from($valname: $from_type) -> Self {
                TagPathPart::AtIndex($value)
            }
        }
    };
    ($from_type:ty; Numeric) => {
        impl From<$from_type> for TagPathPart {
            fn from(value: $from_type) -> Self {
                TagPathPart::AtIndex(value as i64)
            }
        }
    };
    ($valname:ident : $from_type:ty; $value:expr) => {
        impl From<$from_type> for TagPathPart {
            fn from($valname: $from_type) -> Self {
                $value
            }
        }
    };
}

tag_path_part_from_impl!(value:&str; AtKey(value.to_owned()));
tag_path_part_from_impl!(value:String; AtKey(value));

tag_path_part_from_impl!(isize; Numeric);
tag_path_part_from_impl!(usize; Numeric);
tag_path_part_from_impl!(u64; Numeric);
tag_path_part_from_impl!(i64; Numeric);
tag_path_part_from_impl!(i32; Numeric);
tag_path_part_from_impl!(u32; Numeric);
tag_path_part_from_impl!(i16; Numeric);
tag_path_part_from_impl!(u16; Numeric);
tag_path_part_from_impl!(i8; Numeric);
tag_path_part_from_impl!(u8; Numeric);

#[derive(Debug, Error)]
pub enum TagPathError {
    #[error("Tokenize Error")]
    TokenizeError(Vec<Simple<char>>),
    #[error("Parse Error")]
    ParseError(Vec<Simple<TagPathToken>>),
    #[error("Invalid token.")]
    InvalidToken(TagPathToken),
}

#[derive(PartialEq, Eq,PartialOrd, Ord, Clone, Hash, Debug)]
pub struct TagPath(pub Vec<TagPathPart>);

impl TagPath {
    pub fn parse<S: AsRef<str>>(source: S) -> Result<Self, TagPathError> {
        let tokens = TagPathToken::parse(source)
            .map_err(TagPathError::TokenizeError)?;
        let path = tag_path_parser().parse(tokens)
            .map_err(TagPathError::ParseError)?;
        Ok(Self(path))
    }

    pub fn path(&self) -> &[TagPathPart] {
        &self.0
    }

    pub fn join<T: Into<TagPathPart>>(&self, path: T) -> TagPath {
        let mut parts = self.0.clone();
        parts.push(path.into());
        TagPath(parts)
    }
}

impl FromStr for TagPath {
    type Err = TagPathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        TagPath::parse(s)
    }
}

#[derive(PartialEq, Eq,PartialOrd, Ord, Clone, Hash, Debug)]
pub enum TagPathToken {
    Dot,
    OpenBracket,
    CloseBracket,
    Integer(String),
    Identifier(String),
    StringLiteral(String),
}

// I made it easier to make the lexer. Since there is a lot of boilerplate involved, I wrote
// a macro that allows me to bypass writing all the error-prone boilerplate.
// It also allows me to generate a parse function that will parse in the order that I define
// sub-parsers.
// So the syntax for the parsers is similar to the syntax for match arms.
// First you have the name that you want to apply to the function, then "=>", then a block
// for the parser:
//     name => { /* parser initialization */ }
macro_rules! token_parse_functions {
    ($($name:ident => $block:block)+) => {
        impl TagPathToken {
            $(
                pub fn $name() -> impl Parser<char, TagPathToken, Error = Simple<char>>
                $block
            )+

            pub fn parse<S: AsRef<str>>(source: S) -> Result<Vec<TagPathToken>, Vec<Simple<char>>> {
                choice((
                    $(
                        Self::$name(),
                    )+
                ))
                .padded() // each token may be padded with whitespace
                .repeated().at_least(1)
                .then_ignore(end()) // Force read until end.
                .collect::<Vec<TagPathToken>>()
                .parse(source.as_ref())
            }
        }
    };
}

token_parse_functions!{
    open_bracket => { just('[').to(TagPathToken::OpenBracket).labelled("Open Bracket") }
    dot => { just('.').to(TagPathToken::Dot).labelled("Dot") }
    close_bracket => { just(']').to(TagPathToken::CloseBracket).labelled("Close Bracket") }
    // If I want, I can add binary and hex literals.
    integer => {
        just::<char, _, Simple<char>>('-')
            .or_not()
            .chain::<char, _, _>(text::int(10))
            .collect::<String>()
            .then_ignore(choice((
                filter(|c: &char| {
                    !c.is_alphanumeric() && !['_', '+','-','.'].contains(c)
                }),
                end().to('\0')
            )).rewind())
            .map(|(int_text)| TagPathToken::Integer(int_text))
            .labelled("Integer")
    }
    identifier => {
        choice((
            filter(char::is_ascii_alphanumeric),
            one_of("+-_")
        ))
        .repeated().at_least(1)
        .collect::<String>()
        .map(TagPathToken::Identifier)
        .labelled("Identifier")
    }
    string_literal => {
        let escape = just::<_,_,Simple<char>>('\\').ignore_then(
            just('\\')
                .or(just('/'))
                .or(just('"'))
                .or(just('\'')) // Look carefully, this is -> '
                .or(just('b').to('\x08'))
                .or(just('f').to('\x0C'))
                .or(just('n').to('\n'))
                .or(just('r').to('\r'))
                .or(just('t').to('\t'))
        );
        TagPathToken::identifier().or(
            choice::<_,Simple<char>>((
                just('"')
                    .ignore_then(
                        none_of("\\\"").or(escape.clone()).repeated()
                    )
                    .then_ignore(just('"'))
                    .collect::<String>(),
                just('\'')
                    .ignore_then(
                        none_of("\\'").or(escape.clone()).repeated()
                    )
                    .then_ignore(just('\''))
                    .collect::<String>(),
            )).map(TagPathToken::StringLiteral))
            .labelled("String Literal")
    }
}

/// Returns a parser that takes [TagPathToken] as input and returns a [Tag].
fn tag_path_parser() -> impl Parser<TagPathToken, Vec<TagPathPart>, Error = Simple<TagPathToken>> {
    let bracketed = just(TagPathToken::OpenBracket).ignore_then(
        filter(|token| matches!(token, TagPathToken::Integer(_) | TagPathToken::StringLiteral(_) | TagPathToken::Identifier(_)))
            .try_map(|token, span| {
                match token {
                    TagPathToken::Integer(digits) => {
                        digits.parse::<i64>()
                            .map(TagPathPart::AtIndex)
                            .map_err(|_| Simple::custom(span, "Failed to parse i64."))
                    },
                    TagPathToken::Identifier(ident) => Ok(TagPathPart::AtKey(ident)),
                    TagPathToken::StringLiteral(ident) => Ok(TagPathPart::AtKey(ident)),
                    _ => Err(Simple::custom(span, "Impossible failure.")),
                }
            })
    ).then_ignore(just(TagPathToken::CloseBracket));

    let ident = filter(|token| matches!(token, TagPathToken::Identifier(_)))
        .try_map(|token, span| {
            match token {
                TagPathToken::Identifier(ident) => Ok(TagPathPart::AtKey(ident)),
                _ => Err(Simple::custom(span, "Impossible failure.")),
            }
        });

    let dot = just(TagPathToken::Dot).ignore_then(ident.clone());

    let part = choice((
        bracketed,
        dot,
    ));

    choice((
        ident.chain(part.clone().repeated().then_ignore(end())),
        part.repeated().then_ignore(end())
    ))
}

impl Display for TagPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.iter().try_for_each(|part| {
            match part {
                TagPathPart::AtIndex(index) => write!(f, "[{index}]")?,
                TagPathPart::AtKey(key) => {
                    if crate::nbt::format::is_identifier(key) {
                        write!(f, "{key}")?;
                    } else {
                        write!(f, "[\"")?;
                        crate::nbt::format::write_escaped_string(f, key)?;
                        write!(f, "\"]")?;
                    }
                },
            }
            Ok(())
        })
        // let mut remaining = self.0.as_slice();
        // if remaining.len() > 0 {
        // 	match &remaining[0] {
        // 		TagPathPart::AtIndex(index) => write!(f, "[{index}]")?,
        // 		TagPathPart::AtKey(key) => {
        // 			if crate::nbt::format::is_identifier(key) {
        // 				write!(f, "{key}")?;
        // 			} else {
        // 				write!(f, "[\"")?;
        // 				crate::nbt::format::write_escaped_string(f, key)?;
        // 				write!(f, "\"]")?;
        // 			}
        // 		},
        // 	}
        // 	remaining = &remaining[1..];
        // 	while !remaining.is_empty() {
        // 		match &remaining[0] {
        // 			TagPathPart::AtIndex(index) => write!(f, "[{index}]")?,
        // 			TagPathPart::AtKey(key) => {
        // 				if crate::nbt::format::is_identifier(key) {
        // 					write!(f, ".{key}")?;
        // 				} else {
        // 					write!(f, "[\'")?;
        // 					crate::nbt::format::write_escaped_string(f, key)?;
        // 					write!(f, "\']")?;
        // 				}
        // 			},
        // 		}
        // 		remaining = &remaining[1..];
        // 	}
        // }
        // Ok(())
    }
}

pub trait GetChild {
    type ReturnType;
    fn get_child(&self) -> Self::ReturnType;
}