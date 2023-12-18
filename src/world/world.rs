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

	/// Converts a chunk coordinate into a region coordinate.
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

	pub fn chunk_coord(self) -> WorldCoord {
		WorldCoord {
			x: self.x / 16,
			z: self.z / 16,
			dimension: self.dimension,
		}
	}

	pub fn region_coord(self) -> WorldCoord {
		WorldCoord {
			x: self.x / 512,
			z: self.z / 512,
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

	fn get_block_id(&self, block_registry: &BlockRegistry, coord: BlockCoord) -> McResult<Option<u32>>;
	fn get_block_state(&self, block_registry: &BlockRegistry, coord: BlockCoord) -> McResult<Option<BlockState>>;
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

impl JavaChunkManager {
	fn load_region(&mut self, coord: WorldCoord) -> McResult<Arc<Mutex<RegionFile>>> {
		if !self.regions.contains_key(&coord) {
			let region_dir = self.directory.join(match coord.dimension {
				Dimension::Overworld => "region",
				Dimension::Nether => todo!(),
				Dimension::Other(_) => todo!(),
			});
			let file_path = format!("r.{}.{}.mca", coord.x, coord.z);
			let file_path = region_dir.join(file_path);
			let region_file = if file_path.is_file() {
				make_arcmutex(RegionFile::open(file_path)?)
			} else {
				// If the file doesn't exist, we'll create a region file.
				make_arcmutex(RegionFile::create(file_path)?)
			};
			self.regions.insert(coord, region_file.clone());
			Ok(region_file)
		} else {
			Ok(self.regions.get(&coord).unwrap().clone())
		}
	}
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
		let region_coord = coord.region_coord();
		let (x, z) = (coord.x.rem_euclid(32), coord.z.rem_euclid(32));
		let region_file = self.load_region(region_coord)?;
		if let Ok(mut region) = region_file.lock() {
			let chunk_tag: NamedTag = region.read_data(RegionCoord::from((x, z)))?;
			let chunk = decode_chunk(block_registry, chunk_tag.tag)?;
			self.chunks.insert(coord, make_arcmutex(chunk));
		}
		Ok(())
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

	fn get_block_id(&self, block_registry: &BlockRegistry, coord: BlockCoord) -> McResult<Option<u32>> {
		let chunk_coord = coord.chunk_coord();
		if let Some(chunk) = self.chunks.get(&chunk_coord) {
			if let Ok(chunk) = chunk.lock() {
				Ok(chunk.get_block_id(coord.xyz()))
			} else {
				Ok(None)
			}
		} else {
			Ok(None)
		}
	}

	fn get_block_state(&self, block_registry: &BlockRegistry, coord: BlockCoord) -> McResult<Option<BlockState>> {
		todo!()
	}

	fn set_block_id(&self, block_registry: &mut BlockRegistry, coord: BlockCoord, id: u32) -> McResult<()> {
		todo!()
	}

	fn set_block_state(&self, block_registry: &mut BlockRegistry, coord: BlockCoord, state: BlockState) -> McResult<()> {
		todo!()
	}

	
}

#[test]
fn qtest() {
	let mut map = HashMap::new();
	map.insert(1, "Test".to_owned());
	if let Some(item) = map.get(&2) {
		println!("{}", item);
	} else {
		println!("Not found.");
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