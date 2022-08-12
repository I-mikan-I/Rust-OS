use crate::page::PageBits::{Empty, Last, Taken};
use crate::print;
use crate::println;
use core::fmt::{Display, Formatter};
use core::mem::MaybeUninit;

extern "C" {
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}

const PAGE_SIZE: usize = 1 << 12;

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
}
#[non_exhaustive]
pub struct IPage(usize);

impl IPage {
    pub fn available(&self) -> bool {
        self.0 != usize::MAX
    }
}

impl Pmem {
    pub fn init() -> Pmem {
        println!("heap: {:x}, page: {:x}", PAGE_SIZE, PAGE_SIZE);
        unsafe {
            let num_pages = HEAP_SIZE / PAGE_SIZE;
            println!("num pages");
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
                    + (-offset).rem_euclid(PAGE_SIZE as isize) as usize,
            }
        }
    }
    pub fn alloc(&mut self, pages: usize) -> IPage {
        let num_pages = self.descriptors.len();
        let mut found = 0;
        let mut begin = usize::MAX;
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
                break;
            }
        }
        for p in &mut self.descriptors[begin..][..found - 1] {
            p.flags = Taken;
        }
        IPage(begin)
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
            if !allocation && p.flags == Taken {
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
