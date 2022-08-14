use crate::page::PageBits::{Empty, Last, Taken};
use core::cmp::max;
use core::fmt::{Display, Formatter};
use core::marker::PhantomData;
use core::mem::MaybeUninit;

// ========================= PAGES =========================

extern "C" {
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}

pub const PAGE_SIZE: usize = 1 << 12;

#[repr(u8)]
#[derive(PartialEq, Eq, Ord, PartialOrd)]
pub enum PageBits {
    Empty = 0,
    Taken = 1,
    Last = 2,
}
#[repr(transparent)]
pub struct Page {
    flags: PageBits,
}

pub struct Pmem {
    descriptors: &'static mut [Page],
    alloc_start: usize,
    _traits: PhantomData<*mut u8>,
}
#[non_exhaustive]
pub struct IPage(usize, *mut u8);

impl IPage {
    pub fn available(&self) -> bool {
        self.0 != usize::MAX
    }
    pub fn physical(&self) -> *const u8 {
        self.1
    }
}

impl Pmem {
    pub fn init() -> Pmem {
        unsafe {
            let num_pages = HEAP_SIZE / PAGE_SIZE;
            let ptr = HEAP_START as *mut MaybeUninit<Page>;
            let descriptors: &'static mut [MaybeUninit<Page>] =
                core::slice::from_raw_parts_mut(ptr, num_pages);
            for uninit in descriptors.iter_mut() {
                uninit.write(Page { flags: Empty });
            }
            let offset: isize = num_pages as isize * core::mem::size_of::<Page>() as isize;
            Pmem {
                descriptors: core::mem::transmute::<_, &'static mut [Page]>(descriptors),
                alloc_start: HEAP_START
                    + offset as usize
                    + (-(HEAP_START as isize + offset)).rem_euclid(PAGE_SIZE as isize) as usize,
                _traits: PhantomData,
            }
        }
    }
    pub fn zalloc(&mut self, pages: usize) -> IPage {
        let ip = self.alloc(pages);
        match ip {
            IPage(usize::MAX, _) => ip,
            IPage(_, page_begin) => {
                let page_begin = page_begin as *mut u8;
                unsafe {
                    page_begin.write_bytes(0, pages * PAGE_SIZE);
                }
                ip
            }
        }
    }
    pub fn alloc(&mut self, pages: usize) -> IPage {
        let num_pages = self.descriptors.len();
        let mut found = 0;
        let mut begin = usize::MAX;
        let mut physical = core::ptr::null_mut();
        for (i, p) in self.descriptors[..num_pages - pages].iter_mut().enumerate() {
            match p {
                Page { flags: Empty } => {
                    found += 1;
                }
                _ => {
                    found = 0;
                }
            }
            if found == pages {
                begin = 1 + i - found;
                p.flags = Last;
                physical = (self.alloc_start + begin * PAGE_SIZE) as *mut u8;
                assert_eq!(physical.align_offset(PAGE_SIZE), 0);
                break;
            }
        }
        for p in &mut self.descriptors[begin..][..found - 1] {
            p.flags = Taken;
        }
        IPage(begin, physical)
    }
    pub fn dealloc(&mut self, pages: IPage) {
        assert!(pages.available());
        let mut index = pages.0;
        for p in &mut self.descriptors[index..] {
            if let Empty | Last = p.flags {
                break;
            }
            p.flags = Empty;
            index += 1;
        }
        assert!(
            self.descriptors[index].flags == Last,
            "potential double-free detected"
        );
        self.descriptors[index].flags = Empty;
    }
    pub unsafe fn dealloc_phys(&mut self, phys: *mut u8) {
        assert_eq!(phys.align_offset(PAGE_SIZE), 0);
        assert!((phys as usize) < HEAP_START + HEAP_SIZE);
        assert!((phys as usize) >= self.alloc_start);
        let index = (phys as usize - self.alloc_start) / PAGE_SIZE;
        let ip = IPage(index, phys);
        self.dealloc(ip)
    }
}

