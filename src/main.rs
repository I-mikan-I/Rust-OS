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
pub extern "C" fn kinit() {
    uart::initialize();
    println!("uart initialized");

    let mut mm = Pmem::init();
    let mut kmem = Kmem::init(&mut mm);
    kmem.init_mmu();
    kmem.init_trap_memory(&mut mm);
    kmem.id_map_kernel(&mut mm);
    let root_u: *mut Table = kmem.get_root();

    println!("\nALLOCATIONS:\n{}", mm);
    #[cfg(debug_assertions)]
    {
        let p = 0x10000005_usize;
        let m = Table::virt_to_phys(kmem.get_root(), p as *const u8).unwrap_or(0);
        assert_eq!(p, m);
    }
    kmem::GA.0.replace(Some(kmem));
    unsafe {
        MM = Some(mm);
        KERNEL_TABLE = root_u as usize;
    }
}

static mut MM: Option<Pmem> = None;
#[no_mangle]
pub extern "C" fn kmain() {
    println!("This is my operating system!");

    println!("triggering page fault...");
    unsafe {
        (0x0 as *mut u64).write_volatile(0);
    }

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
mod cpu;
mod kmem;
mod page;
mod trap;
mod uart;
