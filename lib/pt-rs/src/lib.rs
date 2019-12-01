#[macro_use]
extern crate bitflags;

use std::boxed::Box;
use std::collections::BTreeMap;
use std::io::Error;
use std::mem;
use std::ops::Index;
use std::slice;

use memmap::MmapMut;

bitflags! {
    pub struct Prot : u32 {
        const R = 1 << 2;
        const W = 1 << 1;
        const X = 1 << 0;
    }
}


const fn pml4_index(vaddr: u64) -> usize {
    vaddr as usize >> (12 + (9 * 3)) & 0b1_1111_1111
}

const fn pdpt_index(vaddr: u64) -> usize {
    vaddr as usize >> (12 + (9 * 2)) & 0b1_1111_1111
}

const fn pd_index(vaddr: u64) -> usize {
    vaddr as usize >> (12 + (9 * 1)) & 0b1_1111_1111
}

const fn pt_index(vaddr: u64) -> usize {
    vaddr as usize >> (12 + (9 * 0)) & 0b1_1111_1111
}

const fn page_offset(vaddr: u64) -> usize {
    vaddr as usize & 0xfff
}

fn commit_next(base: &mut u64) -> u64 {
    let r = *base;
    *base += 0x1000;
    r
}

//
// PTE
//
bitflags! {
    pub struct PteFlags : u64 {
        const Present = 1 << 0;
        const Writable = 1 << 1;
        const User = 1 << 2;
        const WriteThrough = 1 << 3;
        const CacheDisabled = 1 << 4;
        const Accessed = 1 << 5;
        const Dirty = 1 << 6;
        const AttributeTable = 1 << 7;
        const Global = 1 << 8;
        const NX = 1 << 63;
    }
}

pub struct Pte {
    paddr: u64,
    flags: PteFlags,
}

impl Pte {
    fn flags(&self) -> PteFlags {
        self.flags
    }

    fn set_flags(&mut self, f: PteFlags) {
        self.flags.remove(PteFlags::all());
        self.flags.insert(f)
    }

    fn paddr(&self) -> u64 {
        self.paddr
    }

    fn set_paddr(&mut self, paddr: u64) {
        self.paddr = paddr & !0xfff;
    }
}

impl Default for Pte {
    fn default() -> Self {
        Self {
            paddr: 0,
            flags: PteFlags::empty(),
        }
    }
}

//
// Pt
//
struct IterPt<'a> {
    inner: &'a Pt,
    pos: usize,
}

impl<'a> Iterator for IterPt<'a> {
    type Item = Option<&'a Box<Pte>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.inner.pte.len() {
            None
        } else {
            self.pos += 1;
            Some(self.inner.pte[self.pos - 1].as_ref())
        }
    }
}

bitflags! {
    pub struct PtFlags : u64 {
        const Present = 1 << 0;
        const Writable = 1 << 1;
        const User = 1 << 2;
        const WriteThrough = 1 << 3;
        const CacheDisabled = 1 << 4;
        const Accessed = 1 << 5;
        const Size = 1 << 7;
        const NX = 1 << 63;
    }
}

pub struct Pt {
    pte: [Option<Box<Pte>>; 512],
    flags: PtFlags,
}

impl Pt {
    fn flags(&self) -> PtFlags {
        self.flags
    }

    fn set_flags(&mut self, f: PtFlags) {
        self.flags.remove(PtFlags::all());
        self.flags.insert(f)
    }

    fn set_pte(&mut self, vaddr: u64, v: Pte) -> &mut Pte {
        self.pte[pt_index(vaddr)] = Some(Box::new(v));
        self.pte[pt_index(vaddr)].as_mut().unwrap()
    }

    fn pte(&self, vaddr: u64) -> Option<&Box<Pte>> {
        self.pte[pt_index(vaddr)].as_ref()
    }

    fn pte_mut(&mut self, vaddr: u64) -> Option<&mut Box<Pte>> {
        self.pte[pt_index(vaddr)].as_mut()
    }

    fn iter(&self) -> IterPt {
        IterPt {
            inner: self,
            pos: 0,
        }
    }
}

impl Default for Pt {
    fn default() -> Self {
        let pte: [Option<Box<Pte>>; 512] = unsafe { std::mem::zeroed() };

        Self {
            pte,
            flags: PtFlags::empty(),
        }
    }
}

impl Index<usize> for Pt {
    type Output = Option<Box<Pte>>;

