use std::path::{Path};
use std::fs::{write, read};
use thiserror::Error;
use std::cmp::{PartialOrd, Ordering};

#[derive(Debug, PartialEq)]
/// The AVC2 Virtual Drive. An emulated 16mb block-based storage device. Blocks are 256 bytes long.
/// 
/// Only non-zero blocks are actually stored in memory and in the archive representation. This reduces memory and disk usage by a large margin, particularly when there's not much data on the drive.
pub struct Avd {
    blocks: Vec<Block>,
}
impl Avd {
    /// Create a new, blank AVD.
    pub fn new() -> Avd {
        Avd {
            blocks: Vec::new()
        }
    }
    /// Save the AVD to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let a = self.save_archive();
        write(&path, a)?;

        Ok(())
    }
    fn save_archive(&self) -> Vec<u8> {
        let mut ret = vec![0x41, 0x56, 0x44, 0x00];
        for b in &self.blocks {
            ret.extend(b.idx.to_be_bytes());
            ret.extend(b.data)
        }
        ret
    }
    /// Load a file into the AVD. Be warned! This will overwrite the entire drive!
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let archive = read(path)?;
        self.blocks = self.load_archive(&archive)?;
        Ok(())
    }
    fn load_archive(&self, archive: &[u8]) -> Result<Vec<Block>> {
        let header = &archive[..4];
        if header != [0x41, 0x56, 0x44, 0x00] {
            return Err(AvdError::BadHeader(header[0], header[1], header[2], header[3])) // SHOULD NEVER PANIC
        }
        let mut ret = Vec::new();
        let mut data_seg = &archive[4..];
        if data_seg.len() % 258 != 0 {
            return Err(AvdError::MalformedArchive)
        }
        loop {
            if data_seg.len() < 258 {
                break
            }
            let block = &data_seg[..258];
            data_seg = &data_seg[258..];
            let idx = u16::from_be_bytes(block[..2].try_into().unwrap()); // SHOULD NEVER PANIC
            let b = Block {
                idx, data: block[2..].try_into().unwrap()
            };
            ret.push(b)
        }

        Ok(ret)
    }
    /// "Defrag" the in-memory representation of the drive's blocks. Not sure why you'd need this. Theoretically makes access to lower blocks faster but not by much.
    pub fn sort(&mut self) {
        self.blocks.sort_by(|a, b| a.partial_cmp(&b).unwrap())
    }
    /// Load a new AVD from a file.
    pub fn from_host_drive(path: impl AsRef<Path>) -> Result<Avd> {
        let mut d = Avd::new();
        d.load(path)?;
        Ok(d)
    }

    /// Get a block from the drive.
    pub fn get_block(&self, idx: u16) -> Option<[u8; 256]> {
        self.blocks.iter().find(|b| b.idx == idx).map(|b| b.data)
    }
    /// Set a block inside the drive.
    pub fn set_block(&mut self, idx: u16, data: &[u8; 256]) {
        let b = self.blocks.iter().position(|b| b.idx == idx);
        match b {
            Some(v) => self.blocks[v].data = *data,
            None => {
                self.blocks.push(Block {
                    idx, data: *data
                })
            }
        }
    }
}
#[derive(Debug, PartialEq)]
struct Block {
    idx: u16,
    data: [u8; 256]
}
impl PartialOrd for Block {
    fn partial_cmp(&self, other: &Block) -> Option<Ordering> {
        Some(self.idx.cmp(&other.idx))
    }
}

type Result<T> = std::result::Result<T, AvdError>;

#[derive(Error, Debug)]
pub enum AvdError {
    #[error("host fs error")]
    FsError(#[from] std::io::Error),
    #[error("bad file header: {0:02x} {1:02x} {2:02x} {3:02x}")]
    BadHeader(u8, u8, u8, u8),
    #[error("malformed archive file")] // appears when the data segment is not of length 0 (mod 258)
    MalformedArchive
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_set() {
        let mut drive = Avd::new();
        let mut data = [0; 256];
        for i in 0..256 { // init with recognisable data
            data[i] = i as u8
        }
        drive.set_block(1234, &data);
        assert_eq!(drive.get_block(1234), Some(data));
        let _ = drive.save("test.avd");
        let mut drive2 = Avd::new();
        let _ = drive2.load("test.avd");
        assert_eq!(drive, drive2)
    }
}
