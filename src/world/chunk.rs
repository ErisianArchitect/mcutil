#![allow(unused)]

use std::collections::HashMap;
use std::default;
use std::ops::Not;

use super::blockstate::*;

use crate::McError;
use crate::McResult;
use crate::math::bit::BitLength;
use crate::nbt::*;
use crate::nbt::io::*;
use crate::nbt::tag::*;
use crate::nbt::tagtype::*;
use super::blockregistry::BlockRegistry;
use super::world::*;

/// This macro is used to remove an entry from a Map (usually HashMap or IndexMap)
/// the item that is removed from the map is then decoded from the NBT
/// into the requested type.
/// ```rust,no_run
/// let map: Map;
/// let value: Byte = map_decoder!(map; "some tag" -> Byte);
/// // In case the value might not exist.
/// let option: Option<Byte> = map_decoder!(map; "some tag" -> Option<Byte>);
/// ```
macro_rules! map_decoder {
	($map:expr; $name:literal) => {
		$map.remove($name).ok_or(McError::NotFoundInCompound($name.to_owned()))?
	};
	($map:expr; $name:literal -> Option<$type:ty>) => {
		if let Some(tag) = $map.remove($name) {
			Some(<$type>::decode_nbt(tag)?)
		} else {
			None
		}
	};
	($map:expr; $name:literal -> $type:ty) => {
		<$type>::decode_nbt($map.remove($name).ok_or(McError::NotFoundInCompound($name.to_owned()))?)?
	};
}

macro_rules! map_encoder {
	($map:expr; $name:literal = $value:expr) => {
		($map).insert($name.to_owned(), $value.encode_nbt());
	};
	($map:expr; $($name:literal = $value:expr;)+) => {
		$(
			map_encoder!($map; $name = $value);
		)+
	};
}

pub struct BlockStates {
	palette: Vec<BlockState>,
	data: Vec<u32>,
}

pub struct LightData {
	data: Box<[i8; 2048]>
}

pub struct Biomes {
	palette: Vec<String>
}

pub struct TileTick {
	block_id: String,
	precedence: i32,
	ticks: i32,
	x: i32,
	y: i32,
	z: i32,
}

#[derive(Clone)]
pub struct Heightmaps {
	motion_blocking: Vec<i64>,
	motion_blocking_no_leaves: Vec<i64>,
	ocean_floor: Vec<i64>,
	ocean_floor_wg: Option<Vec<i64>>,
	world_surface: Vec<i64>,
	world_surface_wg: Option<Vec<i64>>,
}

impl DecodeNbt for Heightmaps {
	type Error = McError;

	fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
		if let Tag::Compound(mut map) = nbt {
			Ok(Heightmaps {
				motion_blocking: map_decoder!(map; "MOTION_BLOCKING" -> Vec<i64>),
				motion_blocking_no_leaves: map_decoder!(map; "MOTION_BLOCKING_NO_LEAVES" -> Vec<i64>),
				ocean_floor: map_decoder!(map; "OCEAN_FLOOR" -> Vec<i64>),
				ocean_floor_wg: map_decoder!(map; "OCEAN_FLOOR_WG" -> Option<Vec<i64>>),
				world_surface: map_decoder!(map; "WORLD_SURFACE" -> Vec<i64>),
				world_surface_wg: map_decoder!(map; "WORLD_SURFACE_WG" -> Option<Vec<i64>>),
			})
		} else {
			Err(McError::NbtDecodeError)
		}
	}
}

impl EncodeNbt for Heightmaps {
	fn encode_nbt(self) -> Tag {
		let mut map = Map::new();
		map_encoder!(map;
			"MOTION_BLOCKING" = self.motion_blocking;
			"MOTION_BLOCKING_NO_LEAVES" = self.motion_blocking_no_leaves;
			"OCEAN_FLOOR" = self.ocean_floor;
			// "OCEAN_FLOOR_WG" = self.ocean_floor_wg;
			"WORLD_SURFACE" = self.world_surface;
			// "WORLD_SURFACE_WG" = self.world_surface_wg;
		);
		if let Some(ofwg) = self.ocean_floor_wg {
			map_encoder!(map; "OCEAN_FLOOR_WG" = ofwg);
		}
		if let Some(wswg) = self.world_surface_wg {
			map_encoder!(map; "WORLD_SURFACE_WG" = wswg);
		}
		Tag::Compound(map)
	}
}

