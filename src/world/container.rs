use super::{blockregistry::BlockRegistry, blockstate::BlockState};

pub struct BlockContainer {
	size: (u16, u16, u16),
	blocks: Box<[u32]>,
	block_registry: BlockRegistry,
}

impl BlockContainer {
	pub fn new(size: (u16, u16, u16)) -> Self {
		let blocks = vec![0u32; size.0 as usize *size.1 as usize *size.2 as usize];
		Self {
			blocks: blocks.into_boxed_slice(),
			size,
			block_registry: BlockRegistry::new(),
		}
	}

	pub fn size<R: From<(u16, u16, u16)>>(&self) -> R {
		R::from(self.size)
	}

	fn block_index(&self, x: u16, y: u16, z: u16) -> Option<usize> {
		if x > self.size.0 || y > self.size.1 || z > self.size.2 {
			return None;
		}
		let (xs, zs) = (self.size.0 as usize, self.size.2 as usize);
		let (x, y, z) = (x as usize, y as usize, z as usize);
		let index = y * (xs*zs) + z * xs + x;
		Some(index)
	}

	pub fn get_block_id(&self, x: u16, y: u16, z: u16) -> Option<u32> {
		let index = self.block_index(x, y, z)?;
		Some(self.blocks[index])
	}

	pub fn get_block_state(&self, x: u16, y: u16, z: u16) -> Option<BlockState> {
		let id = self.get_block_id(x, y, z)?;
		self.block_registry.get(id)
	}

	pub fn set_block_id(&mut self, x: u16, y: u16, z: u16, id: u32) -> Option<u32> {
		let index = self.block_index(x, y, z)?;
		let old_id = self.blocks[index];
		self.blocks[index] = id;
		Some(old_id)
	}

	pub fn set_block_state(&mut self, x: u16, y: u16, z: u16, state: impl Into<BlockState>) -> Option<BlockState> {
		let state = state.into();
		let id = self.block_registry.register(state);
		let old_id = self.set_block_id(x, y, z, id)?;
		self.block_registry.get(old_id)
	}
}