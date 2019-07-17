use std::io;
use byteorder::{ReadBytesExt};

pub struct BitReader {
	current: u8,
	bit_pos: u32,
}

impl BitReader {
    pub fn new() -> Result<BitReader, failure::Error> {
        Ok(BitReader{
            current: 0,
            bit_pos: 8,
        })
    }


    // returns (read, bits)
    fn read_at_most(&mut self, data: &mut io::Read, n: u32) -> Result<(u32, u32), failure::Error> {
        let mut read: u32;
        let mut bits: u32;

        bits = self.current as u32;
        bits = bits >> self.bit_pos as u32;
        bits = bits & ((1 << n) - 1);
        read = 8 - self.bit_pos;
        if read > n {
            read = n;
        }
        self.bit_pos += read;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.current = data.read_u8()?;
        }
        Ok((read,bits))
    }

     pub fn read_bits(&mut self, data: &mut io::Read, n: u32) -> Result<u32, failure::Error> {
        let mut bits: u32 = 0;
        let mut pos: u32 = 0;

        let mut i = n;

        while i > 0 {
            let (read, next) = self.read_at_most(data, i)?;
            bits = bits | (next << pos);
            pos += read;
            i -= read;
        }
        Ok(bits)
    }
}

