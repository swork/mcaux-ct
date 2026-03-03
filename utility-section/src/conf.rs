use crate::decode;
use aligned::{Aligned, Alignment};
use heapless;

/// Configuration retrieval got a little wet, factored out here.
pub struct Conf<'a, const N: usize> {
    section: &'a [u8],
    items: heapless::Vec<decode::UtilityItem, N>,
}

impl<'a, const N: usize> Conf<'a, N> {
    pub fn new(section: &'a [u8]) -> Self {
        let mut items: heapless::Vec<decode::UtilityItem, N> = heapless::Vec::new();
        if decode::collect_utility_items::<N>(section, &mut items).is_err() {
            panic!("error scanning utility section");
        }
        Self { items, section }
    }

    pub fn get_value_by_key(&self, key: &[u8]) -> Option<&'a [u8]> {
        for item in self.items.iter() {
            if let decode::UtilityItem::String {
                offset: section_offset,
                length: item_length,
            } = item
            {
                let key_end = *section_offset + key.len();
                let compare = &self.section[*section_offset..key_end];
                if compare == key {
                    let s = key_end + 1;
                    let e = s + (*item_length - (key.len() + 1));
                    return Some(&self.section[s..e]);
                }
            }
        }
        None
    }

    pub fn get_value_by_key_n(&self, key: &[u8], n: u8) -> Option<&'a [u8]> {
        let mut k: heapless::String<16, u8> = heapless::String::new();
        let _ = k.push_str(str::from_utf8(key).expect("utf8"));
        let mut b = itoa::Buffer::new();
        let _ = k.push_str(b.format(n));
        self.get_value_by_key(k.as_bytes())
    }

    pub fn get_blob_by_id<A>(&self, find_id: usize) -> Option<&'a Aligned<A, [u8]>>
    where
        A: Alignment,
    {
        for item in self.items.iter() {
            if let decode::UtilityItem::Blob { offset, length, id } = item
                && *id == find_id
            {
                let p = &self.section[*offset..*offset + *length];
                assert_eq!(p as *const [u8] as *const () as usize % A::ALIGN, 0);
                let aligned_slice: &Aligned<A, [u8]> = unsafe {
                    // This cast is safe if the pointer is guaranteed to be 4-byte aligned
                    &*(p as *const [u8] as *const Aligned<A, [u8]>)
                };
                return Some(aligned_slice);
            }
        }
        None
    }
}
