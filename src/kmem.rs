use crate::page;
use crate::page::{Table, PAGE_SIZE};

mod alloc_list_flags {
    pub const TAKEN: usize = 1 << 63;
}
#[repr(transparent)]
struct AllocList(usize);

impl AllocList {
    fn is_taken(&self) -> bool {
        self.0 * alloc_list_flags::TAKEN != 0
    }
    fn set_taken(&mut self) {
        self.0 |= alloc_list_flags::TAKEN;
    }
    fn set_free(&mut self) {
        self.0 &= !alloc_list_flags::TAKEN;
    }
    fn set_size(&mut self, size: usize) {
        assert_eq!(size & alloc_list_flags::TAKEN, 0);
        self.0 = size | (self.0 & alloc_list_flags::TAKEN);
    }
    fn get_size(&self) -> usize {
        self.0 & !alloc_list_flags::TAKEN
    }
}

pub struct Kmem {
    head: *mut u8,
    page_table: *mut Table,
    alloc: usize,
}

impl Kmem {
    pub fn init(pmem: &mut page::Pmem) -> Self {
        let k_alloc = pmem.zalloc(64);
        assert!(k_alloc.available());
        let head = k_alloc.physical() as *mut AllocList;
        let head_ref = unsafe {
            head.write(AllocList(0));
            &mut *head
        };
        head_ref.set_size(64 * PAGE_SIZE);
        Self {
            head: k_alloc.physical() as *mut u8,
            page_table: pmem.zalloc(1).physical() as *mut Table,
            alloc: 64,
        }
    }
    pub fn get_head(&self) -> *const u8 {
        self.head
    }
    pub fn get_allocations(&self) -> usize {
        self.alloc
    }
    pub fn get_root(&mut self) -> &mut Table {
        unsafe { &mut *self.page_table }
    }
}
