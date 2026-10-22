//! LZF compression (liblzf format), as Krita uses for layer tiles in `.kra` files.

/// Decompresses `input` into exactly `out_len` bytes. `None` if the data is corrupt or
/// does not decode to exactly that size.
pub fn decompress(input: &[u8], out_len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(out_len);
    let mut ip = 0;
    while ip < input.len() {
        let ctrl = input[ip] as usize;
        ip += 1;
        if ctrl < 32 {
            // Literal run of ctrl + 1 bytes.
            let n = ctrl + 1;
            let lit = input.get(ip..ip + n)?;
            if out.len() + n > out_len {
                return None;
            }
            out.extend_from_slice(lit);
            ip += n;
        } else {
            // Back reference.
            let mut len = ctrl >> 5;
            if len == 7 {
                len += *input.get(ip)? as usize;
                ip += 1;
            }
            len += 2;
            let back = ((ctrl & 0x1f) << 8) + *input.get(ip)? as usize + 1;
            ip += 1;
            if back > out.len() || out.len() + len > out_len {
                return None;
            }
            let start = out.len() - back;
            // Byte by byte: the source may overlap what is being written.
            for i in 0..len {
                out.push(out[start + i]);
            }
        }
    }
    (out.len() == out_len).then_some(out)
}

const HASH_BITS: u32 = 14;
const MAX_OFFSET: usize = 1 << 13; // 8192
const MAX_MATCH: usize = 264; // 7 + 255 + 2
const MAX_LITERAL: usize = 32;

fn flush_literals(out: &mut Vec<u8>, lit: &[u8]) {
    for chunk in lit.chunks(MAX_LITERAL) {
        out.push((chunk.len() - 1) as u8);
        out.extend_from_slice(chunk);
    }
}

/// Compresses `input`. Returns `None` if the result would not be smaller than `limit`
/// bytes (the caller then stores the data raw, like Krita does).
pub fn compress(input: &[u8], limit: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() / 2 + 16);
    let mut table = vec![usize::MAX; 1 << HASH_BITS];
    let hash = |p: &[u8]| -> usize {
        let v = (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32;
        (v.wrapping_mul(2_654_435_761) >> (32 - HASH_BITS)) as usize
    };
    let mut lit_start = 0;
    let mut ip = 0;
    while ip + 2 < input.len() {
        let h = hash(&input[ip..ip + 3]);
        let r = table[h];
        table[h] = ip;
        if r != usize::MAX
            && r < ip
            && ip - r - 1 < MAX_OFFSET
            && input[r..r + 3] == input[ip..ip + 3]
        {
            let off = ip - r - 1;
            let max = MAX_MATCH.min(input.len() - ip);
            let mut len = 3;
            while len < max && input[r + len] == input[ip + len] {
                len += 1;
            }
            flush_literals(&mut out, &input[lit_start..ip]);
            let l = len - 2;
            if l < 7 {
                out.push(((l << 5) | (off >> 8)) as u8);
            } else {
                out.push(((7 << 5) | (off >> 8)) as u8);
                out.push((l - 7) as u8);
            }
            out.push((off & 0xff) as u8);
            ip += len;
            lit_start = ip;
        } else {
            ip += 1;
        }
        if out.len() >= limit {
            return None;
        }
    }
    flush_literals(&mut out, &input[lit_start..]);
    (out.len() < limit).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(data: &[u8]) {
        let c = compress(data, usize::MAX).expect("compresses");
        let d = decompress(&c, data.len()).expect("decompresses");
        assert_eq!(d, data);
    }

    #[test]
    fn roundtrips() {
        roundtrip(b"");
        roundtrip(b"a");
        roundtrip(b"abcabcabcabcabcabcabcabcabcabcabc");
        roundtrip(&[0u8; 16384]);
        roundtrip(&(0..20000).map(|i| (i * 7 % 256) as u8).collect::<Vec<_>>());
        // Pseudo-random bytes (incompressible) still roundtrip.
        let mut x = 12345u32;
        let noise: Vec<u8> = (0..5000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x as u8
            })
            .collect();
        roundtrip(&noise);
        // Long runs exceed the 264-byte match limit and the 32-byte literal limit.
        let mut mixed = vec![7u8; 1000];
        mixed.extend_from_slice(&noise[..100]);
        mixed.extend(vec![9u8; 777]);
        roundtrip(&mixed);
    }

    #[test]
    fn limit_reports_incompressible_data() {
        let mut x = 99u32;
        let noise: Vec<u8> = (0..4096)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x as u8
            })
            .collect();
        assert!(compress(&noise, noise.len()).is_none());
        assert!(compress(&[0u8; 4096], 4096).unwrap().len() < 100);
    }

    #[test]
    fn corrupt_input_is_rejected_not_panicking() {
        assert!(
            decompress(&[0x20, 0x05], 10).is_none(),
            "back reference before the start"
        );
        assert!(
            decompress(&[5, 1, 2], 6).is_none(),
            "literal runs past the end"
        );
        assert!(decompress(&[0, 1], 2).is_none(), "wrong output size");
        assert!(
            decompress(&[0, 1, 0x20, 0x00], 3).is_none(),
            "longer than expected"
        );
    }
}
