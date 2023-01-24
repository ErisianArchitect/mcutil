/*
Module for NBT format verification.
*/

use std::io::{
	Read, Write,
	Seek,
};

use crate::nbt::{
	*,
	tag::*,
	io::*,
};

pub fn verify_string<R: Read + Seek>(reader: &mut R) -> std::io::Result<bool> {
	todo!()
}

pub fn verify_named_tag<R: Read + Seek>(reader: &mut R) -> std::io::Result<bool> {
	todo!()
}