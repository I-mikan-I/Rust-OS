use crate::process::Process;
extern crate alloc;
use crate::cpu::TrapFrame;
use crate::process::ProcessState::Running;
use crate::{cpu, Table};
use alloc::collections::VecDeque;
use core::arch::asm;

static mut SCHED: Option<Scheduler> = None;

pub fn init() {
    unsafe { SCHED = Some(Scheduler::init()) }
}

pub fn schedule() -> (*mut TrapFrame, usize, usize) {
    let scheduler = unsafe { SCHED.as_mut().unwrap() };
    scheduler.procs.rotate_left(1);
    let mut mepc = 0;
    let mut satp = 0;
    let mut pid = 0;
    let mut frame = None;
    match scheduler.procs.front_mut() {
        Some(p) if p.get_state() == Running => {
            pid = p.get_pid();
            satp = (p.get_table() as *const Table) as usize;
            mepc = p.get_pc();
            frame = Some(p.get_frame());
        }
        _ => {}
    }
    println!("Scheduling {}\n{:?}", pid, (0, mepc, satp));
    if let Some(frame) = frame {
        if satp != 0 {
            (
                frame as *mut TrapFrame,
                mepc,
                cpu::build_satp(cpu::SatpMode::Sv39, pid, satp),
            )
        } else {
            (frame as *mut TrapFrame, mepc, 0)
        }
    } else {
        (core::ptr::null_mut(), 0, 0)
    }
}

struct Scheduler {
    procs: VecDeque<Process>,
}

impl Scheduler {
    pub fn init() -> Scheduler {
        let mut res = Self {
            procs: VecDeque::with_capacity(15),
        };
        res.procs.push_back(Process::new(init_process));
        res
    }
}

fn init_process() {
    let mut i: usize = 0;
    loop {
        i += 1;
        if i > 70_000_000 {
            unsafe {
                asm!("li a0, 0", "ecall", out("a0") _);
            }
            i = 0;
        }
    }
}