impl Display for Pmem {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "PAGE ALLOCATION TABLE\nMETA: {:p} -> {:p}\nPHYS: \
                     0x{:x} -> 0x{:x}",
            &self.descriptors[0],
            &self.descriptors[self.descriptors.len() - 1],
            self.alloc_start,
            self.alloc_start + self.descriptors.len() * PAGE_SIZE
        )?;
        writeln!(f, "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~")?;
        let mut allocation = false;
        let mut start = 0;
        let mut total = 0;
        for (i, p) in self.descriptors.iter().enumerate() {
            if !allocation && (p.flags == Taken || p.flags == Last) {
                allocation = true;
                start = i;
                let mem = self.alloc_start + i * PAGE_SIZE;
                write!(f, "0x{:x} => ", mem)?;
            }
            if allocation && p.flags == Last {
                allocation = false;
                let mem = self.alloc_start + i * PAGE_SIZE;
                writeln!(f, "0x{:x}: {:>3} page(s)", mem, i - start + 1)?;
                total += i - start + 1;
            }
        }
        writeln!(f, "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~")?;
        writeln!(
            f,
            "Allocated: {:>6} pages ({:>10} bytes).",
            total,
            total * PAGE_SIZE
        )?;
        writeln!(
            f,
            "Free     : {:>6} pages ({:>10} bytes).",
            self.descriptors.len() - total,
            (self.descriptors.len() - total) * PAGE_SIZE
        )
    }
}

// ========================= MMU =========================

#[repr(transparent)]
pub struct Table {
    entries: [Entry; 512],
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Entry(u64);

mod entry_bits {
    type Flag = u64;
    pub const NONE: Flag = 0;

    pub const VALID: Flag = 1;
    pub const READ: Flag = 2;
    pub const WRITE: Flag = 4;
    pub const EXECUTE: Flag = 8;
    pub const USER: Flag = 16;
    pub const GLOBAL: Flag = 32;
    pub const ACCESS: Flag = 64;
    pub const DIRTY: Flag = 128;

