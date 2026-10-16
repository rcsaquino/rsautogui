//! Minimal PNG decoder: 8-bit, non-interlaced, color types 0/2/4/6.
//!
//! Just enough to decode screenshots produced by `xdg-desktop-portal` /
//! GNOME Shell (which always write 8-bit RGB/RGBA PNGs). No dependencies.

use std::io;

use super::{adler32, crc32, Image};

fn err(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

// ---------------------------------------------------------------------------
// DEFLATE
// ---------------------------------------------------------------------------

struct BitReader<'a> {
    data: &'a [u8],
    byte: usize,
    bit: u8,
}

impl<'a> BitReader<'a> {
    fn bit(&mut self) -> io::Result<u32> {
        if self.byte >= self.data.len() {
            return Err(err("truncated DEFLATE stream"));
        }
        let v = ((self.data[self.byte] >> self.bit) & 1) as u32;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.byte += 1;
        }
        Ok(v)
    }
    fn bits(&mut self, n: u32) -> io::Result<u32> {
        let mut v = 0u32;
        for i in 0..n {
            v |= self.bit()? << i;
        }
        Ok(v)
    }
    fn align_byte(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.byte += 1;
        }
    }
    fn bytes(&mut self, n: usize) -> io::Result<&'a [u8]> {
        self.align_byte();
        if self.byte + n > self.data.len() {
            return Err(err("truncated DEFLATE stream"));
        }
        let s = &self.data[self.byte..self.byte + n];
        self.byte += n;
        Ok(s)
    }
    fn u16le(&mut self) -> io::Result<u16> {
        let b = self.bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
}

struct Huffman {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huffman {
    fn from_lengths(lengths: &[u8]) -> io::Result<Self> {
        let mut counts = [0u16; 16];
        for &l in lengths {
            if l as usize > 15 {
                return Err(err("invalid Huffman code length"));
            }
            if l != 0 {
                counts[l as usize] += 1;
            }
        }
        let mut order: Vec<usize> = (0..lengths.len()).filter(|&i| lengths[i] != 0).collect();
        order.sort_by_key(|&i| (lengths[i], i));
        let symbols: Vec<u16> = order.iter().map(|&i| i as u16).collect();
        Ok(Self { counts, symbols })
    }

    fn decode(&self, br: &mut BitReader) -> io::Result<u16> {
        // Canonical LSB…MSB-first decode (same algorithm as puff.c):
        // the first bit read is the most significant bit of the code.
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: usize = 0;
        for _len in 1..=15usize {
            code |= br.bit()? as i32;
            let count = self.counts[_len] as i32;
            if code - first < count {
                return Ok(self.symbols[index + (code - first) as usize]);
            }
            index += count as usize;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(err("invalid Huffman code"))
    }
}

const LBASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
    131, 163, 195, 227, 258,
];
const LEXT: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DBASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025,
    1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DEXT: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12,
    12, 13, 13,
];

fn fixed_tables() -> io::Result<(Huffman, Huffman)> {
    let mut lit = [0u8; 288];
    for (i, l) in lit.iter_mut().enumerate() {
        *l = if i <= 143 {
            8
        } else if i <= 255 {
            9
        } else if i <= 279 {
            7
        } else {
            8
        };
    }
    let dist = [5u8; 30];
    Ok((Huffman::from_lengths(&lit)?, Huffman::from_lengths(&dist)?))
}

fn dynamic_tables(br: &mut BitReader) -> io::Result<(Huffman, Huffman)> {
    const ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let hlit = br.bits(5)? as usize + 257;
    let hdist = br.bits(5)? as usize + 1;
    let hclen = br.bits(4)? as usize + 4;
    if hlit > 288 || hdist > 30 {
        return Err(err("invalid dynamic Huffman header"));
    }
    let mut cl_len = [0u8; 19];
    for i in 0..hclen {
        cl_len[ORDER[i]] = br.bits(3)? as u8;
    }
    let cl = Huffman::from_lengths(&cl_len)?;
    let mut lens = vec![0u8; hlit + hdist];
    let mut i = 0;
    while i < lens.len() {
        let s = cl.decode(br)? as usize;
        match s {
            0..=15 => {
                lens[i] = s as u8;
                i += 1;
            }
            16 => {
                if i == 0 {
                    return Err(err("invalid length repeat"));
                }
                let rep = br.bits(2)? as usize + 3;
                let prev = lens[i - 1];
                for _ in 0..rep {
                    if i >= lens.len() {
                        return Err(err("length overflow"));
                    }
                    lens[i] = prev;
                    i += 1;
                }
            }
            17 => {
                let rep = br.bits(3)? as usize + 3;
                for _ in 0..rep {
                    if i >= lens.len() {
                        return Err(err("length overflow"));
                    }
                    lens[i] = 0;
                    i += 1;
                }
            }
            18 => {
                let rep = br.bits(7)? as usize + 11;
                for _ in 0..rep {
                    if i >= lens.len() {
                        return Err(err("length overflow"));
                    }
                    lens[i] = 0;
                    i += 1;
                }
            }
            _ => return Err(err("invalid code-length symbol")),
        }
    }
    if lens[256] == 0 {
        return Err(err("missing end-of-block code"));
    }
    let lit = Huffman::from_lengths(&lens[..hlit])?;
    let dist = Huffman::from_lengths(&lens[hlit..])?;
    Ok((lit, dist))
}

