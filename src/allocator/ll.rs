use super::align_up;
use super::Locked;
use core::alloc::Layout;
use core::ptr;
use core::{alloc::GlobalAlloc, mem};

struct LLNode {
    size: usize,
    next: Option<&'static mut LLNode>,
}

impl LLNode {
    const fn new(size: usize) -> Self {
        LLNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LLAllocator {
    head: LLNode,
}

impl LLAllocator {
    pub const fn new() -> Self {
        Self {
            head: LLNode::new(0),
        }
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        assert_eq!(align_up(addr, mem::align_of::<LLNode>()), addr);
        assert!(size >= mem::size_of::<LLNode>());

        let mut node = LLNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut LLNode;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr)
    }

    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut LLNode, usize)> {
        let mut curr = &mut self.head;
        while let Some(ref mut region) = curr.next {
            if let Ok(alloc_start) = Self::calculate_start(&region, size, align) {
                let next = region.next.take();
                let ret = Some((curr.next.take().unwrap(), alloc_start));
                curr.next = next;
                return ret;
            }
            curr = curr.next.as_mut().unwrap()
        }

        None
    }

    fn calculate_start(region: &LLNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<LLNode>() {
            return Err(());
        }

        return Ok(alloc_start);
    }

    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<LLNode>())
            .expect("Alignment adjustment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<LLNode>());

        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LLAllocator> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut allocator = self.lock();
        let (size, align) = LLAllocator::size_align(layout);

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");

            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size);
            }

            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let (size, _) = LLAllocator::size_align(layout);
        self.lock().add_free_region(ptr as usize, size);
    }
}
