use crate::cpu::TrapFrame;
use crate::sched::schedule;
use crate::syscall::do_syscall;
use crate::{switch_to_user, uart};

#[no_mangle]
extern "C" fn m_trap(
    mut epc: usize,
    tval: usize,
    cause: usize,
    hart: usize,
    status: usize,
    frame: &mut TrapFrame,
) -> usize {
    let is_async = cause & 1 << 63 != 0;
    let cause = cause & 0xfff;
    if is_async {
        match cause {
            3 => {
                println!("Machine software interrupt CPU#{}", hart);
            }
            7 => unsafe {
                println!("Timer interrupt...");
                let (frame, mepc, satp) = schedule();
                let timecmp = 0x02004000 as *mut u64;
                let time = 0x0200bff8 as *const u64;
                timecmp.write_volatile(time.read_volatile() + 10_000_000);
                switch_to_user(frame as usize, mepc, satp);
            },
            11 => {
                if let Some(interrupt) = plic::claim() {
                    match interrupt.get() {
                        10 => {
                            // UART
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
                        _ => {
                            println!("NON-UART async interrupt");
                        }
                    }
                    plic::complete(interrupt.get());
                }
            }
            _ => {
                panic!("Unhandled async trap CPU#{} -> {}", hart, cause);
            }
        }
    } else {
        match cause {
            2 => {
                panic!(
                    "illegal instruction CPU#{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
            }
            8 => {
                println!("E-call from User mode! CPU#{} -> 0x{:08x}", hart, epc);
                epc = do_syscall(epc, frame);
            }
            9 => {
                println!("E-call from Supervisor mode! CPU#{} -> 0x{:08x}", hart, epc);
                epc = do_syscall(epc, frame);
            }
            11 => {
                panic!("E-call from Machine mode! CPU#{} -> 0x{:08x}", hart, epc);
            }
            12 => {
                panic!(
                    "Instruction page fault CPU#{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
            }
            13 => {
                panic!(
                    "Load page fault CPU#{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
            }
            15 => {
                panic!(
                    "Store page fault CPU#{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
            }
            _ => {
                panic!("unhandled sync trap CPU#{} -> {}", hart, cause);
            }
        }
    }
    epc
}

pub mod plic {

    const PLIC_BASE: usize = 0xc000000;
    const ENABLE_0_31: usize = 0x2000;
    const HART_0_M_THRESH: usize = 0x200000;
    const PLIC_CLAIM: usize = 0x200004;
    pub fn get_addresses() -> &'static [usize] {
        &[
            PLIC_BASE + ENABLE_0_31,
            PLIC_BASE + HART_0_M_THRESH,
            PLIC_BASE + PLIC_CLAIM,
            PLIC_BASE,
        ]
    }
    pub fn enable_interrupt(id: usize) {
        assert!(id <= 31);

        let ptr = (PLIC_BASE + ENABLE_0_31) as *mut u32;
        unsafe {
            ptr.write_volatile(ptr.read_volatile() | (1_u32 << id));
        }
    }
    pub fn set_priority(id: usize, prio: u8) {
        assert!(id <= 31);
        assert!(prio <= 7);
        let ptr = PLIC_BASE as *mut u32;
        unsafe {
            ptr.add(id).write_volatile(prio as u32);
        }
    }
    pub fn set_threshold(tsh: u8) {
        let tsh = tsh & 7;
        unsafe {
            ((PLIC_BASE + HART_0_M_THRESH) as *mut u32).write_volatile(tsh as u32);
        }
    }
    pub fn claim() -> Option<core::num::NonZeroU32> {
        let claimed = unsafe { ((PLIC_BASE + PLIC_CLAIM) as *mut u32).read_volatile() };
        core::num::NonZeroU32::new(claimed)
    }
    pub fn complete(id: u32) {
        unsafe { ((PLIC_BASE + PLIC_CLAIM) as *mut u32).write_volatile(id) }
    }
}
