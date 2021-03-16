use std::io::{self, Write};

use pt::{Flags, PageTable};

fn main() {
    let mut pt = PageTable::default();

    pt.insert(0x4141_0000, 0x8181_0000, Flags::User | Flags::Present);
    pt.insert(
        0x1234_5000,
        0x6789_0000,
        Flags::User | Flags::Present | Flags::Writable,
    );


    let (pml4, pagetable) = pt.commit().unwrap();
    assert_eq!(pml4, 0);

    let mut last = 0;
    for page in pagetable {
        assert_eq!(page.0, last);

        io::stdout().write_all(&page.1).unwrap();

        last += 0x1000;
    }
}