#[derive(Clone)]
pub struct Chunk {
	/// DataVersion
	pub data_version: i32,
	/// xPos
	pub x: i32,
	/// yPos
	pub y: i32,
	/// zPos
	pub z: i32,
	/// LastUpdate
	pub last_update: i64,
	/// Status
	pub status: String,
	/// sections
	pub sections: ChunkSections,
	/// block_entities
	pub block_entities: Vec<BlockEntity>,
	/// CarvingMasks
	pub carving_masks: Option<CarvingMasks>,
	/// HeightMaps
	pub heightmaps: Heightmaps,
	/// fluid_ticks
	pub fluid_ticks: ListTag,
	/// block_ticks
	pub block_ticks: ListTag,
	/// InhabitedTime
	pub inhabited_time: i64,
	/// PostProcessing
	pub post_processing: ListTag,
	/// structures
	pub structures: Map,
	/// Lights
	pub lights: Option<ListTag>,
	/// Entities
	pub entities: Option<ListTag>,
	pub other: Map,
}

#[inline(always)]
fn chunk_local_coord(coord: (i64, i64, i64)) -> (i64, i64, i64) {
	(
		coord.0 & 0xf,
		coord.1 & 0xf,
		coord.2 & 0xf,
	)
}

#[inline(always)]
const fn chunk_section_index(coord_y: i64, chunk_y: i64) -> usize {
	let section_index = coord_y.div_euclid(16);
	let adj_index = section_index - chunk_y;
	adj_index as usize
}

impl Chunk {
	pub fn get_block_id(&self, coord: (i64, i64, i64)) -> Option<u32> {
		let lowy = self.sections.sections[0].y;
		let section_index = chunk_section_index(coord.1, lowy as i64);
		let (x, y, z) = chunk_local_coord(coord);
		self.sections.sections[section_index].get_block_id(x, y, z)
	}

	pub fn set_block_id(&mut self, coord: (i64, i64, i64), id: u32) -> Option<u32> {
		let lowy = self.sections.sections[0].y;
		let section_index = chunk_section_index(coord.1, lowy as i64);
		let (x, y, z) = chunk_local_coord(coord);
		self.sections.sections[section_index].set_block_id(x, y, z, id)
	}

	pub fn to_nbt(&self, block_registry: &BlockRegistry) -> Tag {
		Tag::Compound(encode_chunk(block_registry, self))
	}
}

#[derive(Clone)]
pub struct ChunkSection {
	pub y: i8,
	pub blocks: Option<Box<[u32]>>,
	pub biomes: Option<Map>,
	pub skylight: Option<Vec<i8>>,
	pub blocklight: Option<Vec<i8>>,
}

#[inline(always)]
fn chunk_yzx_index(x: i64, y: i64, z: i64) -> usize {
	let local_x = x & 0xf;
	let local_y = y & 0xf;
	let local_z = z & 0xf;
	(local_y*256 + local_z*16 + local_x) as usize
}

impl ChunkSection {
	pub fn get_block_id(&self, local_x: i64, local_y: i64, local_z: i64) -> Option<u32> {
		if let Some(blocks) = &self.blocks {
			let index = chunk_yzx_index(local_x, local_y, local_z);
			Some(blocks[index])
		} else {
			None
		}
	}

	pub fn set_block_id(&mut self, local_x: i64, local_y: i64, local_z: i64, id: u32) -> Option<u32> {
		if id != 0 && self.blocks.is_none() {
			self.blocks = Some(Box::new([0u32; 4096]));
		}
		if let Some(blocks) = &mut self.blocks {
			let index = chunk_yzx_index(local_x, local_y, local_z);
			let result = blocks[index];
			blocks[index] = id;
			Some(result)
		} else {
			None
		}
	}
}

#[derive(Clone)]
pub struct ChunkSections {
	pub sections: Vec<ChunkSection>,
}

#[derive(Clone)]
pub struct CarvingMasks {
	air: Vec<i8>,
	liquid: Vec<i8>,
}

impl DecodeNbt for CarvingMasks {
	type Error = McError;

	fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
		if let Tag::Compound(mut map) = nbt {
			let air = map_decoder!(map; "AIR" -> Vec<i8>);
			let liquid = map_decoder!(map; "LIQUID" -> Vec<i8>);
			Ok(CarvingMasks {
				air,
				liquid,
			})
		} else {
			Err(McError::NbtDecodeError)
		}
	}
}

impl EncodeNbt for CarvingMasks {
	fn encode_nbt(self) -> Tag {
		let mut map = Map::new();
		map_encoder!(map;
			"AIR" = self.air;
			"LIQUID" = self.liquid;
		);
		Tag::Compound(map)
	}
}

#[derive(Clone)]
pub struct BlockEntity {
	id: String,
	keep_packed: i8,
	x: i32,
	y: i32,
	z: i32,
	data: Map,
}

