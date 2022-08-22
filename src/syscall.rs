use crate::cpu::TrapFrame;

pub fn do_syscall(mepc: usize, frame: &mut TrapFrame) -> usize {
    let syscall_num = frame.regs[10];
    match syscall_num {
        0 => {
            println!("exit system call");
            mepc + 4
        }
        _ => {
            println!("unknown system call");
            mepc + 4
        }
    }
}