    fn index(&self, idx: usize) -> &Option<Box<Pte>> {
        &self.pte[idx]
    }
}

//
// Pd
//
struct IterPd<'a> {
    inner: &'a Pd,
    pos: usize,
}

impl<'a> Iterator for IterPd<'a> {
    type Item = Option<&'a Box<Pt>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.inner.pt.len() {
            None
        } else {
            self.pos += 1;
            Some(self.inner.pt[self.pos - 1].as_ref())
        }
    }
}

bitflags! {
    pub struct PdFlags : u64 {
        const Present = 1 << 0;
        const Writable = 1 << 1;
        const User = 1 << 2;
        const WriteThrough = 1 << 3;
        const CacheDisabled = 1 << 4;
        const Accessed = 1 << 5;
        const Size = 1 << 7;
        const NX = 1 << 63;
    }
}

pub struct Pd {
    pt: [Option<Box<Pt>>; 512],
    flags: PdFlags,
}

impl Pd {
    fn flags(&self) -> PdFlags {
        self.flags
    }

    fn set_flags(&mut self, f: PdFlags) {
        self.flags.remove(PdFlags::all());
        self.flags.insert(f)
    }

    fn set_pt(&mut self, vaddr: u64, v: Pt) -> &mut Pt {
        self.pt[pd_index(vaddr)] = Some(Box::new(v));
        self.pt[pd_index(vaddr)].as_mut().unwrap()
    }

    fn pt(&self, vaddr: u64) -> Option<&Box<Pt>> {
        self.pt[pd_index(vaddr)].as_ref()
    }

    fn pt_mut(&mut self, vaddr: u64) -> Option<&mut Box<Pt>> {
        self.pt[pd_index(vaddr)].as_mut()
    }

    fn iter(&self) -> IterPd {
        IterPd {
            inner: self,
            pos: 0,
        }
    }
}

impl Default for Pd {
    fn default() -> Self {
        let pt: [Option<Box<Pt>>; 512] = unsafe { std::mem::zeroed() };

        Self {
            pt,
            flags: PdFlags::empty(),
        }
    }
}

impl Index<usize> for Pd {
    type Output = Option<Box<Pt>>;

    fn index(&self, idx: usize) -> &Option<Box<Pt>> {
        &self.pt[idx]
    }
}

//
// PDPT
//
struct IterPdpt<'a> {
    inner: &'a Pdpt,
    pos: usize,
}

impl<'a> Iterator for IterPdpt<'a> {
    type Item = Option<&'a Box<Pd>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.inner.pd.len() {
            None
        } else {
            self.pos += 1;
            Some(self.inner.pd[self.pos - 1].as_ref())
        }
    }
}

bitflags! {
    pub struct PdptFlags : u64 {
        const Present = 1 << 0;
        const Writable = 1 << 1;
        const User = 1 << 2;
        const WriteThrough = 1 << 3;
        const CacheDisabled = 1 << 4;
        const Accessed = 1 << 5;
        const Size = 1 << 7;
        const NX = 1 << 63;
    }
}

pub struct Pdpt {
    pd: [Option<Box<Pd>>; 512],
    flags: PdptFlags,
}

impl Pdpt {
    fn flags(&self) -> PdptFlags {
        self.flags
    }

    fn set_flags(&mut self, f: PdptFlags) {
        self.flags.remove(PdptFlags::all());
        self.flags.insert(f)
    }

    fn set_pd(&mut self, vaddr: u64, v: Pd) -> &mut Pd {
        self.pd[pdpt_index(vaddr)] = Some(Box::new(v));
        self.pd[pdpt_index(vaddr)].as_mut().unwrap()
    }

    fn pd(&self, vaddr: u64) -> Option<&Box<Pd>> {
        self.pd[pdpt_index(vaddr)].as_ref()
    }

    fn pd_mut(&mut self, vaddr: u64) -> Option<&mut Box<Pd>> {
        self.pd[pdpt_index(vaddr)].as_mut()
    }

    fn iter(&self) -> IterPdpt {
        IterPdpt {
            inner: self,
            pos: 0,
        }
    }
}

impl Default for Pdpt {
    fn default() -> Self {
        let pd: [Option<Box<Pd>>; 512] = unsafe { std::mem::zeroed() };
        Self {
            pd,
            flags: PdptFlags::empty(),
        }
    }
}

impl Index<usize> for Pdpt {
    type Output = Option<Box<Pd>>;

    fn index(&self, idx: usize) -> &Option<Box<Pd>> {
        &self.pd[idx]
    }
}

