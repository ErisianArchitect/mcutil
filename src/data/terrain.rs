/*
I couldn't think of a better name for this module.
This module is for a special data structure that holds minecraft block data.
There are 4096 blocks in each block section. That is 16x16x16.
There are various ways that the data may be arranged in a chunk. That means
there are various ways we can split the data to make it have a smaller memory footprint.

Terrain Splits:
	None -> There is no splitting. All 4096 block states are stored.
	Fill(block) -> A single block fills the entire chunk.
	Octree(1) -> The chunk is split into 8 8x8x8 chunks.
	Octree(2) -> The chunk is split into 8 8x8x8 octrees which contain 8 4x4x4 chunks.

*/

#![allow(unused)]

use crate::math::geometry;

struct Octree<T> {
	nodes: Box<[Option<T>; 8]>
}

impl<T> Octree<T> {

	fn set_node(&mut self, x: u8, y: u8, z: u8, value: T) {
		let index = geometry::octree_node_index(x, y, z);
		self.nodes[index] = Some(value)
	}

	fn get_node(&self, x: u8, y: u8, z: u8) -> Option<&T> {
		let index = geometry::octree_node_index(x, y, z);
		// remap &mut Option<T> into Option<&T>
		let Some(value) = &self.nodes[index] else { return None; };
		Some(value)
	}

	fn get_node_mut(&mut self, x: u8, y: u8, z: u8) -> Option<&mut T> {
		let index = geometry::octree_node_index(x, y, z);
		// remap &mut Option<T> into Option<&mut T>
		let Some(value) = &mut self.nodes[index] else { return None; };
		Some(value)
	}
}