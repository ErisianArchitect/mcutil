/*

*/

use std::collections::HashMap;

// 32x32 chunks
struct JavaRegion {

}

struct JavaWorld<Ct> {
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