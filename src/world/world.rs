/*

*/
#![allow(unused)]

use std::{collections::HashMap, path::{PathBuf, Path}, marker::PhantomData, sync::{Arc, Mutex}, ops::Rem, borrow::Borrow};

use glam::I64Vec3;

use crate::{McResult, McError, nbt::tag::NamedTag, math::bounds::{Bounds2, Bounds3}};
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

pub struct CubeNeighbors<T> {
	/// +Y
	top: T,
	/// -Y
	bottom: T,
	/// +X
	east: T,
	/// -X
	west: T,
	/// +Z
	south: T,
	/// -Z
	north: T,
}

impl<T> CubeNeighbors<T> {
	pub fn new(
		top: T,
		bottom: T,
		north: T,
		west: T,
		south: T,
		east: T
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

// I was thinking that I could make it so that for get_block/set_block function
// I could make it so that the world automatically loads/creates chunks that are
// not yet loaded.
// pub enum MissingAction {
// 	None,
// 	Load,
// 	LoadOrCreate,
// }

pub struct RegionSlot {
	region: RegionFile,
	load_count: usize,
}

impl RegionSlot {
	pub fn new(region: RegionFile) -> Self {
		Self {
			region,
			load_count: 0,
		}
	}

	pub fn arc_new(region: RegionFile) -> ArcRegionSlot {
		make_arcmutex(Self::new(region))
	}

	pub fn increment(&mut self) {
		self.load_count += 1;
	}

	/// Returns `true` when count gets to 0.
	pub fn decrement(&mut self) -> bool {
		self.load_count = self.load_count.checked_sub(1).unwrap_or_default();
		self.load_count == 0
	}
}

pub struct ChunkSlot {
	pub chunk: Chunk,
	/// Determines if the chunk has been altered since last saved.
	pub dirty: bool,
}

impl ChunkSlot {
	pub fn new(chunk: Chunk) -> Self {
		Self {
			chunk,
			dirty: false,
		}
	}

	pub fn arc_new(chunk: Chunk) -> ArcChunkSlot {
		make_arcmutex(Self::new(chunk))
	}

	#[inline(always)]
	pub fn mark_dirty(&mut self) {
		self.dirty = true;
	}
}

type ArcChunkSlot = Arc<Mutex<ChunkSlot>>;
type ArcRegionSlot = Arc<Mutex<RegionSlot>>;

/*
VirtualJavaWorld is for testing purposes. I plan on rewriting the entire
system after I get a better idea of what I'm working with.
*/
pub struct VirtualJavaWorld {
	pub block_registry: BlockRegistry,
	pub chunks: HashMap<WorldCoord, ArcChunkSlot>,
	pub regions: HashMap<WorldCoord, ArcRegionSlot>,
	pub directory: PathBuf,
}

// I would like to implement a system where I keep track of
// how many chunks are loaded per region so that I can unload
// a region when there are no more chunks.

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
	pub fn get_or_load_region(&mut self, coord: WorldCoord) -> McResult<ArcRegionSlot> {
		if let Some(slot) = self.regions.get(&coord) {
			Ok(slot.clone())
		} else {
			let regiondir = self.get_region_directory(coord.dimension);
			let regname = format!("r.{}.{}.mca", coord.x, coord.z);
			let regfilepath = regiondir.join(regname);
			let regionfile = RegionFile::open_or_create(regfilepath)?;
			let slot = RegionSlot::arc_new(regionfile);
			self.regions.insert(coord, slot.clone());
			Ok(slot)
		}
	}

	/// Loads a chunk into the world for editing.
	/// (This forces the loading of a chunk. If the chunk was already
	/// loaded, the old chunk will be discarded.)
	pub fn load_chunk(&mut self, coord: WorldCoord) -> McResult<ArcChunkSlot> {
		let region = self.get_or_load_region(coord.region_coord())?;
		let reglock = region.lock();
		if let Ok(mut regionlock) = reglock {
			let root = regionlock.region.read_data::<_, NamedTag>(coord.xz())?;
			let chunk = decode_chunk(&mut self.block_registry, root.tag)?;
			let slot = ChunkSlot::arc_new(chunk);
			let old = self.chunks.insert(coord, slot.clone());
			// If there was already a chunk loaded at this coord, there's no need
			// for incrementation.
			if old.is_none() {
				regionlock.increment();
			}
			Ok(slot)
		} else {
			McError::custom("Failed to lock region file.")
		}
	}

	// TODO: 	I want to transform this function so that it can load
	//			from (center, radius)
	pub fn load_area<T: Into<Bounds2>>(&mut self, dimension: Dimension, bounds: T) -> McResult<()> {
		let bounds: Bounds2 = bounds.into();
		(bounds.min.y..bounds.max.y).try_for_each(|y| {
			(bounds.min.x..bounds.max.x).try_for_each(|x| {
				self.load_chunk(WorldCoord::new(x, y, dimension))?;
				McResult::Ok(())
			})
		})
	}

	/// Get a chunk if it's already been loaded or otherwise load the chunk.
	pub fn get_or_load_chunk(&mut self, coord: WorldCoord) -> McResult<ArcChunkSlot> {
		if let Some(slot) = self.get_chunk(coord) {
			Ok(slot)
		} else {
			self.load_chunk(coord)
		}
	}

	/// Get a chunk (if it has been loaded).
	pub fn get_chunk(&self, coord: WorldCoord) -> Option<ArcChunkSlot> {
		self.chunks.get(&coord).map(|slot| slot.clone())
	}

	/// Attempts to save a chunk (assuming the chunk has already been loaded)
	pub fn save_chunk(&mut self, coord: WorldCoord) -> McResult<()> {
		if let Some(slot) = self.get_chunk(coord) {
			if let Ok(mut slot) = slot.lock() {
				if !slot.dirty {
					return Ok(());
				}
				let region = self.get_or_load_region(coord.region_coord())?;
				let reglock = region.lock();
				if let Ok(mut region) = reglock {
					let nbt = slot.chunk.to_nbt(&self.block_registry);
					let root = NamedTag::new(nbt);
					region.region.write_with_utcnow(coord.xz(), &root)?;
					slot.dirty = false;
					return Ok(());
				}
			}
			return Err(McError::FailedToSaveChunk)
		}
		Ok(())
	}

	pub fn save_area<T: Into<Bounds2>>(&mut self, dimension: Dimension, bounds: T) -> McResult<()> {
		let bounds: Bounds2 = bounds.into();
		(bounds.min.y..bounds.max.y).try_for_each(|y| {
			(bounds.min.x..bounds.max.x).try_for_each(|x| {
				self.save_chunk(WorldCoord::new(x, y, dimension))?;
				McResult::Ok(())
			})
		})
	}

	pub fn save_all(&mut self) -> McResult<()> {
		let keys_clone = self.chunks.keys().map(|c| *c).collect::<Box<[WorldCoord]>>();
		keys_clone.into_iter().try_for_each(|coord| {
			self.save_chunk(*coord)
		})
	}

	/// Remove a chunk from internal storage.
	pub fn unload_chunk(&mut self, coord: WorldCoord) -> Option<ArcChunkSlot> {
		if self.chunks.contains_key(&coord) {
			let removed = self.chunks.remove(&coord);
			let mut unload_region: bool = false;
			{
				let region = self.regions.get(&coord.region_coord());
				if let Some(region) = region {
					let reglock = region.lock();
					if let Ok(mut region) = reglock {
						unload_region = region.decrement();
					}
				}
			}
			if unload_region {
				self.regions.remove(&coord.region_coord());
			}
			removed
		} else {
			None
		}
	}

	pub fn unload_area<T: Into<Bounds2>>(&mut self, dimension: Dimension, bounds: T) {
		let bounds: Bounds2 = bounds.into();
		(bounds.min.y..bounds.max.y).for_each(|y| {
			(bounds.min.x..bounds.max.x).for_each(|x| {
				self.unload_chunk(WorldCoord::new(x, y, dimension));
			})
		})
	}

	/// Unloads all loaded chunks and all loaded region files.
	pub fn unload_all(&mut self) {
		self.chunks.clear();
		self.regions.clear();
	}

	/// Get a block id at the given coordinate.
	pub fn get_id(&self, coord: BlockCoord) -> Option<u32> {
		if let Some(slot) = self.get_chunk(coord.chunk_coord()) {
			if let Ok(slot) = slot.lock() {
				return slot.chunk.get_id(coord.xyz());
			}
		}
		None
	}

	/// Get a block state at the given coordinate.
	pub fn get_state(&self, coord: BlockCoord) -> Option<&BlockState> {
		if let Some(id) = self.get_id(coord) {
			self.block_registry.get(id)
		} else {
			None
		}
	}

	/// Set a block id, returning the old block id.
	/// (This function does not check that the ids are the same)
	pub fn set_id(&mut self, coord: BlockCoord, id: u32) -> Option<u32> {
		if let Some(slot) = self.get_chunk(coord.chunk_coord()) {
			if let Ok(mut slot) = slot.lock() {
				let old_id = slot.chunk.set_id(coord.xyz(), id);
				if let Some(old_id) = old_id {
					if old_id != id {
						slot.mark_dirty();
					}
				} else {
					slot.mark_dirty();
				}
				return old_id
			}
		}
		None
	}

	/// Set the block state at a coordinate. This will return the old block state.
	pub fn set_state<T: Borrow<BlockState>>(&mut self, coord: BlockCoord, state: T) -> Option<&BlockState> {
		let id = self.block_registry.register(state.borrow());
		self.set_id(coord, id).and_then(|id| {
			self.block_registry.get(id)
		})
	}

	pub fn query_neighbor_ids(&self, coord: BlockCoord) -> CubeNeighbors<u32> {
		macro_rules! get_neighbor {
			($x:expr, $y:expr, $z:expr) => {
				self.get_id(BlockCoord::new(coord.x + ($x), coord.y + ($y), coord.z + ($z), coord.dimension)).unwrap_or_default()
			};
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

	pub fn query_neighbor_states(&self, coord: BlockCoord) -> CubeNeighbors<Option<&BlockState>> {
		macro_rules! get_neighbor {
			($x:expr, $y:expr, $z:expr) => {
				self.get_state(BlockCoord::new(coord.x + ($x), coord.y + ($y), coord.z + ($z), coord.dimension))
			};
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

	pub fn is_chunk_loaded(&self, coord: WorldCoord) -> bool {
		self.chunks.contains_key(&coord)
	}

	pub fn copy_blocks(&self, dimension: Dimension, bounds: Bounds3) -> BlockContainer {
		let size = bounds.size::<I64Vec3>();
		todo!()
	}

	pub fn copy_blocks_into(&self, origin: BlockCoord, container: &mut BlockContainer) {
		todo!()
	}

	pub fn fill_area_id(&mut self, dimension: Dimension, bounds: Bounds3, id: u32) {
		bounds.for_each(|coord| {
			let (x,y,z): (i64, i64, i64) = coord.into();
			self.set_id(dimension.blockcoord(x, y, z), id);
		});
	}

	pub fn fill_area_state<T: Borrow<BlockState>>(&mut self, dimension: Dimension, bounds: Bounds3, state: T) {
		let id = self.block_registry.register(state);
		self.fill_area_id(dimension, bounds, id);
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

/*
One of the issues with my current implementation of the virtual world is that you have to manually load
the chunks that you want to edit. Ideally, it should be possible to edit a world without ever having to
worry about calling functions to manually load chunks. This is actually pretty easy to achieve. All I
have to do is set a limit to the number of chunks that can be loaded at the same time. Then every time
the user wants to get/set a block, the chunk is automatically loaded if it hasn't been loaded already.
The chunk is then stored in a queue. The once the queue reaches a certain size, the oldest element is
pulled out of the queue and it is saved and unloaded. Any time a chunk that is already in the queue is
edited, it goes to the back of the queue.
*/