impl DecodeNbt for Vec<BlockEntity> {
	type Error = McError;
	fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
		if let Tag::List(list) = nbt {
			if let ListTag::Compound(entities) = list {
				entities.into_iter().map(|mut entity| {
					Ok(BlockEntity {
						id: map_decoder!(entity; "id" -> String),
						keep_packed: map_decoder!(entity; "keepPacked" -> i8),
						x: map_decoder!(entity; "x" -> i32),
						y: map_decoder!(entity; "y" -> i32),
						z: map_decoder!(entity; "z" -> i32),
						data: entity,
					})
				}).collect::<Result<Vec<BlockEntity>, McError>>()
			} else {
				Ok(Vec::new())
			}
		} else {
			Err(McError::NbtDecodeError)
		}
	}
}

impl EncodeNbt for Vec<BlockEntity> {
	fn encode_nbt(self) -> Tag {
		let entities = self.into_iter().map(|entity| {
			let mut map = Map::new();
			map_encoder!(map;
				"id" = entity.id;
				"keepPacked" = entity.keep_packed;
				"x" = entity.x;
				"y" = entity.y;
				"z" = entity.z;
			);
			map.extend(entity.data);
			map
		}).collect::<Vec<Map>>();
		Tag::List(ListTag::Compound(entities))
	}
}

/*
def extract_index(full_index, palette_size, block_states):
	bitsize = max((palette_size - 1).bit_length(), 4)
	#vpl = values per long
	vpl = 64 // bitsize
	mask = 2**bitsize-1
	state_index = full_index // vpl
	value_offset = (full_index % vpl) * bitsize
	slot = int(block_states[state_index])
	return (slot & (mask << value_offset)) >> value_offset

	states_tag = section_tag['BlockStates']
	palette = section_tag['Palette']

	
if palette is not None and states_tag is not None:
	states = list()
	blocks = numpy.ndarray(shape=(4096,),dtype=numpy.object_)
	for v in palette.data:
		name = v.Name.value
		props = {}
		if 'Properties' in v:
			props = { k : val.value for k, val in v.Properties.data.items() }
		states.append(blockregistry.register(name, props))
	
	for i in range(4096):
		ind = extract_index(i, len(palette.data), states_tag.data)
		blocks[i] = states[ind].unique_key
*/

pub fn decode_palette(palette: ListTag) -> Result<Vec<BlockState>, McError> {
	if let ListTag::Compound(mut states) = palette {
		states.into_iter().map(|mut state| {
			let name = map_decoder!(state; "Name" -> String);
			// The "Properties" tag may not exist.
			let properties = if state.contains_key("Properties") {
				let props = map_decoder!(state; "Properties" -> Map);
				BlockProperties::from(props.into_iter().map(|(name, value)| {
					if let Tag::String(value) = value {
						Ok((name, value))
					} else {
						Err(McError::NbtDecodeError)
					}
				}).collect::<Result<Vec<(String, String)>, McError>>()?)
			} else {
				BlockProperties::none()
			};
			Ok(BlockState::new(name, properties))
		}).collect::<Result<Vec<BlockState>, McError>>()
	} else {
		Err(McError::NbtDecodeError)
	}
}

pub fn extract_palette_index(index: usize, palette_size: usize, states: &[i64]) -> usize {
	// Subtract 1 because it's the bit length of the largest possible index
	// If the palette size is 16, the bit length to represent
	// 16 is 5, but the bit length to represent the largest index (15)
	// is only 4.
	let bitsize = (palette_size - 1).bit_length().max(4);
	let vpl = (64 / bitsize) as u64;
	let mask = 2u64.pow(bitsize) - 1;
	let state_index = index as u64 / vpl;
	let value_offset = ((index as u64).rem_euclid(vpl)) as u32 * bitsize;
	let slot = states[state_index as usize] as u64;
	((slot & (mask << value_offset)) >> value_offset) as usize
}

