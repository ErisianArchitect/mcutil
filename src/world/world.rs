/*

*/
#![allow(unused)]

use std::{collections::HashMap, path::{PathBuf, Path}, marker::PhantomData, sync::{Arc, Mutex}, ops::Rem};

use crate::{McResult, McError, nbt::tag::NamedTag};
use super::container::*;

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
	block::CubeDirection,
};
use crate::math::coord::*;

#[inline(always)]
fn make_arcmutex<T>(value: T) -> Arc<Mutex<T>> {
	Arc::new(Mutex::new(value))
}

type ArcChunk = Arc<Mutex<Chunk>>;
type ArcRegion = Arc<Mutex<RegionFile>>;
type ArcVirtual = Arc<Mutex<VirtualChunk>>;

struct VirtualChunk {
	chunk: Chunk,
	dirty: bool,
}

pub struct CubeNeighbors {
	top: u32,	// +Y
	bottom: u32,// -Y
	east: u32,	// +X
	west: u32,	// -X
	south: u32,	// +Z
	north: u32,	// -Z
}

impl CubeNeighbors {
	pub fn new(
		top: u32,
		bottom: u32,
		north: u32,
		west: u32,
		south: u32,
		east: u32
	) -> Self {
		Self {
			top,
			bottom,
			north,
			west,
			south,
			east
		}
	}
}

/*
VirtualJavaWorld is for testing purposes. I plan on rewriting the entire
system after I get a better idea of what I'm working with.
*/
pub struct VirtualJavaWorld {
	pub block_registry: BlockRegistry,
	pub chunks: HashMap<WorldCoord, ArcChunk>,
	pub regions: HashMap<WorldCoord, ArcRegion>,
	pub directory: PathBuf,
}

impl VirtualJavaWorld {
	pub fn open(directory: impl AsRef<Path>) -> Self {
		Self {
			block_registry: BlockRegistry::with_air(),
			chunks: HashMap::new(),
			regions: HashMap::new(),
			directory: directory.as_ref().to_owned(),
		}
	}

	/// Get the directory that the region files are located at for each dimension.
	pub fn get_region_directory(&self, dimension: Dimension) -> PathBuf {
		self.directory.join(match dimension {
			Dimension::Overworld => "region",
			Dimension::Nether => "Dim-1/region",
			Dimension::TheEnd => "Dim1/region",
			Dimension::Other(_) => todo!(),
		})
	}

	/// Loads a region file into memory so that it IO can be performed.
	pub fn get_or_load_region(&mut self, coord: WorldCoord) -> McResult<ArcRegion> {
		if let Some(region) = self.regions.get(&coord) {
			Ok(region.clone())
		} else {
			let regiondir = self.get_region_directory(coord.dimension);
			let regname = format!("r.{}.{}.mca", coord.x, coord.z);
			let regfilepath = regiondir.join(regname);
			let regionfile = make_arcmutex(RegionFile::open_or_create(regfilepath)?);
			self.regions.insert(coord, regionfile.clone());
			Ok(regionfile)
		}
	}

	/// Loads a chunk into the world for editing.
	/// (This forces the loading of a chunk. If the chunk was already
	/// loaded, the old chunk will be discarded.)
	pub fn load_chunk(&mut self, coord: WorldCoord) -> McResult<ArcChunk> {
		let region = self.get_or_load_region(coord.region_coord())?;
		let regionlock = region.lock();
		if let Ok(mut regionfile) = regionlock {
			let root = regionfile.read_data::<_, NamedTag>(coord.xz())?;
			let chunk = make_arcmutex(decode_chunk(&mut self.block_registry, root.tag)?);
			self.chunks.insert(coord, chunk.clone());
			Ok(chunk)
		} else {
			McError::custom("Failed to lock region file.")
		}
	}

	/// Get a chunk if it's already been loaded or otherwise load the chunk.
	pub fn get_or_load_chunk(&mut self, coord: WorldCoord) -> McResult<ArcChunk> {
		if let Some(chunk) = self.get_chunk(coord) {
			Ok(chunk)
		} else {
			self.load_chunk(coord)
		}
	}

