use crate::cpu::TrapFrame;
use crate::page::IPage;
use crate::{cpu, get_mm, page, Pmem, Table, PAGE_SIZE};
use core::ops::DerefMut;

const STACK_PAGES: usize = 2;
const START_ADDR: usize = 0x2000_0000;
const STACK_ADDR: usize = 0xf_0000_0000;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum ProcessState {
    Running,
    Sleeping,
    Waiting,
    Dead,
}

pub struct Process {
    frame: TrapFrame,
    stack: IPage,
    pc: usize,
    pid: u16,
    root: *mut Table,
    state: ProcessState,
    sleep_until: usize,
}

impl Process {
    pub fn new(func: fn()) -> Self {
        static mut NEXT_PID: u16 = 0;
        let mut pm = get_mm();
        let pm = pm.deref_mut();
        let mut res = Self {
            frame: cpu::TrapFrame::zero(),
            stack: pm.alloc(STACK_PAGES),
            pc: START_ADDR | (func as usize & 0xfff),
            pid: unsafe { NEXT_PID },
            root: pm.zalloc(1).leak() as *mut Table,
            state: ProcessState::Running,
            sleep_until: 0,
        };
        unsafe { NEXT_PID += 1 };
        res.frame.regs[2] = STACK_ADDR + PAGE_SIZE * STACK_PAGES; // set sp
        let table = unsafe { &mut *res.root };
        let stack_top = res.stack.physical() as usize;

        for i in 0..STACK_PAGES {
            Table::map(
                table,
                pm,
                STACK_ADDR + i * PAGE_SIZE,
                stack_top + i * PAGE_SIZE,
                page::entry_bits::READ_WRITE | page::entry_bits::USER,
                0,
            );
        }
        let func = (func as usize) & !0xfff;
        Table::map(
            table,
            pm,
            START_ADDR,
            func,
            page::entry_bits::USER | page::entry_bits::READ_EXECUTE,
            0,
        );
        Table::map(
            table,
            pm,
            START_ADDR + 0x1000,
            func + 0x1000,
            page::entry_bits::USER | page::entry_bits::READ_EXECUTE,
            0,
        );

        res
    }
    pub fn get_frame(&mut self) -> &mut TrapFrame {
        &mut self.frame
    }
    pub fn get_pc(&self) -> usize {
        self.pc
    }
    pub fn get_table(&mut self) -> &mut Table {
        unsafe { &mut *self.root }
    }
    pub fn get_state(&self) -> ProcessState {
        self.state
    }
    pub fn get_pid(&self) -> u16 {
        self.pid
    }
    pub fn get_sleep_until(&self) -> usize {
        self.sleep_until
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let mut pm = get_mm();
        let pm = pm.deref_mut();
        let table = unsafe { &mut *self.root };
        table.unmap(pm);
        unsafe { pm.dealloc_phys(self.root as *mut u8) };
    }
}
