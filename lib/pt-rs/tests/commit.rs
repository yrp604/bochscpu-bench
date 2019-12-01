extern crate pt;

use pt::{PageTable, Flags};

#[test]
fn commit() {
    let mut x = PageTable::default();

    x.insert(0x4141_0000, 0x8181_0000, Flags::Present);

    let (pml4, y) = x.commit().unwrap();

    assert_eq!(pml4, 0);
    assert_eq!(y.len(), 4);
    eprintln!("{:x?}", y);
}

#[test]
fn commit_one() {
    let mut x = PageTable::default();

    x.insert(0x4141_0000, 0x8181_0000, Flags::Present);
    x.insert(0x4141_1000, 0x8181_1000, Flags::Present);

    let (pml4, y) = x.commit().unwrap();

    assert_eq!(pml4, 0);
    assert_eq!(y.len(), 4);
    eprintln!("{:x?}", y);
}
