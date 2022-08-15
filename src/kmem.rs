use crate::page;
use crate::page::{Table, PAGE_SIZE};

const PAGES_POW: usize = 6;
const MIN_SIZE_POW: usize = 7;
#[repr(transparent)]
struct BuddyLeaf(u8);

// taken 0b111111xx
// free 0bxxxxxxxx
// leaf 0bxxxxxx00
// parent 0bxxxxxx01
impl BuddyLeaf {
    fn parent(&self) -> bool {
        self.0 & 1 != 0
    }
    fn leaf(&self) -> bool {
        !self.parent()
    }
    fn set_parent(&mut self) {
        self.0 |= 1
    }
    fn set_leaf(&mut self) {
        self.0 &= !1
    }
    fn get_level(&self) -> u8 {
        self.0 >> 2
    }
    fn set_level(&mut self, level: u8) {
        self.0 |= level << 2
    }
}
#[repr(transparent)]
struct BuddyMeta {
    nodes: [BuddyLeaf; (1 << (MAX_ALLOCATION - MIN_SIZE_POW + 1)) - 1],
}

const MAX_ALLOCATION: usize = PAGES_POW + 12;

impl BuddyMeta {
    pub fn largest() -> usize {
        1 << (MAX_ALLOCATION)
    }
    pub fn get_parent(child: usize) -> usize {
        (child - 1) / 2
    }
    pub fn get_left(parent: usize) -> usize {
        parent * 2 + 1
    }
    pub fn get_right(parent: usize) -> usize {
        parent * 2 + 2
    }
    pub fn get_buddy(other: usize) -> usize {
        if other % 2 == 1 {
            other + 1
        } else {
            other - 1
        }
    }
    pub fn access_mut(&mut self, index: usize) -> &mut BuddyLeaf {
        &mut self.nodes[index]
    }
    pub fn access(&self, index: usize) -> &BuddyLeaf {
        &self.nodes[index]
    }
    pub fn addr_to_index(&self, begin_alloc: usize, addr: usize) -> usize {
        assert_eq!(begin_alloc % PAGE_SIZE, 0);
        assert!(addr >= begin_alloc);
        assert_eq!(addr & ((1 << 8) - 1), 0);
        let mut current = 0;
        let mut current_addr = begin_alloc;
        let mut level = 0;
        loop {
            let node = self.access(current);
            if node.leaf() {
                assert_eq!(node.get_level(), 0b111111);
                break;
            }
            let node_size = MAX_ALLOCATION >> level;
            if addr >= current_addr + node_size {
                current_addr += node_size;
                current = BuddyMeta::get_right(current);
            } else {
                current = BuddyMeta::get_left(current);
            }
        }
        current
    }
    pub fn index_to_addr(&self, begin_alloc: usize, index: usize) -> usize {
        assert_eq!(begin_alloc % PAGE_SIZE, 0);
        assert!(index < self.nodes.len());
        let level = usize::BITS - (index + 1).leading_zeros() - 1;
        let pow = MAX_ALLOCATION - level as usize;
        let offset = (1 << pow) * ((index + 1) & ((1 << level) - 1));
        assert!(offset <= BuddyMeta::largest());
        begin_alloc + offset
    }
}

pub struct Kmem {
    head: *mut BuddyMeta, // BuddyMeta
    page_table: *mut Table,
    alloc: usize,
    data_start: *mut u8,
}

impl Kmem {
    pub fn init(pmem: &mut page::Pmem) -> Self {
        let k_alloc = pmem.zalloc(1 + (1 << PAGES_POW));
        assert!(k_alloc.available());
        let head = k_alloc.physical() as *mut BuddyMeta;
        assert!(core::mem::size_of::<BuddyMeta>() <= PAGE_SIZE);

        let head_ref = unsafe { &mut *head };
        head_ref.access_mut(0).set_leaf();
        head_ref.access_mut(0).set_level(0);
        Self {
            head,
            page_table: pmem.zalloc(1).physical() as *mut Table,
            alloc: 1 + (1 << PAGES_POW),
            data_start: unsafe { k_alloc.physical().add(PAGE_SIZE) } as *mut u8,
        }
    }
    pub fn get_head(&self) -> *const u8 {
        self.head as *const u8
    }
    pub fn get_allocations(&self) -> usize {
        self.alloc
    }
    pub fn get_root(&mut self) -> &mut Table {
        unsafe { &mut *self.page_table }
    }
    pub fn kamlloc(&mut self, pow: usize) -> *mut u8 {
        assert!(pow >= MIN_SIZE_POW);
        let meta = unsafe { &mut *self.head };
        // parent and free -> a child is a free leaf
        let mut current = 0_usize;
        let max_pow = MAX_ALLOCATION;
        let mut level = 0;
        loop {
            let node = meta.access_mut(current);
            if node.leaf() {
                if max_pow - (level + 1) >= pow {
                    meta.access_mut(BuddyMeta::get_right(current))
                        .set_level(level as u8 + 1);
                    current = BuddyMeta::get_left(current);
                } else if max_pow - level == pow {
                    break;
                }
            } else if node.parent() {
                if max_pow - node.get_level() as usize >= pow {
                    let left = meta.access(BuddyMeta::get_left(current));
                    let right = meta.access(BuddyMeta::get_right(current));
                    let left_size = max_pow - left.get_level() as usize;
                    let right_size = max_pow - right.get_level() as usize;
                    if left_size >= pow && right_size >= pow {
                        if right_size < left_size {
                            current = BuddyMeta::get_right(current);
                        } else {
                            current = BuddyMeta::get_left(current);
                        }
                    } else if left_size >= pow {
                        current = BuddyMeta::get_left(current);
                    } else if right_size >= pow {
                        current = BuddyMeta::get_right(current)
                    } else {
                        panic!("malformed metadata")
                    }
                } else {
                    panic!("out of kernel memory")
                }
            }
            level += 1;
        }
        //walk up and patch parents
        let chosen = current;
        let node = meta.access_mut(current);
        assert!(node.leaf());
        node.set_level(u8::MAX);
        loop {
            if current == 0 {
                break;
            }
            current = BuddyMeta::get_parent(current);
            let left_level = meta.access(BuddyMeta::get_left(current)).get_level();
            let right_level = meta.access(BuddyMeta::get_right(current)).get_level();
            let node = meta.access_mut(current);
            node.set_level(core::cmp::min(left_level, right_level));
            node.set_parent();
        }
        let ptr: *mut u8 = meta.index_to_addr(self.data_start as usize, chosen) as *mut u8;
        debug_assert_eq!(
            meta.addr_to_index(self.data_start as usize, ptr as usize),
            chosen
        );
        ptr
    }
}
