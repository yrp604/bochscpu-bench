#![feature(test)]
extern crate test;

use rand::Rng;
use rand::seq::SliceRandom;
use test::Bencher;

use mapping::Prot;

use pt::{Flags, PageTable};

const ENTRIES: usize = 2400;

#[bench]
fn bench_translation(b: &mut Bencher) {
    let mut addrs = vec![];
    let mut pt = PageTable::default();

    let mut rng = rand::thread_rng();

    for _ in 0..ENTRIES {
        let vaddr = rng.gen::<u64>() & !0xfff;
        let paddr = rng.gen::<u64>() & !0xfff;

        pt.insert(vaddr, paddr, Flags::Present);
        addrs.push((vaddr, paddr));
    }

    b.iter(|| {
        let (v, p) = addrs.choose(&mut rng).unwrap();
        assert_eq!(pt.translate(*v, Prot::R).unwrap(), *p);
    });
}
