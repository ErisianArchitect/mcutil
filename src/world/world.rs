/*

*/

use std::collections::HashMap;
use super::{
	blockregistry::BlockRegistry,
	blockstate::*,
};

// 32x32 chunks
struct JavaRegion {

}

struct JavaWorld<Ct> {
	block_registry: BlockRegistry,
	chunks: HashMap<(i32, i32), Ct>,
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