    pub const READ_WRITE: Flag = READ | WRITE;
    pub const READ_EXECUTE: Flag = READ | EXECUTE;
    pub const RWE: Flag = READ | WRITE | EXECUTE;
}

impl Table {
    pub fn new() -> Self {
        Self {
            entries: [Entry(0); 512],
        }
    }
    pub fn len() -> usize {
        512
    }
    pub fn map(
        root: &mut Table,
        pmem: &mut Pmem,
        vaddr: usize,
        paddr: usize,
        bits: u64,
        level: usize,
    ) {
        assert_eq!((vaddr as *const u8).align_offset(PAGE_SIZE), 0);
        assert_eq!((paddr as *const u8).align_offset(PAGE_SIZE), 0);
        assert_ne!(bits & entry_bits::RWE, 0);
        let vpn = [
            vaddr >> 12 & 0x1ff,
            vaddr >> 21 & 0x1ff,
            vaddr >> 30 & 0x1ff,
        ];
        let mut current = root;
        for i in (level + 1..=2).rev() {
            let v = &mut current.entries[vpn[i]];
            if !v.is_valid() {
                let page = pmem.zalloc(1);
                assert!(page.available(), "out of memory");
                v.set_entry(page.physical() as u64 >> 2 | entry_bits::VALID);
            }
            let next = v.get_phys() as *mut Table;
            current = unsafe { &mut *next };
        }
        let entry = (paddr >> 2) as u64 | bits | entry_bits::VALID;
        current.entries[vpn[level]].set_entry(entry);
    }
    pub fn unmap(&mut self, pmem: &mut Pmem) {
        for entry in &mut self.entries {
            if entry.is_valid() && !entry.is_leaf() {
                let next = entry.get_phys() as *mut Table;
                unsafe { (*next).unmap(pmem) };
            }
            if entry.is_valid() {
                unsafe {
                    pmem.dealloc_phys(entry.get_phys() as *mut u8);
                }
            }
        }
    }
    pub fn virt_to_phys(root: &Table, vaddr: *const u8) -> Option<usize> {
        let vaddr = vaddr as usize;
        let vpn = [
            vaddr >> 12 & 0x1ff,
            vaddr >> 21 & 0x1ff,
            vaddr >> 30 & 0x1ff,
        ];
        let mut start = &root.entries[vpn[2]];
        for i in (0..=2).rev() {
            if !start.is_valid() {
                break;
            }
            if start.is_leaf() {
                let mask = (1 << (12 + i * 9)) - 1;
                let vaddr = vaddr & mask;
                let addr = start.get_phys() as usize & !mask;
                return Some((addr | vaddr) as usize);
            }
            assert!(i > 0, "more than three levels found");
            let next = start.get_phys() as *const Table;
            unsafe {
                start = &(*next).entries[vpn[i - 1]];
            }
        }
        None
    }
}

impl Entry {
    pub fn is_valid(&self) -> bool {
        self.0 & entry_bits::VALID != 0
    }
    pub fn is_leaf(&self) -> bool {
        self.0 & entry_bits::RWE != 0
    }
    pub fn set_entry(&mut self, entry: u64) {
        self.0 = entry;
    }
    pub fn get_entry(&self) -> u64 {
        self.0
    }
    pub fn get_phys(&self) -> u64 {
        (self.0 & !0x3ff) << 2
    }
}

extern "C" {
    static TEXT_START: usize;
    static TEXT_END: usize;
    static DATA_START: usize;
    static DATA_END: usize;
    static RODATA_START: usize;
    static RODATA_END: usize;
    static BSS_START: usize;
    static BSS_END: usize;
    static KERNEL_STACK_START: usize;
    static KERNEL_STACK_END: usize;
}

fn id_map_range(root: &mut Table, alloc: &mut Pmem, start: usize, end: usize, bits: u64) {
    let mut addr = start & !(PAGE_SIZE - 1);
    let pages = (end - addr).div_ceil(PAGE_SIZE);
    let pages = max(1, pages);
    for _ in 0..pages {
        Table::map(root, alloc, addr, addr, bits, 0);
        addr += 1 << 12;
    }
}

pub fn id_map(root: &mut Table, alloc: &mut Pmem, kheap_head: usize, kheap_pages: usize) {
    unsafe {
        println!("TEXT:   0x{:x} -> 0x{:x}", TEXT_START, TEXT_END);
        println!("RODATA: 0x{:x} -> 0x{:x}", RODATA_START, RODATA_END);
        println!("DATA:   0x{:x} -> 0x{:x}", DATA_START, DATA_END);
        println!("BSS:    0x{:x} -> 0x{:x}", BSS_START, BSS_END);
        println!(
            "STACK:  0x{:x} -> 0x{:x}",
            KERNEL_STACK_START, KERNEL_STACK_END
        );
        println!(
            "HEAP:   0x{:x} -> 0x{:x}",
            kheap_head,
            kheap_head + kheap_pages * 4096
        );
    }

    id_map_range(
        root,
        alloc,
        kheap_head,
        kheap_head + kheap_pages * PAGE_SIZE,
        entry_bits::READ_WRITE,
    );

    id_map_range(
        root,
        alloc,
        alloc.descriptors.as_ptr() as usize,
        alloc.descriptors.as_ptr() as usize
            + alloc.descriptors.len() * core::mem::size_of::<Page>(),
        entry_bits::READ_WRITE,
    );
    unsafe {
        id_map_range(root, alloc, TEXT_START, TEXT_END, entry_bits::READ_EXECUTE);

        id_map_range(
            root,
            alloc,
            RODATA_START,
            RODATA_END,
            entry_bits::READ_EXECUTE,
        );

        id_map_range(root, alloc, DATA_START, DATA_END, entry_bits::READ_WRITE);

        id_map_range(root, alloc, BSS_START, BSS_END, entry_bits::READ_WRITE);

        id_map_range(
            root,
            alloc,
            KERNEL_STACK_START,
            KERNEL_STACK_END,
            entry_bits::READ_WRITE,
        );
    }

    id_map_range(root, alloc, 0x10000000, 0x1000000F, entry_bits::READ_WRITE);
}
