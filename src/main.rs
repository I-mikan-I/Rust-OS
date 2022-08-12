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

#[no_mangle]
pub extern "C" fn kmain() {
    uart::initialize();

    let mut mm = page::Pmem::init();
    let _ = mm.alloc(2);
    let _ = mm.alloc(64);
    let c = mm.alloc(3);
    println!("{}", mm);
    println!("freeing 3 pages...");
    mm.dealloc(c);
    println!("{}", mm);
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
mod page;
mod uart;