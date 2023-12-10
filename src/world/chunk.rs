#![allow(unused)]

use std::collections::HashMap;
use std::default;

use crate::nbt::*;
use crate::nbt::io::*;
use crate::nbt::tag::*;

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

pub struct ChunkSections {
	sections: Vec<ChunkSection>,
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

impl DecodeNbt for Vec<ChunkSection> {
	type Error = ();

	fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
		if let Tag::List(ListTag::Compound(compounds)) = nbt {
			let sections = compounds.into_iter().map(|section| {
				ChunkSection {
					y: todo!(),
					block_states: todo!(),
					biomes: todo!(),
					skylight: todo!(),
					blocklight: todo!(),
				}
			}).collect();
			Ok(sections)
		} else {
			Err(())
		}
	}
}

// impl DecodeNbt for Chunk {
//     type Error = ();

//     fn decode_nbt(nbt: Tag) -> Result<Self, Self::Error> {
// 		macro_rules! map_decoder {
// 			($map:expr => $name:literal: $type:ty) => {
// 				<$type>::decode_nbt(*$map.get($name).expect(&format!("{} not found.", $name))).expect(&format!("Failed to decode {}", $name))
// 			};
// 		}
//         if let Tag::Compound(mut map) = nbt {
// 			Ok(Self {
// 				data_version: i32::decode_nbt(*map.get("data_version").expect("data_version not found.")).expect("Failed to decode data_version."),
// 				x: i32::decode_nbt(*map.get("xPos").expect("xPos not found.")).expect("Failed to decode xPos."),
// 				y: i32::decode_nbt(*map.get("zPos").expect("yPos not found.")).expect("Failed to decode yPos."),
// 				z: i32::decode_nbt(*map.get("zPos").expect("zPos not found.")).expect("Failed to decode zPos."),
// 				last_update: i64::decode_nbt(*map.get("last_update").expect("last_update not found.")).expect("Failed to decode last_update"),
// 				sections: todo!(),
// 				block_entities: todo!(),
// 				carving_masks: todo!(),
// 				heightmaps: todo!(),
// 				lights: todo!(),
// 				entities: todo!(),
// 				fluid_ticks: todo!(),
// 				block_ticks: todo!(),
// 				post_processing: todo!(),
// 				structures: todo!(),
// 				inhabited_time: todo!(),
// 			})
// 		} else {
// 			Err(())
// 		}
//     }
// }