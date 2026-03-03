/*

Compiling requires file utility.bin, make it like this (in the parent directory, alongside Cargo.x):

cargo utility-section --load-address 0x10222000 --string "S0=A" --string "S1=BB" --string "URL0=http://web.local/" --blob 22,Cargo.toml,2 --blob 33,Cargo.toml,2

*/

use utility_section::conf;
use utility_section::decode;

const SECTION: &[u8] = include_bytes!("../utility.bin");

#[test]
fn parses() {
    let mut v: heapless::Vec<decode::UtilityItem, 20> = heapless::Vec::new();
    let r = decode::collect_utility_items(SECTION, &mut v).unwrap();
    assert_eq!(r, 500);
    assert_eq!(v.len(), 6);
}

#[test]
fn items() {
    let mut v: heapless::Vec<decode::UtilityItem, 20> = heapless::Vec::new();
    let _ = decode::collect_utility_items(SECTION, &mut v).unwrap();
    assert_eq!(
        500,
        match v[v.len() - 1] {
            decode::UtilityItem::End { offset } => offset,
            _ => 0,
        }
    );
    assert_eq!(
        33,
        match v[v.len() - 2] {
            decode::UtilityItem::Blob {
                offset: _,
                length: _,
                id,
            } => id,
            _ => 0,
        }
    );
    assert_eq!(
        22,
        match v[v.len() - 3] {
            decode::UtilityItem::Blob {
                offset: _,
                length: _,
                id,
            } => id,
            _ => 0,
        }
    );
}

#[test]
fn strs() {
    let c: conf::Conf<25> = conf::Conf::new(SECTION);
    assert_eq!(c.get_value_by_key(b"S0").unwrap(), b"A");
    assert_eq!(c.get_value_by_key(b"S1").unwrap(), b"BB");
    assert_eq!(c.get_value_by_key(b"URL0").unwrap(), b"http://web.local/");
}

#[test]
fn strs_n() {
    let c: conf::Conf<25> = conf::Conf::new(SECTION);
    assert_eq!(c.get_value_by_key_n(b"S", 0).unwrap(), b"A");
    assert_eq!(c.get_value_by_key_n(b"S", 1).unwrap(), b"BB");
    assert_eq!(c.get_value_by_key_n(b"URL", 0).unwrap(), b"http://web.local/");
}

#[test]
fn mapping() {
    let c: conf::Conf<25> = conf::Conf::new(SECTION);
    let dfu_prefix: [&str; 1] = [0u8,].map(|i| {
        if let Some(url) = c.get_value_by_key_n(b"URL", i) {
            println!("dfu {} {:?}", i, &url);
            str::from_utf8(url).expect("utf8")
        } else {
            ""
        }
    });
    assert_eq!(dfu_prefix, ["http://web.local/"]);
}
