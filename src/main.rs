#![warn(
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    rust_2021_compatibility,
    noop_method_call
)]
#![allow(trivial_casts)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(int_roundings)]
use crate::kmem::Kmem;
use crate::page::{Pmem, Table, PAGE_SIZE};
use core::arch::asm;
use core::cell::{RefCell, RefMut};

#[macro_export]
macro_rules! print {
    ($($args:tt)+) => {
        {
            use core::fmt::Write;
            let _ = write!($crate::uart::get_uart(), $($args)+);
        }
    };
}

#[macro_export]
macro_rules! println {
    () => ({
        print!("\r\n")
    });
    ($fmt:expr) => {
        print!(concat!($fmt, "\r\n"))
    };
    ($fmt:expr, $($args:tt)+) => {
        print!(concat!($fmt, "\r\n"), $($args)+)
    };
}

#[no_mangle]
pub extern "C" fn eh_personality() {}
#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    print!("Aborting: ");
    if let Some(p) = info.location() {
        println!(
            "line {}, file {}: {}",
            p.line(),
            p.file(),
            info.message().unwrap()
        );
    } else {
        println!("no information available.");
    }
    abort();
}

#[no_mangle]
pub extern "C" fn abort() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

extern "C" {
    static mut KERNEL_TABLE: usize;
    fn switch_to_user(frame: usize, mepc: usize, satp: usize) -> !;
}

#[no_mangle]
pub extern "C" fn kinit() {
    uart::initialize();
    println!("uart initialized");

    let mut mm = Pmem::init();
    let mut kmem = Kmem::init(&mut mm);
    //kmem.init_mmu();
    kmem.init_trap_memory(&mut mm);
    kmem.id_map_kernel(&mut mm);
    let root_u: *mut Table = kmem.get_root();

    println!("\nALLOCATIONS:\n{}", mm);
    #[cfg(debug_assertions)]
    {
        let p = 0x800060a8_usize;
        let m = Table::virt_to_phys(kmem.get_root(), p as *const u8).unwrap_or(0);
        assert_eq!(p, m);
    }
    kmem::GA.0.replace(Some(kmem));
    unsafe {
        MM = Some(RefCell::new(mm));
        KERNEL_TABLE = root_u as usize;
    }
    sched::init();

    trap::plic::set_threshold(0);
    trap::plic::enable_interrupt(10);
    trap::plic::set_priority(10, 1);

    let (frame, mepc, satp) = sched::schedule();
    assert!(!frame.is_null(), "no user process");

    unsafe {
        // enable timer
        let mtimecmp = 0x0200_4000 as *mut u64;
        let mtime = 0x0200_bff8 as *const u64;
        mtimecmp.write_volatile(mtime.read_volatile() + 10_000_000);

        // user mode
        switch_to_user(frame as usize, mepc, satp);
    }
}

static mut MM: Option<RefCell<Pmem>> = None;

pub fn get_mm() -> RefMut<'static, Pmem> {
    unsafe { MM.as_mut().unwrap().borrow_mut() }
}

mod assembly;
mod cpu;
mod kmem;
mod page;
mod process;
mod sched;
mod syscall;
mod trap;
mod uart;
