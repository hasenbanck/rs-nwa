mod bitreader;

extern crate byteorder;
#[macro_use]
extern crate failure;

use bitreader::BitReader;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{self, Write};

pub struct NWAHeader {
    pub channels: i16,
    pub bps: i16,
    pub freq: i32,
    pub complevel: i32,
    pub userunlength: i32,
    pub blocks: i32,
    pub datasize: i32,
    pub compdatasize: i32,
    pub samplecount: i32,
    pub blocksize: i32,
    pub restsize: i32,
    pub offsets: Vec<i32>,
}

impl NWAHeader {
    pub fn new(input: &mut io::Read) -> Result<NWAHeader, failure::Error> {
        let channels = input.read_i16::<LittleEndian>()?;
        let bps = input.read_i16::<LittleEndian>()?;
        let freq = input.read_i32::<LittleEndian>()?;
        let complevel = input.read_i32::<LittleEndian>()?;
        let userunlength = input.read_i32::<LittleEndian>()?;
        let mut blocks = input.read_i32::<LittleEndian>()?;
        let datasize = input.read_i32::<LittleEndian>()?;
        let compdatasize = input.read_i32::<LittleEndian>()?;
        let samplecount = input.read_i32::<LittleEndian>()?;
        let mut blocksize = input.read_i32::<LittleEndian>()?;
        let mut restsize = input.read_i32::<LittleEndian>()?;
        let _dummy = input.read_i32::<LittleEndian>()?;

        if complevel == -1 {
            blocksize = 65536;
            restsize = (datasize % (blocksize * (bps as i32 / 8))) / (bps as i32 / 8);
            let mut rest = 0;
            if restsize > 0 {
                rest = 1;
            }
            blocks = (datasize / (blocksize * (bps as i32 / 8))) + rest;
        }
        if blocks <= 0 || blocks > 1000000 {
            // There can't be a file with over 1hr music
            bail!("blocks are too large: {}", blocks);
        }

        let mut offsets = vec![0; blocks as usize];
        if complevel != -1 {
            // Read the offset indexes
            for i in 0..blocks as usize {
                offsets[i] = input.read_i32::<LittleEndian>()?;
            }
        }

        Ok(NWAHeader {
            channels,
            bps,
            freq,
            complevel,
            userunlength,
            blocks,
            datasize,
            compdatasize,
            samplecount,
            blocksize,
            restsize,
            offsets,
        })
    }

    fn check(&self) -> Result<(), failure::Error> {
        if self.complevel != -1 && self.offsets.is_empty() {
            bail!("no offsets set even thought they are needed");
        }
        if self.channels != 1 && self.channels != 2 {
            bail!(
                "this library only supports mono / stereo data: data has {} channels\n",
                self.channels
            );
        }
        if self.bps != 8 && self.bps != 16 {
            bail!(
                "this library only supports 8 / 16bit data: data is {} bits\n",
                self.bps
            );
        }
        if self.complevel == -1 {
            let byps = self.bps as i32 / 8; // Bytes per sample
            if self.datasize != self.samplecount * byps {
                bail!(
                    "invalid datasize: datasize {} != samplecount {} * samplesize {}\n",
                    self.datasize,
                    self.samplecount,
                    byps
                );
            }
            if self.samplecount != (self.blocks - 1) * self.blocksize + self.restsize {
                bail!("total sample count is invalid: samplecount {} != {}*{}+{}(block*blocksize+lastblocksize)\n", self.samplecount, self.blocks-1, self.blocksize, self.restsize);
            }
            return Ok(());
        }
        if self.complevel < -1 || self.complevel > 5 {
            bail!("this library supports only compression level from -1 to 5: the compression level of the data is {}\n", self.complevel);
        }
        if self.offsets[self.blocks as usize - 1] >= self.compdatasize {
            bail!("the last offset overruns the file.\n");
        }
        let byps = self.bps as i32 / 8; // Bytes per sample
        if self.datasize != self.samplecount * byps {
            bail!(
                "invalid datasize: datasize {} != samplecount {} * samplesize {}\n",
                self.datasize,
                self.samplecount,
                byps
            );
        }
        if self.samplecount != (self.blocks - 1) * self.blocksize + self.restsize {
            bail!("total sample count is invalid: samplecount {} != {}*{}+{}(block*blocksize+lastblocksize).\n", self.samplecount, self.blocks-1, self.blocksize, self.restsize);
        }
        Ok(())
    }
}

