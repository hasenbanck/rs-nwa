use std::io::Read;

pub struct BitReader<'a> {
    data: &'a mut Read,
    current: u32,
    bit_pos: u32,
}

struct ReadResult {
    bits: u32,
    read: u32,
}

impl<'a> BitReader<'a> {
    pub fn new(r: &'a mut Read) -> BitReader<'a> {
        BitReader {
            data: r,
            current: 0,
            bit_pos: 8,
        }
    }

    pub fn read_bits(&mut self, n: u32) -> Result<u32, failure::Error> {
        let mut bits: u32 = 0;
        let mut pos: u32 = 0;
        let mut n = n;
        while n > 0 {
            let r = self.read_at_most(n)?;
            bits = bits | (r.bits << pos);
            pos += r.read;
            n -= r.read;
        }
        Ok(bits)
    }

    fn read_at_most(&mut self, n: u32) -> Result<ReadResult, failure::Error> {
        let mut bits: u32 = self.current;
        bits = bits >> self.bit_pos;
        bits = bits & ((1 << n) - 1);
        let mut read = 8 - self.bit_pos;
        if read > n {
            read = n;
        }
        self.bit_pos += read;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            let mut buffer = [0; 1];
            self.data.read(&mut buffer)?;
            self.current = buffer[0] as u32;
        }
        Ok(ReadResult { read, bits })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitreader() -> Result<(), failure::Error> {
        let test = vec![0x11, 0x99, 0xff, 0xff];
        let mut s = test.as_slice();
        let mut br = BitReader::new(&mut s);

        assert_eq!(br.read_bits(8)?, 0x11);
        assert_eq!(br.read_bits(8)?, 0x99);
        assert_eq!(br.read_bits(6)?, 0x3f);
        assert_eq!(br.read_bits(2)?, 0x03);
        assert_eq!(br.read_bits(8)?, 0xff);

        Ok(())
    }
}