fn decode_block(br: &mut BitReader, lit: &Huffman, dist: &Huffman, out: &mut Vec<u8>) -> io::Result<bool> {
    loop {
        let sym = lit.decode(br)? as usize;
        if sym < 256 {
            out.push(sym as u8);
        } else if sym == 256 {
            return Ok(true);
        } else {
            let li = sym - 257;
            if li >= 29 {
                return Err(err("invalid length symbol"));
            }
            let len = LBASE[li] as usize + br.bits(LEXT[li] as u32)? as usize;
            let dsym = dist.decode(br)? as usize;
            if dsym >= 30 {
                return Err(err("invalid distance symbol"));
            }
            let d = DBASE[dsym] as usize + br.bits(DEXT[dsym] as u32)? as usize;
            if d == 0 || d > out.len() {
                // `d > out.len()` can legitimately happen only in corrupt
                // streams; dist tables with a single code are handled by the
                // encoder emitting real distances, so treat as corrupt.
                return Err(err("invalid back-reference distance"));
            }
            let start = out.len() - d;
            for i in 0..len {
                let b = out[start + i];
                out.push(b);
            }
        }
    }
}

fn inflate(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut br = BitReader {
        data,
        byte: 0,
        bit: 0,
    };
    let mut out = Vec::new();
    loop {
        let last = br.bits(1)? != 0;
        let typ = br.bits(2)?;
        match typ {
            0 => {
                br.align_byte();
                let len = br.u16le()?;
                let nlen = br.u16le()?;
                if nlen != !len {
                    return Err(err("bad stored-block length"));
                }
                let b = br.bytes(len as usize)?;
                out.extend_from_slice(b);
            }
            1 => {
                let (lit, dist) = fixed_tables()?;
                decode_block(&mut br, &lit, &dist, &mut out)?;
            }
            2 => {
                let (lit, dist) = dynamic_tables(&mut br)?;
                decode_block(&mut br, &lit, &dist, &mut out)?;
            }
            _ => return Err(err("reserved DEFLATE block type")),
        }
        if last {
            break;
        }
    }
    Ok(out)
}

