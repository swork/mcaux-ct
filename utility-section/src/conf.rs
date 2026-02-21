use heapless;
use crate::decode;

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
        Self {
            items,
            section,
        }
    }

    pub fn get_value_by_key(&self, key: &'a [u8]) -> Option<&'a [u8]> {
        for item in self.items.iter() {
            match item {
                decode::UtilityItem::String { offset, length } => {
                    let end = *offset + key.len();
                    let v = &self.section[*offset..end];
                    if v == key {
                        return Some(&self.section[*offset+end+1..*offset+end+1+length]);
                    }
                },
                _ => (),
            }
        }
        None
    }

    pub fn get_blob_by_id(&self, find_id: usize) -> Option<&'a [u8]> {
        for item in self.items.iter() {
            match item {
                decode::UtilityItem::Blob { offset, length, id } => {
                    if *id == find_id {
                        return Some(&self.section[*offset..*offset+length]);
                    }
                },
                _ => (),
            }
        }
         None
    }
}
