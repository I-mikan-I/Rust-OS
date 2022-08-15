#![warn(
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    rust_2021_compatibility,
    trivial_casts,
    noop_method_call
)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(int_roundings)]
use crate::kmem::Kmem;
use crate::page::{Pmem, Table};
use core::arch::asm;

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
}

#[no_mangle]
pub extern "C" fn kinit() -> usize {
    uart::initialize();
    println!("uart initialized");

    let mut mm = Pmem::init();
    let mut kmem = Kmem::init(&mut mm);
    let head = kmem.get_head() as usize;
    let pages = kmem.get_allocations();
    page::id_map(kmem.get_root(), &mut mm, head, pages);
    let root_u: *mut Table = kmem.get_root();
    #[cfg(debug_assertions)]
    {
        let p = 0x10000005_usize;
        let m = Table::virt_to_phys(kmem.get_root(), p as *const u8).unwrap_or(0);
        assert_eq!(p, m);
    }
    println!("\nALLOCATIONS:\n{}", mm);
    #[allow(trivial_casts)]
    unsafe {
        MM = Some(mm);
        KERNEL_TABLE = root_u as usize;
        KMEM = Some(kmem);
    }
    (root_u as usize >> 12) | (8 << 60)
}

static mut MM: Option<Pmem> = None;
static mut KMEM: Option<Kmem> = None;
#[no_mangle]
pub extern "C" fn kmain() {
    println!("This is my operating system!");
    println!("Typing...");
    loop {
        if let Some(c) = uart::get_uart().get() {
            match c {
                8 => {
                    print!("{} {}", 8_u8 as char, 8_u8 as char);
                }
                10 | 13 => {
                    println!();
                }
                0x1b => {
                    if let Some(91) = uart::get_uart().get() {
                        if let Some(b) = uart::get_uart().get() {
                            match b as char {
                                'A' => {
                                    println!("That's the up arrow!");
                                }
                                'B' => {
                                    println!("That's the down arrow!");
                                }
                                'C' => {
                                    println!("That's the right arrow!");
                                }
                                'D' => {
                                    println!("That's the left arrow!");
                                }
                                _ => {
                                    println!("That's something else.....");
                                }
                            }
                        }
                    }
                }
                _ => {
                    print!("{}", c as char);
                }
            }
        }
    }
}

mod assembly;
mod kmem;
mod page;
mod uart;