	/// Get a chunk (if it has been loaded).
	pub fn get_chunk(&self, coord: WorldCoord) -> Option<ArcChunk> {
		if let Some(chunk) = self.chunks.get(&coord) {
			Some(chunk.clone())
		} else {
			None
		}
	}

	/// Attempts to save a chunk (assuming the chunk has already been loaded)
	pub fn save_chunk(&mut self, coord: WorldCoord) -> McResult<()> {
		if let Some(chunk) = self.chunks.get(&coord) {
			let chunk = chunk.clone();
			let chunklock = chunk.lock();
			if let Ok(chunk) = chunklock {
				let nbt = chunk.to_nbt(&self.block_registry);
				let region = self.get_or_load_region(coord.region_coord())?;
				let regionlock = region.lock();
				if let Ok(mut regionfile) = regionlock {
					let root = NamedTag::new(nbt);
					regionfile.write_with_utcnow(coord.xz(), &root)?;
					return Ok(())
				}
			}
			McError::custom("Failed to write chunk to file.")
		} else {
			Ok(())
		}
	}

	/// Remove a chunk from internal storage.
	pub fn unload_chunk(&mut self, coord: WorldCoord) -> Option<ArcChunk> {
		self.chunks.remove(&coord)
	}

	/// Get a block id at the given coordinate.
	pub fn get_block_id(&self, coord: BlockCoord) -> Option<u32> {
		if let Some(chunk) = self.chunks.get(&coord.chunk_coord()) {
			if let Ok(chunk) = chunk.lock() {
				return chunk.get_block_id(coord.xyz());
			}
		}
		None
	}

	/// Get a block state at the given coordinate.
	pub fn get_block_state(&self, coord: BlockCoord) -> Option<&BlockState> {
		if let Some(id) = self.get_block_id(coord) {
			self.block_registry.get(id)
		} else {
			None
		}
	}

	/// Set a block id, returning the old block id.
	/// (This function does not check that the ids are the same)
	pub fn set_block_id(&mut self, coord: BlockCoord, id: u32) -> Option<u32> {
		if let Some(chunk) = self.chunks.get(&coord.chunk_coord()) {
			if let Ok(mut chunk) = chunk.lock() {
				return chunk.set_block_id(coord.xyz(), id);
			}
		}
		None
	}

	/// Set the block state at a coordinate. This will return the old block state.
	pub fn set_block_state(&mut self, coord: BlockCoord, state: &BlockState) -> Option<&BlockState> {
		let id = self.block_registry.register(state);
		self.set_block_id(coord, id).and_then(|id| {
			self.block_registry.get(id)
		})
	}

	pub fn query_neighbors(&self, coord: BlockCoord) -> CubeNeighbors {
		macro_rules! get_neighbor {
			($x:expr, $y:expr, $z:expr) => {
				self.get_block_id(BlockCoord::new(coord.x + ($x), coord.y + ($y), coord.z + ($z), coord.dimension)).unwrap_or_default()
			};
			// ($direction:expr) => {
			// 	self.get_block_id(coord.neighbor($direction)).unwrap_or_default()
			// };
		}
		CubeNeighbors {
			top: get_neighbor!(0, 1, 0),
			bottom: get_neighbor!(0, -1, 0),
			east: get_neighbor!(1, 0, 0),
			west: get_neighbor!(-1, 0, 0),
			south: get_neighbor!(0, 0, 1),
			north: get_neighbor!(0, 0, -1),
		}
	}
}

/*
Plan for new JavaWorld impl:
Choice between sparse and fixed bounds.
	Sparse: Chunks are loaded based on coordinates and any chunk can be loaded at any time.
	Fixed Bounds: Chunks within a fixed area are loaded into the world.
There are other things present in chunks, such as entities. I should do some research to determine
if I can/should integrate chunk entities with some sort of storage within the world itself, similar to how the palette
in subchunks is remapped.
I want to be able to use this library to inspect/edit the contents of a chest, for example.
*/

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