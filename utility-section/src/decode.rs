use heapless;
use crate::hinteger::hinteger;

pub enum UtilityItem {
    String { offset: usize, length: usize },
    Blob { offset: usize, length: usize, id: usize },
    End { offset: usize },
}

pub fn collect_utility_items<const N: usize>(section: &[u8], items: &mut heapless::Vec<UtilityItem, N>) -> Result<(), UtilityItem> {
    let mut offset: usize = 0;  // pointing at first string

    loop {
        let (bytes, length) = hinteger(&section[offset..]).expect("hinteger");
        if length == 0 {
            break;
        }
        offset += bytes;  // now at start of string
        items.push(UtilityItem::String { offset, length })?;
        offset += length; // now at beginning of next string, maybe end marker
    }
    offset += 1;  // skip the zero-length string marking end of strings
    loop {
        let (bytes, length) = hinteger(&section[offset..]).expect("hinteger");
        if length == 0 {
            break;
        }
        offset += bytes;  // now at id
        let (bytes, id) = hinteger(&section[offset..]).expect("hinteger");
        offset += bytes;  // now at alignment
        let (bytes, align) = hinteger(&section[offset..]).expect("hinteger");
        offset += bytes;  // now at padding for alignment, if any
        let align_helper = offset % (1 << align);
        if align_helper > 0 {
            offset += (1 << align) - align_helper;  // now past padding, at blob start
        }
        items.push(UtilityItem::Blob { offset, length, id })?;
    }
    offset += 1;  // skip the zero-length blob marking end of blobs
    items.push(UtilityItem::End { offset })?;
    Ok(())
}

