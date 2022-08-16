use crate::cpu::TrapFrame;

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
                let timecmp = 0x02004000 as *mut u64;
                let time = 0x0200bff8 as *const u64;
                timecmp.write_volatile(time.read_volatile() + 10_000_000);
            },
            11 => {
                println!("Machine external interrupt CPU #{}", hart);
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
                epc += 4;
            }
            9 => {
                println!("E-call from Supervisor mode! CPU#{} -> 0x{:08x}", hart, epc);
                epc += 4;
            }
            11 => {
                panic!("E-call from Machine mode! CPU#{} -> 0x{:08x}", hart, epc);
            }
            12 => {
                println!(
                    "Instruction page fault CPU#{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
                epc += 4;
            }
            13 => {
                println!(
                    "Load page fault CPU#{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
                epc += 4;
            }
            15 => {
                println!(
                    "Store page fault CPU#{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
                epc += 4;
            }
            _ => {
                panic!("unhandled sync trap CPU#{} -> {}", hart, cause);
            }
        }
    }
    epc
}