pub struct NWAFile {
    pub header: NWAHeader,
    cur_block: i32,
    data: Vec<u8>,
}

impl NWAFile {
    pub fn new(input: &mut io::Read) -> Result<NWAFile, failure::Error> {
        let header = NWAHeader::new(input)?;
        header.check()?;

        // Calculate target data size
        // WAVE header = 36 bytes
        let size = 36 + header.datasize as usize;
        let data = Vec::with_capacity(size);
        let mut nwa = NWAFile {
            header,
            cur_block: 0,
            data,
        };

        // Write the WAVE header
        nwa.write_wave_header()?;

        let mut done = 0;
        while done < nwa.header.datasize as u64 {
            done += nwa.decode_block(input)?;
        }

        Ok(nwa)
    }

    pub fn save(&mut self, filename: String) -> Result<(), failure::Error> {
        let mut f = File::create(filename)?;
        f.write_all(&mut self.data)?;
        Ok(())
    }

    #[rustfmt::skip]
    fn write_wave_header(&mut self) -> Result<(), failure::Error> {
        let byps = (self.header.bps as i16 + 7) >> 3;

        self.data.write_all(&['R' as u8, 'I' as u8, 'F' as u8, 'F' as u8])?;
        self.data.write_i32::<LittleEndian>((self.header.datasize + 0x24) as i32)?;
        self.data.write_all(&['W' as u8, 'A' as u8, 'V' as u8, 'E' as u8])?;
        self.data.write_all(&['f' as u8, 'm' as u8, 't' as u8, ' ' as u8])?;
        self.data.write_all(&[16, 0, 0, 0, 1, 0])?;
        self.data.write_i16::<LittleEndian>(self.header.channels)?;
        self.data.write_i32::<LittleEndian>(self.header.freq)?;
        self.data.write_i32::<LittleEndian>(byps as i32 * self.header.freq * self.header.channels as i32)?;
        self.data.write_i16::<LittleEndian>(byps * self.header.channels as i16)?;
        self.data.write_i16::<LittleEndian>(self.header.bps)?;
        self.data.write_all(&['d' as u8, 'a' as u8, 't' as u8, 'a' as u8])?;
        self.data.write_i32::<LittleEndian>(self.header.datasize)?;

        Ok(())
    }

    // decode_block decodes one block with each call. Returns the length of the
    // written bytes and an error if there was one.
    fn decode_block(&mut self, input: &mut io::Read) -> Result<u64, failure::Error> {
        // Uncompressed wave data stream
        if self.header.complevel == -1 {
            self.cur_block = self.header.blocks;
            let ret = io::copy(input, &mut self.data)?;
            return Ok(ret);
        }

        if self.header.blocks == self.cur_block {
            return Ok(0);
        }

        // Calculate the size of the decoded block
        let cur_blocksize: i32;
        let curcompsize: i32;
        if self.cur_block != self.header.blocks - 1 {
            cur_blocksize = self.header.blocksize * (self.header.bps as i32 / 8);
            curcompsize = self.header.offsets[self.cur_block as usize + 1]
                - self.header.offsets[self.cur_block as usize];
            if cur_blocksize >= self.header.blocksize * (self.header.bps as i32 / 8) * 2 {
                bail!("Current block exceeds the excepted count.");
            }
        } else {
            cur_blocksize = self.header.restsize * (self.header.bps as i32 / 8);
            curcompsize = self.header.blocksize * (self.header.bps as i32 / 8) * 2;
        }

        // Read in the block data
        let mut buf = vec![0; curcompsize as usize];
        input.read(&mut buf)?;

        // Decode the compressed block
        self.decode(&mut buf.as_slice(), cur_blocksize as usize)?;

        self.cur_block += 1;
        Ok(cur_blocksize as u64)
    }

