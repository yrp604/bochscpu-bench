extern crate pt;

use pt::{Flags, PageTable, Prot};

#[test]
fn translate_none() {
    let mut x = PageTable::default();

    x.insert(0x7171_0000, 0, Flags::NX | Flags::Present);
    assert_eq!(x.translate(0x8181_0000, Prot::R), None);
}

#[test]
fn translate_read() {
    let mut x = PageTable::default();

    x.insert(0x4141_4000, 0, Flags::NX | Flags::Present);

    assert_eq!(x.translate(0x4141_4000, Prot::R).unwrap(), 0);
    assert_eq!(x.translate(0x4141_4000, Prot::W), None);
    assert_eq!(x.translate(0x4141_4000, Prot::X), None);
}

#[test]
fn translate_write() {
    let mut x = PageTable::default();

    x.insert(0x8_1818_1000, 0, Flags::NX | Flags::Present | Flags::Writable);

    assert_eq!(x.translate(0x8_1818_1000, Prot::R).unwrap(), 0);
    assert_eq!(x.translate(0x8_1818_1000, Prot::W).unwrap(), 0);
    assert_eq!(x.translate(0x8_1818_1000, Prot::X), None);
}

#[test]
fn translate_execute() {
    let mut x = PageTable::default();

    x.insert(0x4141_4000, 0, Flags::Present);

    assert_eq!(x.translate(0x4141_4000, Prot::R).unwrap(), 0);
    assert_eq!(x.translate(0x4141_4000, Prot::W), None);
    assert_eq!(x.translate(0x4141_4000, Prot::X).unwrap(), 0);
}

#[test]
fn translate_all() {
    let mut x = PageTable::default();

    x.insert(0x4141_4000, 0, Flags::Present | Flags::Writable);

    assert_eq!(x.translate(0x4141_4000, Prot::R).unwrap(), 0);
    assert_eq!(x.translate(0x4141_4000, Prot::W).unwrap(), 0);
    assert_eq!(x.translate(0x4141_4000, Prot::X).unwrap(), 0);
}
