use crate::decode;
use aligned::{Aligned, Alignment};
use heapless;

const MAXITEMS: usize = 9;

/// Configuration retrieval got a little wet, factored out here.
pub struct Conf<'a, const N: usize> {
    section: &'a [u8],
    items: heapless::Vec<decode::UtilityItem, MAXITEMS>,
}

impl<'a, const N: usize> Conf<'a, N> {
    pub fn new(section: &'a [u8]) -> Self {
        let mut items: heapless::Vec<decode::UtilityItem, MAXITEMS> = heapless::Vec::new();
        if decode::collect_utility_items::<MAXITEMS>(section, &mut items).is_err() {
            panic!("error scanning utility section");
        }
        Self { items, section }
    }

    pub fn get_value_by_key(&self, key: &[u8]) -> Option<&'a [u8]> {
        for item in self.items.iter() {
            if let decode::UtilityItem::String { offset, length } = item {
                let end = *offset + &key.len();
                let v = &self.section[*offset..end];
                if v == key {
                    return Some(&self.section[*offset + end + 1..*offset + end + 1 + length]);
                }
            }
        }
        None
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
