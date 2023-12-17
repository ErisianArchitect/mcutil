/*

*/
#![allow(unused)]

use std::{collections::HashMap, path::{PathBuf, Path}, marker::PhantomData, sync::{Arc, Mutex}};

use crate::{McResult, McError, nbt::tag::NamedTag};

use super::{
	blockregistry::BlockRegistry,
	blockstate::*,
	chunk::{Chunk, decode_chunk},
	io::region::{
		RegionFile,
		coord::RegionCoord,
		regionfile::{
			RegionManager,
		},
	},
};

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Dimension {
	Overworld,
	Nether,
	Other(u32),
}

impl Default for Dimension {
	fn default() -> Self {
		Dimension::Overworld
	}
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub struct WorldCoord {
	pub x: i64,
	pub z: i64,
	pub dimension: Dimension,
}

impl WorldCoord {
	pub fn new(x: i64, z: i64, dimension: Dimension) -> Self {
		Self {
			x,
			z,
			dimension
		}
	}

	pub fn xz(self) -> (i64, i64) {
		(
			self.x,
			self.z
		)
	}

	pub fn region_coord(self) -> Self {
		Self {
			x: self.x / 32,
			z: self.z / 32,
			dimension: self.dimension,
		}
	}
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub struct BlockCoord {
	pub x: i64,
	pub y: i64,
	pub z: i64,
	pub dimension: Dimension,
}

impl BlockCoord {
	pub fn new(x: i64, y: i64, z: i64, dimension: Dimension) -> Self {
		Self {
			x,
			y,
			z,
			dimension,
		}
	}

	pub fn xyz(self) -> (i64, i64, i64) {
		(
			self.x,
			self.y,
			self.z,
		)
	}

	pub fn subchunk_coord(self) -> Self {
		BlockCoord {
			x: self.x.rem_euclid(16),
			y: self.y.rem_euclid(16),
			z: self.z.rem_euclid(16),
			dimension: self.dimension,
		}
	}
}

// 32x32 chunks
// struct JavaRegion {

// }

pub trait ChunkManager: Sized {
	fn create(directory: PathBuf) -> McResult<Self>;
	fn load_chunk(&mut self, block_registry: &mut BlockRegistry, coord: WorldCoord) -> McResult<()>;
	fn save_chunk(&self, block_registry: &BlockRegistry, coord: WorldCoord) -> McResult<()>;
	fn save_all(&self, block_registry: &BlockRegistry) -> McResult<()>;
	fn unload_chunk(&mut self, coord: WorldCoord) -> McResult<()>;

	fn get_block_id(&self, block_registry: &BlockRegistry, coord: BlockCoord) -> McResult<u32>;
	fn get_block_state(&self, block_registry: &BlockRegistry, coord: BlockCoord) -> McResult<BlockState>;
	fn set_block_id(&self, block_registry: &mut BlockRegistry, coord: BlockCoord, id: u32) -> McResult<()>;
	fn set_block_state(&self, block_registry: &mut BlockRegistry, coord: BlockCoord, state: BlockState) -> McResult<()>;
}

pub struct JavaChunkManager {
	pub chunks: HashMap<WorldCoord, Arc<Mutex<Chunk>>>,
	pub regions: HashMap<WorldCoord, Arc<Mutex<RegionFile>>>,
	pub directory: PathBuf,
}

#[inline(always)]
fn make_arcmutex<T>(value: T) -> Arc<Mutex<T>> {
	Arc::new(Mutex::new(value))
}

impl ChunkManager for JavaChunkManager {
	fn create(directory: PathBuf) -> McResult<Self> {
		if directory.is_dir() {
			Ok(Self {
				directory,
				chunks: HashMap::new(),
				regions: HashMap::new(),
			})
		} else {
			Err(McError::WorldDirectoryNotFound(directory))
		}
	}

	fn load_chunk(&mut self, block_registry: &mut BlockRegistry, coord: WorldCoord) -> McResult<()> {
		match coord.dimension {
			Dimension::Overworld => {
				let region_dir = self.directory.join("region");
				let region_coord = coord.region_coord();
				let (x, z) = region_coord.xz();
				let file_path = format!("r.{x}.{z}.mca");
				let file_path = region_dir.join(file_path);
				let region_file = if !self.regions.contains_key(&region_coord) {
					// Now to load the RegionFile.
					let region_file = if file_path.is_file() {
						make_arcmutex(RegionFile::open(file_path)?)
					} else {
						// If the file doesn't exist, we'll create a region file.
						make_arcmutex(RegionFile::create(file_path)?)
					};
					self.regions.insert(region_coord, region_file.clone());
					region_file
				} else {
					self.regions.get(&region_coord).unwrap().clone()
				};
				if let Ok(mut region) = region_file.lock() {
					let chunk_tag: NamedTag = region.read_data(RegionCoord::from(region_coord.xz()))?;
					let chunk = decode_chunk(block_registry, chunk_tag.tag)?;
					self.chunks.insert(coord, make_arcmutex(chunk));
				}
				Ok(())
			},
			Dimension::Nether => todo!(),
			Dimension::Other(_) => todo!(),
		}
	}

	fn save_chunk(&self, block_registry: &BlockRegistry, coord: WorldCoord) -> McResult<()> {
		todo!()
	}

	fn save_all(&self, block_registry: &BlockRegistry) -> McResult<()> {
		todo!()
	}

	fn unload_chunk(&mut self, coord: WorldCoord) -> McResult<()> {
		todo!()
	}

	fn get_block_id(&self, block_registry: &BlockRegistry, coord: BlockCoord) -> McResult<u32> {
		todo!()
	}

	fn get_block_state(&self, block_registry: &BlockRegistry, coord: BlockCoord) -> McResult<BlockState> {
		todo!()
	}

	fn set_block_id(&self, block_registry: &mut BlockRegistry, coord: BlockCoord, id: u32) -> McResult<()> {
		todo!()
	}

	fn set_block_state(&self, block_registry: &mut BlockRegistry, coord: BlockCoord, state: BlockState) -> McResult<()> {
		todo!()
	}

	
}

pub struct JavaWorld<M: ChunkManager> {
	pub block_registry: BlockRegistry,
	pub chunk_manager: M,
	directory: PathBuf,
}

impl<M: ChunkManager> JavaWorld<M> {
	pub fn open<P: AsRef<Path>>(directory: P) -> McResult<Self> {
		let directory = directory.as_ref().to_owned();
		if directory.is_dir() {
			Ok(Self {
				block_registry: BlockRegistry::new(),
				chunk_manager: M::create(directory.clone())?,
				directory,
			})
		} else {
			Err(McError::WorldDirectoryNotFound(directory))
		}
	}

	pub fn save(&mut self) -> McResult<()> {
		todo!()
	}
}

impl<M: ChunkManager> JavaWorld<M> {
	pub fn load_chunk(&mut self, coord: WorldCoord) -> McResult<()> {
		self.chunk_manager.load_chunk(&mut self.block_registry, coord)
	}

	pub fn save_chunk(&mut self, coord: WorldCoord) -> McResult<()> {
		self.chunk_manager.save_chunk(&mut self.block_registry, coord)
	}

	pub fn save_all(&self, block_registry: &BlockRegistry) -> McResult<()> {
		self.chunk_manager.save_all(block_registry)
	}

	pub fn unload_chunk(&mut self, coord: WorldCoord) -> McResult<()> {
		self.chunk_manager.unload_chunk(coord)
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