pub fn decode_section(block_registry: &mut BlockRegistry, mut section: Map) -> Result<ChunkSection, McError> {
	let y = map_decoder!(section; "Y" -> Byte);
	// The following three may or may not exist.
	let biomes = map_decoder!(section; "biomes" -> Option<Map>);
	let blocklight = map_decoder!(section; "BlockLight" -> Option<ByteArray>);
	let skylight = map_decoder!(section; "SkyLight" -> Option<ByteArray>);

	let mut block_states = map_decoder!(section; "block_states" -> Option<Map>);

	let blocks = if let Some(mut block_states) = block_states {
		// Now I need to transform the block_data and palette into registry IDs.
		// The ending block_data should contain 4096 u32 values representing their IDs in
		// the registry. So I need to register each BlockState in the palette with the
		// registry, retrieving the ID. I think the appropriate way to do this would be
		// to do an iterator map to the block_registry IDs.
		let palette = decode_palette(map_decoder!(block_states; "palette" -> ListTag))?;
		// Register blocks.
		let palette = palette.iter()
			.map(|state| {
				block_registry.register(state)
			}).collect::<Vec<u32>>();
		if block_states.contains_key("data") {
			// Extract indices from packed values.
			let data = map_decoder!(block_states; "data" -> LongArray);
			let data = (0..4096).into_iter().map(|full_index| {
				let index = extract_palette_index(full_index, palette.len(), data.as_slice());
				palette[index]
			}).collect::<Vec<u32>>();
			Some(data.into_boxed_slice())
		} else {
			None
		}
	} else {
		None
	};
	Ok(ChunkSection {
		y,
		biomes,
		blocklight,
		skylight,
		blocks,
	})
	
	// todo!()
}

pub fn decode_chunk(block_registry: &mut BlockRegistry, nbt: Tag) -> McResult<Chunk> {
	if let Tag::Compound(mut map) = nbt {
		let sections = if let ListTag::Compound(sections) = map_decoder!(map; "sections" -> ListTag) {
			sections.into_iter()
				.map(|section| decode_section(block_registry, section))
				.collect::<McResult<Vec<ChunkSection>>>()?
		} else {
			return Err(McError::NbtDecodeError);
		};
		let sections = ChunkSections {
			sections,
		};
		Ok(Chunk {
			sections,
			data_version: map_decoder!(map; "DataVersion" -> i32),
			x: map_decoder!(map; "xPos" -> i32),
			y: map_decoder!(map; "yPos" -> i32),
			z: map_decoder!(map; "zPos" -> i32),
			last_update: map_decoder!(map; "LastUpdate" -> i64),
			block_entities: map_decoder!(map; "block_entities" -> Vec<BlockEntity>),
			heightmaps: map_decoder!(map; "Heightmaps" -> Heightmaps),
			fluid_ticks: map_decoder!(map; "fluid_ticks" -> ListTag),
			block_ticks: map_decoder!(map; "block_ticks" -> ListTag),
			post_processing: map_decoder!(map; "PostProcessing" -> ListTag),
			structures: map_decoder!(map; "structures" -> Map),
			inhabited_time: map_decoder!(map; "InhabitedTime" -> i64),
			status: map_decoder!(map; "Status" -> String),
			carving_masks: map_decoder!(map; "CarvingMasks" -> Option<CarvingMasks>),
			lights: map_decoder!(map; "Lights" -> Option<ListTag>),
			entities: map_decoder!(map; "Entities" -> Option<ListTag>),
			other: map,
		})
	} else {
		Err(McError::NbtDecodeError)
	}
}

// def inject_index(full_index, palette_size, block_states, value):
//     bitsize = max((palette_size - 1).bit_length(), 4)
//     #vpl = values per long
//     vpl = 64 // bitsize
//     mask = 2**bitsize-1
//     masked_value = value & mask
//     #state_index is the index in the array of longs that our value will be injected to.
//     state_index = full_index // vpl
//     #value_offset represents the number of bits to shift to form our mask for setting the value.
//     value_offset = (full_index % vpl) * bitsize
//     #block_state will be a 64 bit integer
//     block_state = block_states[state_index]
//     #Injecting our value to the block_state
//     block_states[state_index] = (block_state & ~(mask << value_offset)) | (value << value_offset)
fn inject_palette_index(full_index: usize, palette_size: usize, states: &mut [i64], value: u32) {
	let bitsize = (palette_size - 1).bit_length().max(4);
	let vpl = (64 / bitsize) as u64;
	let mask = 2u64.pow(bitsize) - 1;
	let state_index = full_index as u64 / vpl;
	let value_offset = ((full_index as u64).rem_euclid(vpl) as u32) * bitsize;
	let state = states[state_index as usize] as u64;
	let new_value = (state & (mask << value_offset).not()) | ((value as u64) << value_offset);
	states[state_index as usize] = new_value as i64;
}