//
// Page Table
//
bitflags! {
    pub struct Flags : u64 {
        const Present = 1 << 0;
        const Writable = 1 << 1;
        const User = 1 << 2;
        const WriteThrough = 1 << 3;
        const CacheDisabled = 1 << 4;
        const Accessed = 1 << 5;
        const Dirty = 1 << 6;
        const NX = 1 << 63;
    }
}

pub struct IterPageTable<'a> {
    inner: &'a PageTable,
    pos: usize,
}

impl<'a> Iterator for IterPageTable<'a> {
    type Item = Option<&'a Box<Pdpt>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.inner.pml4.len() {
            None
        } else {
            self.pos += 1;
            Some(self.inner.pml4[self.pos - 1].as_ref())
        }
    }
}

pub struct PageTable {
    pml4: [Option<Box<Pdpt>>; 512],
}

impl PageTable {
    pub fn translate(&self, vaddr: u64, p: Prot) -> Option<u64> {
        // pdpt
        let pdpt = self.pdpt(vaddr)?;
        if !pdpt.flags().contains(PdptFlags::Present) {
            return None;
        }
        if p.contains(Prot::W) && !pdpt.flags().contains(PdptFlags::Writable) {
            return None;
        }
        if p.contains(Prot::X) && pdpt.flags().contains(PdptFlags::NX) {
            return None;
        }

        // pd
        let pd = self.pd(vaddr)?;

        if !pd.flags().contains(PdFlags::Present) {
            return None;
        }
        if p.contains(Prot::W) && !pd.flags().contains(PdFlags::Writable) {
            return None;
        }
        if p.contains(Prot::X) && pd.flags().contains(PdFlags::NX) {
            return None;
        }

        // pt
        let pt = self.pt(vaddr)?;

        if !pt.flags().contains(PtFlags::Present) {
            return None;
        }
        if p.contains(Prot::W) && !pt.flags().contains(PtFlags::Writable) {
            return None;
        }
        if p.contains(Prot::X) && pt.flags().contains(PtFlags::NX) {
            return None;
        }

        // pte
        let pte = self.pte(vaddr)?;

        if !pte.flags().contains(PteFlags::Present) {
            return None;
        }
        if p.contains(Prot::W) && !pte.flags().contains(PteFlags::Writable) {
            return None;
        }
        if p.contains(Prot::X) && pte.flags().contains(PteFlags::NX) {
            return None;
        }

        Some(pte.paddr() + page_offset(vaddr) as u64)
    }

    pub fn insert(&mut self, vaddr: u64, paddr: u64, f: Flags) {
        let pdpt = match self.pdpt_mut(vaddr) {
            None => self.set_pdpt(vaddr, Pdpt::default()),
            Some(x) => x,
        };

        pdpt.set_flags(pdpt.flags() | PdptFlags::from_bits_truncate(f.bits()));

        let pd = match pdpt.pd_mut(vaddr) {
            None => pdpt.set_pd(vaddr, Pd::default()),
            Some(x) => x,
        };

        pd.set_flags(pd.flags() | PdFlags::from_bits_truncate(f.bits()));

        let pt = match pd.pt_mut(vaddr) {
            None => pd.set_pt(vaddr, Pt::default()),
            Some(x) => x,
        };

        pt.set_flags(pt.flags() | PtFlags::from_bits_truncate(f.bits()));

        let pte = match pt.pte_mut(vaddr) {
            None => pt.set_pte(vaddr, Pte::default()),
            Some(x) => x,
        };

        pte.set_flags(pte.flags() | PteFlags::from_bits_truncate(f.bits()));
        pte.set_paddr(paddr);
    }

