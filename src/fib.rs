use std::mem;
use std::time::Instant;

use memmap::MmapMut;
use pt::{Flags, PageTable};

use bochscpu::cpu::{Cpu, RunState, Seg};
use bochscpu::hook::{self, MemAccess};
use bochscpu::mem as guest_mem;

static mut INS: usize = 0;
static mut READS: usize = 0;
static mut WRITES: usize = 0;

static CODE: &'static [u8] = include_bytes!("../asm/fib.o");
//static CODE: &'static [u8] = b"\xcc";

unsafe fn fib() {
    // first let's set up our address space, we need two pages:
    // - one for our code to live on
    // - one for our stack

    let mut pt = PageTable::default();

    println!("creating a mapping from gva 0x41410000 to gpa 0x81810000 for the text...");
    pt.insert(0x4141_0000, 0x8181_0000, Flags::User | Flags::Present);
    println!("creating a mapping from gva 0x12345000 to gpa 0x67890000 for the stack...");
    pt.insert(
        0x1234_5000,
        0x6789_0000,
        Flags::User | Flags::Present | Flags::Writable,
    );

    // we need to map in our phys mem for the mappings we created above
    let mut code_backing = MmapMut::map_anon(0x1000).unwrap();
    let mut stack_backing = MmapMut::map_anon(0x1000).unwrap();
    println!(
        "mapping gpa 0x81810000 to hva {:p}...",
        code_backing.as_ptr()
    );
    guest_mem::add_page(0x8181_0000, code_backing.as_mut_ptr());
    println!(
        "mapping gpa 0x12345000 to hva {:p}...",
        stack_backing.as_ptr()
    );
    guest_mem::add_page(0x6789_0000, stack_backing.as_mut_ptr());

    // serialize our page tables
    let (pml4, mut pagetable) = pt.commit().unwrap();
    println!("page table serialized, base @ {:#x}", pml4);

    // write our page tables into our guest
    for (gpa, hva) in pagetable.iter_mut() {
        println!(
            "mapping page table gpa {:#x} to hva {:p}...",
            gpa,
            hva.as_ptr()
        );

        guest_mem::add_page(*gpa, hva.as_mut_ptr());
    }
    // if we let the mem mapping go out of scope it will be dropped and
    // the backing page unmapped, so we'll just leak it
    mem::forget(pagetable);

    println!("setting up cpu registers...");
    let c = Cpu::new(0);

    c.set_cr0(0x8005_0031);
    c.set_cr3(pml4);
    c.set_cr4(0x0017_0678);
    c.set_efer(0xd01);

    let cs = Seg {
        present: true,
        selector: 0x33,
        base: 0,
        limit: 0xffff_ffff,
        attr: 0x22fb,
    };
    c.set_cs(cs);

    let ds = Seg {
        present: true,
        selector: 0x2b,
        base: 0,
        limit: 0xffff_ffff,
        attr: 0xcf3,
    };
    c.set_ds(ds);
    c.set_ss(ds);
    c.set_es(ds);
    c.set_fs(ds);
    c.set_gs(ds);

    c.set_rip(0x4141_0000);
    c.set_rsp(0x1234_5800);

    c.print_gprs();

    // now we need to write our actual code into our guest
    println!(
        "writing {} bytes of fib code to gva 0x41410000...",
        CODE.len()
    );
    guest_mem::virt_write(c.cr3(), 0x4141_0000, CODE);

    // now we're going to set up our hooks:
    // - one to instrument instructions
    // - one to instrument memory accesses
    println!("setting up emulation hooks...");

    hook::after_execution(|cpuid, _| {
        INS += 1;
    });
    hook::lin_access(|_, _, _, _, _, access| match access {
        MemAccess::Read => READS += 1,
        MemAccess::Write => WRITES += 1,
        MemAccess::Execute => (),
        _ => panic!("bad access type in lin access hook"),
    });

    struct D();
    impl Drop for D {
        fn drop(&mut self) {
            println!("dropped");
        }
    }

    hook::exception(|cpuid, _, _| {
        let d = D();
        Cpu::from(cpuid).set_run_state(RunState::Stop);
    });

    println!("done, starting emulation...");

    // now we're off to the races
    let start = Instant::now();
    c.run();
    let end = Instant::now();

    // print stats and bail
    println!("result in rax is {:x}, {} loops", c.rax(), c.rcx());

    println!(
        "emulated {} ins with {} mem reads and {} mem writes in {:?}, {:0.2}m ips",
        INS,
        READS,
        WRITES,
        end - start,
        INS as f64 / (end - start).as_secs() as f64 / 1_000_000 as f64
    );
}

fn main() {
    //stderrlog::new().verbosity(11).init().unwrap();

    unsafe { fib() };
}