    fn decode(&mut self, buf: &mut io::Read, outsize: usize) -> Result<(), failure::Error> {
        let mut d: [i32; 2] = [0, 0];
        let mut flipflag: usize = 0;
        let mut runlength: i32 = 0;

        // Read the first data (with full accuracy)
        if self.header.bps == 8 {
            d[0] = buf.read_u8()? as i32;
        } else {
            // bps == 16bit
            d[0] = buf.read_u16::<LittleEndian>()? as i32;
        }
        // Stereo
        if self.header.channels == 2 {
            if self.header.bps == 8 {
                d[1] = buf.read_u8()? as i32;
            } else {
                // bps == 16bit
                d[1] = buf.read_u16::<LittleEndian>()? as i32;
            }
        }

        let mut reader = BitReader::new(buf);

        let dsize = outsize / (self.header.bps as usize / 8);
        for _ in 0..dsize {
            // If we are not in a copy loop (RLE), read in the data
            if runlength == 0 {
                let exponent = reader.read_bits(3)?;
                // Branching according to the mantissa: 0, 1-6, 7
                match exponent {
                    7 => {
                        // 7: big exponent
                        // In case we are using RLE (complevel==5) this is disabled
                        if reader.read_bits(1)? == 1 {
                            d[flipflag] = 0;
                        } else {
                            let bits: u32;
                            let shift: u32;
                            if self.header.complevel >= 3 {
                                bits = 8;
                                shift = 9;
                            } else {
                                bits = 8 - (self.header.complevel as u32);
                                shift = 2 + 7 + (self.header.complevel as u32);
                            }
                            let mask1 = (1 << (bits - 1)) as u32;
                            let mask2 = ((1 << (bits - 1)) - 1) as u32;
                            let b = reader.read_bits(bits)?;
                            if b & mask1 != 0 {
                                d[flipflag] -= ((b & mask2) << shift) as i32;
                            } else {
                                d[flipflag] += ((b & mask2) << shift) as i32;
                            }
                        }
                    }
                    1...6 => {
                        // 1-6 : normal differencial
                        let bits: u32;
                        let shift: u32;
                        if self.header.complevel >= 3 {
                            bits = (self.header.complevel as u32) + 3;
                            shift = 1 + exponent;
                        } else {
                            bits = 5 - (self.header.complevel as u32);
                            shift = 2 + exponent + (self.header.complevel as u32);
                        }
                        let mask1 = (1 << (bits - 1)) as u32;
                        let mask2 = ((1 << (bits - 1)) - 1) as u32;
                        let b = reader.read_bits(bits)?;
                        if b & mask1 != 0 {
                            d[flipflag] -= ((b & mask2) << shift) as i32;
                        } else {
                            d[flipflag] += ((b & mask2) << shift) as i32;
                        }
                    }
                    0 => {
                        // Skips when not using RLE
                        if self.header.userunlength == 1 {
                            runlength = reader.read_bits(1)? as i32;
                            if runlength == 1 {
                                runlength = reader.read_bits(2)? as i32;
                                if runlength == 3 {
                                    runlength = reader.read_bits(8)? as i32;
                                }
                            }
                        }
                    }
                    _ => {
                        bail!("unreachable code reched");
                    }
                }
            } else {
                runlength -= 1;
            }
            if self.header.bps == 8 {
                self.data.write_u8(d[flipflag] as u8)?;
            } else {
                self.data.write_i16::<LittleEndian>(d[flipflag] as i16)?;
            }
            if self.header.channels == 2 {
                // Changing the channel
                flipflag ^= 1
            }
        }
        Ok(())
    }
}
