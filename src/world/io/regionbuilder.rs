use super::region::*;

/*
My plan here is to create a new RegionRebuilder construct.
I can create a RegionManager that can open a region file for
editing and would allow writing and deleting chunks.

So now the new problem to solve is the problem of efficiently
finding unused sectors of the required size within a given
region file.
*/


pub struct RegionManager {
	/// Marks chunks to be copied after the ChunkBuilder is finished
	/// writing/deleting chunks.
	writer: RegionWriter<BufWriter<tempfile::NamedTempFile>>,
	reader: RegionReader<BufReader<File>>,
	header: RegionHeader,
	compression: Compression,
	timestamp: Timestamp,
	copy_bits: RegionBitmask,
}

pub trait ChunkBuilder2 {
	pub fn build(&mut self, region_file: &mut RegionManager) -> McResult<()>;
}

pub struct RegionBuilder2 {
	origin: PathBuf,
}

impl RegionBuilder2 {
	pub fn build() -> McResult<u64> {
		todo!()
	}
}