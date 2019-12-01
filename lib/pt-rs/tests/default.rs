extern crate pt;

use pt::PageTable;

#[test]
fn default() {
    let _x = PageTable::default();
}
