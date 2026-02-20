use crate::decode::UtilitySection;

unsafe extern "C" {
    static __utility_start: u8;
}

/// Configuration retrieval got a little wet, factored out here.
pub struct Conf<const N: usize> {}

impl<const N: usize> Conf<N> {
    pub fn get_value_by_key<'a>(key: &[u8], buf: &'a mut [u8; N]) -> Option<&'a [u8]> {
        let mut u: UtilitySection<N> = unsafe { UtilitySection::new(&__utility_start) };
        loop {
            let mut mybuf: [u8; N] = [0; N];
            if let Some(p) = u.next_string(&mut mybuf) {
                if let Some(sep) = p.iter().position(|&b| b == b'=') {
                    if &p[..sep] == key {
                        *buf = mybuf;
                        return Some(&buf[sep + 1..]);
                    }
                } else {
                    panic!("config string isn't in k=v form");
                }
            } else {
                break;
            }
        }
        None
    }

    pub fn get_blob_by_id(id: usize) -> Option<&'static [u8]> {
        let mut u: UtilitySection<N> = unsafe { UtilitySection::new(&__utility_start) };
        while let Some((len, raw_ptr, blob_id)) = u.next_blob() {
            if blob_id == id {
                return unsafe { Some(core::slice::from_raw_parts(raw_ptr, len)) };
            }
        }
        None
    }
}
