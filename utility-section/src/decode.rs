use crate::hinteger::hinteger;
use heapless;

#[derive(Debug)]
pub enum UtilityItem {
    String {
        offset: usize,
        length: usize,
    },
    Blob {
        offset: usize,
        length: usize,
        id: usize,
    },
    End {
        offset: usize,
    },
}

pub fn collect_utility_items<const N: usize>(
    section: &[u8],
    items: &mut heapless::Vec<UtilityItem, N>,
) -> Result<usize, UtilityItem> {
    let mut offset: usize = 0; // pointing at first string

    loop {
        let (result, hinteger_length) = hinteger(&section[offset..]).expect("hinteger");
        if result == 0 {
            break;
        }
        offset += hinteger_length; // now at start of string
        items.push(UtilityItem::String {
            offset,
            length: result,
        })?;
        offset += result; // now at beginning of next string, maybe end marker
    }
    offset += 1; // skip the zero-length string marking end of strings
    loop {
        let (result_length, hinteger_length) = hinteger(&section[offset..]).expect("hinteger");
        if result_length == 0 {
            break;
        }
        offset += hinteger_length; // now at id
        let (result_id, hinteger_length) = hinteger(&section[offset..]).expect("hinteger");
        offset += hinteger_length; // now at alignment
        let (result_align, hinteger_length) = hinteger(&section[offset..]).expect("hinteger");
        offset += hinteger_length; // now at padding for alignment, if any
        let align_helper = offset % (1 << result_align);
        if align_helper > 0 {
            offset += (1 << result_align) - align_helper; // now past padding, at blob start
        }
        items.push(UtilityItem::Blob {
            offset,
            length: result_length,
            id: result_id,
        })?;
        offset += result_length;
    }
    offset += 1; // skip the zero-length blob marking end of blobs
    items.push(UtilityItem::End { offset })?;
    Ok(offset)
}
