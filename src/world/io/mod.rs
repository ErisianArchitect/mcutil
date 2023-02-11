pub mod region;
pub mod regionfile;

// impl<T: NbtWrite> Writable for T {
//     fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize,crate::McError> {
//         use crate::nbt::io::*;
// 		Ok(writer.write_nbt(self)?)
//     }
// }

// impl<T: NbtRead> Readable for T {
//     fn read_from<R: Read>(reader: &mut R) -> Result<Self,crate::McError> {
//         use crate::nbt::io::*;
// 		Ok(reader.read_nbt()?)
//     }
// }
