/// Decode the ersatz-Huffman integer at addr. Inversely:
///
/// :         0..=127        : one byte (bounded at 2^7)
/// :       128..=16383      : above << 7 | next_byte (bounded by 2^14)
/// :     16384..=2097151    : above << 7 | next_byte (bounded by 2^21)
/// :   2097152..=268435455  : above << 7 | next_byte (bounded by 2^28)
/// : 268435456..=4294967295 : above << 7 | next_byte (bounded by 2^32)
///
/// Larger values don't fit usize=32, but the definition continues to recurse
/// for 64- (or whatever-) bit architectures. Type parameter M puts a smaller
/// upper limit on the acceptable bit count, mostly so tests can be run on wider
/// architectures than a cross-compiled target. You probably want the default
/// value for production use, via hinteger() below. Note too that it's OK to
/// include unnecessary leading zeros, using more bytes than needed.
///
/// Returns a tuple: the decoded value, and the raw pointer advanced to the
/// first following byte. Or None if the decoded value wasn't going to fit.
pub unsafe fn hinteger_explicit_maximum<const M: usize>(addr: *const u8) -> Option<(usize, *const u8)> {
    let max_accum_okay: usize = M >> 7;
    let mut accum = 0usize;
    let mut val: u8 = unsafe { *addr };
    let mut addr = addr;
    loop {
        addr = unsafe { addr.add(1) };
        if val < 128 {
            break;
        }
        val &= 0x7f;
        if accum > max_accum_okay - val as usize {
            return None; // result will overflow
        }
        accum = (accum << 7) + val as usize;
        val = unsafe { *addr };
    }
    Some(((accum << 7) + (val as usize), addr))
}

/// hinteger_explicit_maximum defaulted to the host's usize range.
pub fn hinteger(addr: *const u8) -> Option<(usize, *const u8)> {
    hinteger_explicit_maximum::<{ usize::MAX }>(addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero() {
        const B: [u8; 1] = *b"\0";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(val, 0usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(1) });
    }

    #[test]
    fn padded_zero() {
        const B: [u8; 2] = *b"\x80\0";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(val, 0usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(2) });
    }

    #[test]
    fn way_padded_zero() {
        const B: [u8; 10] = *b"\x80\x80\x80\x80\x80\x80\x80\x80\x80\0";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(val, 0usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(10) });
    }

    #[test]
    fn one() {
        const B: [u8; 1] = *b"\x01";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(val, 1usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(1) });
    }

    #[test]
    fn two_seventh_m1() {
        const B: [u8; 1] = *b"\x7f";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(val, 127usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(1) });
    }

    #[test]
    fn two_seventh() {
        const B: [u8; 2] = *b"\x81\x00";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(val, 128usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(2) });
    }

    #[test]
    fn two_seventh_p1() {
        const B: [u8; 2] = *b"\x81\x01";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(val, 129usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(2) });
    }

    #[test]
    fn two_fourteenth_m1() {
        const B: [u8; 2] = *b"\xff\x7f";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(val, 16383usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(2) });
    }

    #[test]
    fn two_fourteenth() {
        const B: [u8; 3] = *b"\x81\x80\x00";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(val, 16384usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(3) });
    }

    #[test]
    fn two_fourteenth_p1() {
        const B: [u8; 3] = *b"\x81\x80\x01";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(val, 16385usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(3) });
    }

    #[test]
    fn two_twentyfirst_m1() {
        const B: [u8; 3] = *b"\xff\xff\x7f";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(0x1fffff, 2_097_151usize);
        assert_eq!(val, 2_097_151usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(3) });
    }

    #[test]
    fn two_twentyfirst() {
        const B: [u8; 4] = *b"\x81\x80\x80\x00";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(0x200000, 2_097_152usize);
        assert_eq!(val, 2_097_152usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(4) });
    }

    #[test]
    fn two_twentyfirst_p1() {
        const B: [u8; 4] = *b"\x81\x80\x80\x01";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(0x200001, 2_097_153usize);
        assert_eq!(val, 2_097_153usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(4) });
    }

    #[test]
    fn two_twentyeighth_m1() {
        const B: [u8; 4] = *b"\xff\xff\xff\x7f";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(0xfffffffusize, 268_435_455usize);
        assert_eq!(val, 268_435_455usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(4) });
    }

    #[test]
    fn two_twentyeighth() {
        const B: [u8; 5] = *b"\x81\x80\x80\x80\x00";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(0x10000000usize, 268_435_456usize);
        assert_eq!(val, 268_435_456usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(5) });
    }

    #[test]
    fn two_twentyeighth_p1() {
        const B: [u8; 5] = *b"\x81\x80\x80\x80\x01";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(0x10000001usize, 268_435_457usize);
        assert_eq!(val, 268_435_457usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(5) });
    }

    #[test]
    fn two_thirtysecond_m1() {
        const B: [u8; 5] = *b"\x8f\xff\xff\xff\x7f";
        let (val, addr) = hinteger(B.as_ptr()).unwrap();
        assert_eq!(0xffffffffusize, 4_294_967_295usize);
        assert_eq!(val, 4_294_967_295usize);
        assert_eq!(addr, unsafe { B.as_ptr().add(5) });
    }

    #[test]
    fn two_thirtysecond() {
        const B: [u8; 5] = *b"\x90\x80\x80\x80\x00"; // 33 significant bits
        let (val, addr) =
            hinteger(B.as_ptr()).expect("Decodes to 33 significant bits, so panic here");
        assert_eq!(0x100000000usize, 4_294_967_296usize); // this shouldn't compile
        assert_eq!(val, 4_294_967_296usize); // nor this
        assert_eq!(addr, unsafe { B.as_ptr().add(5) });
    }
}
