#![allow(unused)]

use std::collections::HashMap;
use std::default;

use crate::nbt::*;
use crate::nbt::io::*;
use crate::nbt::tag::*;

use crate::world::io::region_old::*;

pub struct BlockStateProperty {
	name: String,
	value: String,
}

pub enum BlockStateProperties {
	None,
	Single {
		name: String,
		value: String,
	},
	Many(HashMap<String, String>),
}

pub struct BlockState {
	name: String,
	properties: BlockStateProperties,
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

pub struct HeightMaps {
	motion_blocking: Vec<i64>,
	motion_blocking_no_leaves: Vec<i64>,
	ocean_floor: Vec<i64>,
	ocean_floor_wg: Vec<i64>,
	world_surface: Vec<i64>,
	world_surface_wg: Vec<i64>,
}

pub struct Chunk {
	data_version: i32,
	x: i32,
	y: i32,
	z: i32,
	last_update: i64,
	inhabited_time: i64,
	sections: Vec<ChunkSection>,
	block_entities: Vec<Map>,
	carving_masks: CarvingMasks,
	heightmaps: HeightMaps,
	lights: ListTag,
	entities: ListTag,
	fluid_ticks: Vec<Map>,
	block_ticks: Vec<Map>,
	post_processing: Vec<ListTag>,
	structures: Map,
}

pub struct ChunkSection {
	y: i8,
	block_states: BlockStates,
	biomes: Option<Biomes>,
	skylight: Option<LightData>,
	blocklight: Option<LightData>,
}

pub struct CarvingMasks {
	air: Vec<i8>,
	liquid: Vec<i8>,
}

pub fn get_coord_from_chunk_nbt(tag: &Tag) -> Option<(i32, i32)> {
	if let Tag::Compound(compound) = tag {
		if let Some(Tag::Int(x_pos)) = compound.get("xPos") {
			if let Some(Tag::Int(z_pos)) = compound.get("zPos") {
				return Some((*x_pos, *z_pos));
			}
		}
	}
	None
}

// impl DecodeNbt for Chunk {
//     type Error = ();

//     fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
//         if let Tag::Compound(map) = nbt {
// 			Ok(
// 				Self {
// 					data_version: 0,
// 					x: i32::decode_nbt(*map.get("xPos").expect("xPos not found.")).expect("Failed to decode xPos."),
// 					z: i32::decode_nbt(*map.get("zPos").expect("zPos not found.")).expect("Failed to decode zPos."),

// 					..Default::default()
// 				}
// 			)
// 		} else {
// 			Err(())
// 		}
//     }
// }