    pub fn commit(self) -> Result<(u64, BTreeMap<u64, MmapMut>), Error> {
        let mut r = BTreeMap::new();
        let mut base = 0;

        let mut pml4_backing = MmapMut::map_anon(0x1000)?;

        let pml4_data: &mut [u64] = unsafe {
            slice::from_raw_parts_mut(
                pml4_backing.as_mut_ptr() as *mut _,
                0x1000 / mem::size_of::<u64>(),
            )
        };

        let pml4_paddr = commit_next(&mut base);

        for (pp, pdpt) in self
            .iter()
            .enumerate()
            .filter_map(|(pp, pdpt)| pdpt.map(|x| (pp, x)))
        {
            let mut pdpt_backing = MmapMut::map_anon(0x1000)?;

            let pdpt_data: &mut [u64] = unsafe {
                slice::from_raw_parts_mut(
                    pdpt_backing.as_mut_ptr() as *mut _,
                    0x1000 / mem::size_of::<u64>(),
                )
            };

            let pdpt_paddr = commit_next(&mut base);

            for (qq, pd) in pdpt
                .iter()
                .enumerate()
                .filter_map(|(qq, pd)| pd.map(|x| (qq, x)))
            {
                let mut pd_backing = MmapMut::map_anon(0x1000)?;

                let pd_data: &mut [u64] = unsafe {
                    slice::from_raw_parts_mut(
                        pd_backing.as_mut_ptr() as *mut _,
                        0x1000 / mem::size_of::<u64>(),
                    )
                };

                let pd_paddr = commit_next(&mut base);

                for (rr, pt) in pd
                    .iter()
                    .enumerate()
                    .filter_map(|(rr, pt)| pt.map(|x| (rr, x)))
                {
                    let mut pt_backing = MmapMut::map_anon(0x1000)?;
                    let pt_paddr = commit_next(&mut base);

                    let pt_data: &mut [u64] = unsafe {
                        slice::from_raw_parts_mut(
                            pt_backing.as_mut_ptr() as *mut _,
                            0x1000 / mem::size_of::<u64>(),
                        )
                    };

                    for (ss, pte) in pt
                        .iter()
                        .enumerate()
                        .filter_map(|(ss, pte)| pte.map(|x| (ss, x)))
                    {
                        pt_data[ss] = pte.paddr() | pte.flags().bits();
                    }

                    pd_data[rr] = pt_paddr | pt.flags().bits();
                    r.insert(pt_paddr, pt_backing);
                }

                pdpt_data[qq] = pd_paddr | pd.flags().bits();
                r.insert(pd_paddr, pd_backing);
            }

            pml4_data[pp] = pdpt_paddr | pdpt.flags().bits();
            r.insert(pdpt_paddr, pdpt_backing);
        }

        r.insert(pml4_paddr, pml4_backing);

        Ok((pml4_paddr, r))
    }

    pub fn set_pdpt(&mut self, vaddr: u64, v: Pdpt) -> &mut Pdpt {
        self.pml4[pml4_index(vaddr)] = Some(Box::new(v));
        self.pml4[pml4_index(vaddr)].as_mut().unwrap()
    }

    pub fn pdpt(&self, vaddr: u64) -> Option<&Box<Pdpt>> {
        self.pml4[pml4_index(vaddr)].as_ref()
    }

    pub fn pdpt_mut(&mut self, vaddr: u64) -> Option<&mut Box<Pdpt>> {
        self.pml4[pml4_index(vaddr)].as_mut()
    }

    pub fn pd(&self, vaddr: u64) -> Option<&Box<Pd>> {
        let pdpt = self.pdpt(vaddr)?;
        pdpt.pd(vaddr)
    }

    pub fn pd_mut(&mut self, vaddr: u64) -> Option<&mut Box<Pd>> {
        let pdpt = self.pdpt_mut(vaddr)?;
        pdpt.pd_mut(vaddr)
    }

    pub fn pt(&self, vaddr: u64) -> Option<&Box<Pt>> {
        let pd = self.pd(vaddr)?;
        pd.pt(vaddr)
    }

    pub fn pt_mut(&mut self, vaddr: u64) -> Option<&mut Box<Pt>> {
        let pd = self.pd_mut(vaddr)?;
        pd.pt_mut(vaddr)
    }

    pub fn pte(&self, vaddr: u64) -> Option<&Box<Pte>> {
        let pt = self.pt(vaddr)?;
        pt.pte(vaddr)
    }

    pub fn pte_mut(&mut self, vaddr: u64) -> Option<&mut Box<Pte>> {
        let pt = self.pt_mut(vaddr)?;
        pt.pte_mut(vaddr)
    }

    pub fn iter(&self) -> IterPageTable {
        IterPageTable {
            inner: self,
            pos: 0,
        }
    }
}

impl Index<usize> for PageTable {
    type Output = Option<Box<Pdpt>>;

    fn index(&self, idx: usize) -> &Option<Box<Pdpt>> {
        &self.pml4[idx]
    }
}

impl Default for PageTable {
    fn default() -> Self {
        let pml4: [Option<Box<Pdpt>>; 512] = unsafe { std::mem::zeroed() };

        Self { pml4 }
    }
}
