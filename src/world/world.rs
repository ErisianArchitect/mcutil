/*

*/
#![allow(unused)]

use std::{collections::HashMap, path::{PathBuf, Path}, marker::PhantomData};
use crate::{McResult, McError};

use super::{
	blockregistry::BlockRegistry,
	blockstate::*,
	chunk::Chunk,
	io::region::RegionFile,
};

pub type CoordTup = (i32, i32);

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Dimension {
	Overworld,
	Nether,
	Other(String),
}

// 32x32 chunks
// struct JavaRegion {

// }

pub trait ChunkManager<T: Sized>: Sized {
	fn load_chunk(world: &mut JavaWorld<T, Self>, coord: CoordTup) -> McResult<()>;
	fn save_chunk(world: &mut JavaWorld<T, Self>, coord: CoordTup) -> McResult<()>;
	/// Do not handle the removing of chunks from JavaWorld
	fn unload_chunk(world: &mut JavaWorld<T, Self>, coord: CoordTup) -> McResult<()>;
}

pub struct JavaWorld<Ct, M: ChunkManager<Ct>> {
	pub block_registry: BlockRegistry,
	pub chunks: HashMap<(i32, i32, Dimension), Ct>,
	pub regions: HashMap<(i32, i32, Dimension), RegionFile>,
	directory: PathBuf,
	_m: PhantomData<M>,
}

impl<Ct, M: ChunkManager<Ct>> JavaWorld<Ct, M> {
	pub fn open<P: AsRef<Path>>(directory: P) -> McResult<Self> {
		let directory = directory.as_ref().to_owned();
		if directory.is_dir() {
			todo!()
		} else {
			Err(McError::WorldDirectoryNotFound(directory))
		}
	}

	pub fn save(&mut self) -> McResult<()> {
		todo!()
	}
}

impl<M: ChunkManager<Chunk>> JavaWorld<Chunk, M> {
	pub fn load_chunk(&mut self, coord: CoordTup) -> McResult<()> {
		M::load_chunk(self, coord)
	}

	pub fn save_chunk(&mut self, coord: CoordTup) -> McResult<()> {
		M::save_chunk(self, coord)
	}
}

/*
World:
	chunks: HashMap<(i32, i32), ChunkType>
	
	Chunk Manager
		load_chunk
		save_chunk
	Block Registry
		register_block
		find_block
*/