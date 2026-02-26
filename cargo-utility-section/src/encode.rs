use std::boxed::Box;
use std::vec::Vec;

pub fn hinteger_encode(num: usize) -> Box<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut topbit: u8 = 0;
    let mut val = num;
    loop {
        let lsb7 = (val & 0x7f as usize) as u8;
        buf.push(lsb7 | topbit);
        topbit = 0x80;
        val = val >> 7;
        if val == 0 {
            break;
        }
    }
    buf.reverse();
    Box::new(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero() {
        let result = hinteger_encode(0);
        assert_eq!([b'\0'], **result);
    }
}