fn create_block_states(block_registry: &BlockRegistry, blocks: &Option<Box<[u32]>>) -> Map {
	if let Some(blocks) = blocks {
		// Collect unique block-ids
		let mut local_registry = HashMap::<u32, u32>::new();
		let mut palette = Vec::<BlockState>::new();
		let local_ids = blocks.iter().map(|block_id| {
			if let Some(local_id) = local_registry.get(block_id) {
				*local_id
			} else {
				if let Some(state) = block_registry.get(*block_id) {
					let id = palette.len() as u32;
					local_registry.insert(*block_id, id);
					palette.push(state.clone());
					id
				} else {
					0
				}
			}
		}).collect::<Vec<u32>>();
		// Pack 4096 block ids into array of i64.
		// The buffer size for the long_array is calculated based on
		// palette size.
		let bitsize = (palette.len() - 1).bit_length().max(4);
		let vpl = (64 / bitsize) as u64;
		let buffer_size = 4096/vpl + if 4096u64.rem_euclid(vpl) != 0 { 1 } else { 0 };
		let mut data = vec![0i64; buffer_size as usize];
		local_ids.into_iter().enumerate().for_each(|(i, id)| {
			inject_palette_index(i, palette.len(), &mut data, id);
		});
		// Build palette
		let palette = palette.into_iter().map(|state| {
			state.to_map()
		}).collect::<Vec<Map>>();
		let palette = Tag::List(ListTag::Compound(palette));
		let data = Tag::LongArray(data);
		Map::from([
			("palette".to_owned(), palette),
			("data".to_owned(), data),
		])
	} else {
		let mut palette = Map::new();
		palette.insert("Name".to_owned(), Tag::string("minecraft:air"));
		let palette = ListTag::Compound(vec![palette]);
		Map::from([
			("palette".to_owned(), Tag::List(palette)),
		])
	}
}

fn encode_section(block_registry: &BlockRegistry, section: &ChunkSection) -> Map {
	// In order to encode a ChunkSection into a HashMap<String, Tag>
	// I will need to create a block state palette from the blocks
	// in the section.
	let mut map = Map::new();
	map_encoder!(map; "Y" = section.y);
	if let Some(biomes) = &section.biomes {
		let biomes = biomes.clone();
		map_encoder!(map; "biomes" = biomes);
	}
	if let Some(blocklight) = &section.blocklight {
		let blocklight = blocklight.clone();
		map_encoder!(map; "BlockLight" = blocklight);
	}
	if let Some(skylight) = &section.skylight {
		let skylight = skylight.clone();
		map_encoder!(map; "SkyLight" = skylight);
	}
	let block_states = create_block_states(block_registry, &section.blocks);
	map_encoder!(map; "block_states" = block_states);
	map
}

pub fn encode_chunk(block_registry: &BlockRegistry, chunk: &Chunk) -> Map {
	let mut map = Map::new();
	let data_version = chunk.data_version;
	let x = chunk.x;
	let y = chunk.y;
	let z = chunk.z;
	let last_update = chunk.last_update;
	let inhabited_time = chunk.inhabited_time;
	let status = chunk.status.clone(); 
	let block_entities = chunk.block_entities.clone();
	let carving_masks = chunk.carving_masks.clone();
	let heightmaps = chunk.heightmaps.clone();
	let fluid_ticks = chunk.fluid_ticks.clone();
	let block_ticks = chunk.block_ticks.clone();
	let post_processing = chunk.post_processing.clone();
	let structures = chunk.structures.clone();
	map_encoder!(map;
		"DataVersion" = data_version;
		"xPos" = x;
		"yPos" = y;
		"zPos" = z;
		"LastUpdate" = last_update;
		"InhabitedTime" = inhabited_time;
		"Status" = status;
		"block_entities" = block_entities;
		"Heightmaps" = heightmaps;
		"fluid_ticks" = fluid_ticks;
		"block_ticks" = block_ticks;
		"PostProcessing" = post_processing;
		"structures" = structures;
	);
	if let Some(carvingmasks) = carving_masks {
		map_encoder!(map; "CarvingMasks" = carvingmasks);
	}
	if let Some(lights) = &chunk.lights {
		let lights = lights.clone();
		map_encoder!(map; "Lights" = lights);
	}
	if let Some(entities) = &chunk.entities {
		let entities = entities.clone();
		map_encoder!(map; "Entities" = entities);
	}
	let sections = ListTag::Compound(chunk.sections.sections.iter().map(|section| {
		encode_section(block_registry, section)
	}).collect::<Vec<Map>>());
	map_encoder!(map; "sections" = sections);
	if !chunk.other.is_empty() {
		map.extend(chunk.other.clone());
	}
	map
}

/*
TODO: 	Make it so that chunks can be loaded directly from memory.
		This would involve more complicated programming, but it would
		give faster load times. I also need to make it so that there
		is a World block registry to register blocks to.
*/