fn zlib_decompress(data: &[u8]) -> io::Result<Vec<u8>> {
    if data.len() < 6 {
        return Err(err("truncated zlib stream"));
    }
    let cmf = data[0] as u32;
    let flg = data[1] as u32;
    if (cmf * 256 + flg) % 31 != 0 {
        return Err(err("bad zlib header check"));
    }
    if cmf & 0x0F != 8 {
        return Err(err("unsupported zlib compression method"));
    }
    if flg & 0x20 != 0 {
        return Err(err("zlib preset dictionaries not supported"));
    }
    let out = inflate(&data[2..data.len() - 4])?;
    let stored = u32::from_be_bytes([
        data[data.len() - 4],
        data[data.len() - 3],
        data[data.len() - 2],
        data[data.len() - 1],
    ]);
    if adler32(&out) != stored {
        return Err(err("zlib Adler-32 mismatch"));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Scanline unfiltering
// ---------------------------------------------------------------------------

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (a, b, c) = (a as i32, b as i32, c as i32);
    let p = a + b - c;
    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

fn unfilter(width: usize, height: usize, channels: usize, raw: &[u8]) -> io::Result<Vec<u8>> {
    let stride = width * channels;
    if raw.len() != height * (stride + 1) {
        return Err(err("PNG scanline length mismatch"));
    }
    let mut out = vec![0u8; height * stride];
    let mut prev = vec![0u8; stride];
    for y in 0..height {
        let row = &raw[y * (stride + 1)..(y + 1) * (stride + 1)];
        let f = row[0];
        if f > 4 {
            return Err(err("unknown PNG filter"));
        }
        let cur = &mut out[y * stride..(y + 1) * stride];
        for x in 0..stride {
            let a = if x >= channels { cur[x - channels] } else { 0 };
            let b = prev[x];
            let c = if x >= channels { prev[x - channels] } else { 0 };
            cur[x] = match f {
                0 => row[1 + x],
                1 => row[1 + x].wrapping_add(a),
                2 => row[1 + x].wrapping_add(b),
                3 => row[1 + x].wrapping_add(((a as u16 + b as u16) / 2) as u8),
                _ => row[1 + x].wrapping_add(paeth(a, b, c)),
            };
        }
        prev.copy_from_slice(cur);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Top level
// ---------------------------------------------------------------------------

/// Decodes an 8-bit non-interlaced PNG (color types 0/2/4/6) into an [`Image`].
pub(crate) fn decode_png(bytes: &[u8]) -> io::Result<Image> {
    const SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if bytes.len() < 8 || bytes[0..8] != SIG {
        return Err(err("not a PNG file"));
    }
    let mut pos = 8usize;
    let mut ihdr: Option<(u32, u32, u8, u8)> = None;
    let mut idat: Vec<u8> = Vec::new();

    let read_u32 = |b: &[u8], p: &mut usize| -> io::Result<u32> {
        if *p + 4 > b.len() {
            return Err(err("truncated PNG"));
        }
        let v = u32::from_be_bytes([b[*p], b[*p + 1], b[*p + 2], b[*p + 3]]);
        *p += 4;
        Ok(v)
    };

    loop {
        if pos == bytes.len() {
            break; // tolerate missing IEND if we already have data
        }
        let len = read_u32(bytes, &mut pos)? as usize;
        if pos + 8 > bytes.len() {
            return Err(err("truncated PNG chunk"));
        }
        let ty = [bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]];
        pos += 4;
        if pos + len + 4 > bytes.len() {
            return Err(err("truncated PNG chunk data"));
        }
        let data = &bytes[pos..pos + len];
        pos += len;
        let stored_crc = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]);
        pos += 4;
        let mut chk = Vec::with_capacity(4 + len);
        chk.extend_from_slice(&ty);
        chk.extend_from_slice(data);
        if crc32(&chk) != stored_crc {
            return Err(err("PNG CRC mismatch"));
        }

        match &ty {
            b"IHDR" => {
                if len != 13 || ihdr.is_some() {
                    return Err(err("bad IHDR"));
                }
                let w = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                let h = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                let (depth, ctype) = (data[8], data[9]);
                if data[10] != 0 || data[11] != 0 || data[12] != 0 {
                    return Err(err("unsupported PNG format (need 8-bit non-interlaced)"));
                }
                if depth != 8 {
                    return Err(err("only 8-bit PNGs are supported"));
                }
                if !matches!(ctype, 0 | 2 | 4 | 6) {
                    return Err(err("unsupported PNG color type (no palette)"));
                }
                if w == 0 || h == 0 || (w as u64) * (h as u64) > 100_000_000 {
                    return Err(err("implausible PNG dimensions"));
                }
                ihdr = Some((w, h, depth, ctype));
            }
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            _ if ty[0] & 0x20 == 0 => {
                // Uppercase first letter = critical chunk we don't understand.
                return Err(err(format!(
                    "unsupported critical PNG chunk {}",
                    String::from_utf8_lossy(&ty)
                )));
            }
            _ => {} // ignore ancillary chunks (tEXt, pHYs, tRNS, …)
        }
    }

    let (w, h, _depth, ctype) = ihdr.ok_or_else(|| err("PNG missing IHDR"))?;
    if idat.is_empty() {
        return Err(err("PNG missing IDAT"));
    }
    let raw = zlib_decompress(&idat)?;
    let channels = match ctype {
        0 => 1,
        2 => 3,
        4 => 2,
        _ => 4,
    };
    let pixels = unfilter(w as usize, h as usize, channels, &raw)?;

    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    match channels {
        1 => {
            for v in pixels {
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        2 => {
            for px in pixels.chunks_exact(2) {
                rgba.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
        }
        3 => {
            for px in pixels.chunks_exact(3) {
                rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        _ => rgba = pixels,
    }
    Ok(Image::from_rgba(w, h, rgba))
}
