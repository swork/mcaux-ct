use core::ptr::copy_nonoverlapping;
use crate::hinteger::hinteger_explicit_maximum;

/// Track an in-memory UTILITY section at a given fixed address in memory.
pub struct UtilitySection<const N: usize, const M: usize = { usize::MAX }> {
    strings_addr: *const u8,
    blobs_addr: Option<*const u8>,
}

impl<const N: usize, const M:usize> UtilitySection<N, M> {
    pub fn new(start_addr: *const u8) -> Self {
        Self {
            strings_addr: start_addr,
            blobs_addr: None,
        }
    }

    /// Return a string from this UtilitySection, or None. Subsequent calls
    /// after getting a string give another string or None. Calls
    /// after None always return None. Calls to next_string() can be
    /// intermixed with calls to next_blob() in any order.
    pub fn next_string(&mut self) -> Option<(usize, [u8;N])> {
        if let Some((len, addr)) = hinteger_explicit_maximum::<M>(self.strings_addr) {
            if len > 0 {
                if len > N {
                    panic!("Inadequate buffer for string of length {:?}", len);
                }
                self.strings_addr = unsafe { addr.add(len) };
                let mut buf: [u8; N] = [0;N];
                unsafe { copy_nonoverlapping(addr, buf.as_mut_ptr(), len); }
                Some((len, buf))
            } else {
                self.blobs_addr = unsafe { Some(self.strings_addr.add(1)) };
                None
            }
        } else {
            None
        }
    }

    /// Return a blob from this UtilitySection, or None. Subsequent calls
    /// after getting a blob give another blob or None. Calls
    /// after None always return None. Calls to next_blob() can be
    /// intermixed with calls to next_string() in any order.
    pub fn next_blob(&mut self) -> Option<(usize, *const u8, usize)> {
        if self.blobs_addr == None {
            let save = self.strings_addr;
            loop {
                if let Some((len, addr)) = hinteger_explicit_maximum::<M>(self.strings_addr) {
                    if len == 0 { break; }
                    self.strings_addr = unsafe { addr.add(len) };
                } else {
                    return None;
                }
            }
            self.blobs_addr = unsafe { Some(self.strings_addr.add(1)) };
            self.strings_addr = save;
        }
        if let Some((blob_len, id_addr)) = hinteger_explicit_maximum::<M>(self.blobs_addr.unwrap()) {
            if blob_len > 0 {
                if let Some((blob_id, align_addr)) = hinteger_explicit_maximum::<M>(id_addr) {
                    if let Some((blob_align, pad_addr)) = hinteger_explicit_maximum::<M>(align_addr) {
                        let alignment = 1 << blob_align;
                        let blob_addr = unsafe { pad_addr.add(pad_addr.align_offset(alignment)) };
                        self.blobs_addr = unsafe { Some(blob_addr.add(blob_len)) };
                        Some((blob_len, blob_addr, blob_id))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(align(8))]
    struct Aligned([u8;24]);
    const S1: Aligned = Aligned(*b"\x04AP=X\x03P=Y\0\x07\0\x03>>>DEADBEE\0");
    //                             ____----____---__----__----___-------__
    //          1st string len is 4 +   |    |  |  |  |  |  |  |    |    +- no second blob
    //                      The string -+    |  |  |  |  |  |  |    +- bytes of first blob
    //                 3-byte second string -+  |  |  |  |  |  +- pad to alignment
    //                       The second string -+  |  |  |  +- alignment spec: 2^3 = 8 bytes
    //                  There are no more strings -+  |  +- blob ID: zero
    //                                                +- seven byte blob length

    #[test]
    fn it_works() {
        const MAX: usize = 8;
        let mut s: UtilitySection<MAX> = UtilitySection::new(S1.0.as_ptr());

        // two strings, no more, ever
        let (len, buf) = s.next_string().unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], "AP=X".as_bytes());
        let (len, buf) = s.next_string().unwrap();
        assert_eq!(len, 3);
        assert_eq!(&buf[..len], "P=Y".as_bytes());
        assert_eq!(s.next_string(), None);
        assert_eq!(s.next_string(), None);

        // One blob, aligned, no more ever
        let (len, addr, id) = s.next_blob().unwrap();
        assert_eq!(len, 7);
        assert_eq!(id, 0);
        unsafe {
            assert_eq!(*addr, b'D');
            assert_eq!(*addr.add(1), b'E');
            assert_eq!(*addr.add(2), b'A');
            assert_eq!(*addr.add(3), b'D');
            assert_eq!(*addr.add(4), b'B');
            assert_eq!(*addr.add(5), b'E');
            assert_eq!(*addr.add(6), b'E');
        }
        assert_eq!(addr as usize % 8, 0);  // aligned
        assert_eq!(s.next_blob(), None);
        assert_eq!(s.next_blob(), None);
    }

    #[test]
    fn it_works_backward() {
        const MAX: usize = 8;
        let mut s: UtilitySection<MAX> = UtilitySection::new(S1.0.as_ptr());

        // One blob, aligned, no more ever
        let (len, addr, id) = s.next_blob().unwrap();
        assert_eq!(len, 7);
        assert_eq!(id, 0);
        unsafe {
            assert_eq!(*addr, b'D');
            assert_eq!(*addr.add(1), b'E');
            assert_eq!(*addr.add(2), b'A');
            assert_eq!(*addr.add(3), b'D');
            assert_eq!(*addr.add(4), b'B');
            assert_eq!(*addr.add(5), b'E');
            assert_eq!(*addr.add(6), b'E');
        }
        assert_eq!(addr as usize % 8, 0);  // aligned
        assert_eq!(s.next_blob(), None);
        assert_eq!(s.next_blob(), None);

        // two strings, no more, ever
        let (len, buf) = s.next_string().unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], "AP=X".as_bytes());
        let (len, buf) = s.next_string().unwrap();
        assert_eq!(len, 3);
        assert_eq!(&buf[..len], "P=Y".as_bytes());
        assert_eq!(s.next_string(), None);
        assert_eq!(s.next_string(), None);
    }

    #[test]
    #[should_panic]
    fn it_panics_for_length() {
        const MAX: usize = 3;
        let mut s: UtilitySection<MAX> = UtilitySection::new(S1.0.as_ptr());
        let (_len, _buf) = s.next_string().unwrap();
    }
}
