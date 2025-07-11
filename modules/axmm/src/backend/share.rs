use axalloc::global_allocator;
use axhal::mem::{phys_to_virt, virt_to_phys};
use axhal::paging::{MappingFlags, PageSize, PageTable};
use memory_addr::{PAGE_SIZE_4K, PhysAddr, VirtAddr};

use super::Backend;

//类似线性地址，但是没有那个麻烦。signal trampoline

//引用计数
use ::alloc::sync::Arc;
use alloc::vec::Vec;

fn alloc_frame(zeroed: bool) -> Option<PhysAddr> {
    let vaddr = VirtAddr::from(global_allocator().alloc_pages(1, PAGE_SIZE_4K).ok()?);
    if zeroed {
        unsafe { core::ptr::write_bytes(vaddr.as_mut_ptr(), 0, PAGE_SIZE_4K) };
    }
    let paddr = virt_to_phys(vaddr);
    Some(paddr)
}

fn dealloc_frame(frame: PhysAddr) {
    let vaddr = phys_to_virt(frame);
    global_allocator().dealloc_pages(vaddr.as_usize(), 1);
}

impl Backend {
    /// Creates a new allocation mapping backend.
    ///
    pub fn new_share(page_num: usize, phys_pages: Option<Arc<[PhysAddr]>>) -> Self {
        let pages = if let Some(phys_pages) = phys_pages {
            phys_pages //有？直接返回
        } else {
            //error!("alloc");
            Arc::from(
                //没有？新建
                (0..page_num)
                    .map(|_| alloc_frame(true).unwrap())
                    .collect::<Vec<_>>(),
            )
        };
        Self::Share { pages }
    }

    pub(crate) fn map_share(
        start: VirtAddr,
        pages: &Arc<[PhysAddr]>,
        flags: MappingFlags,
        pt: &mut PageTable,
    ) -> bool {
        debug!(
            "map_share: [{:#x}, {:#x}) {:?}",
            start,
            start + pages.len() * PAGE_SIZE_4K,
            flags
        );
        for (i, &frame) in pages.iter().enumerate() {
            let vaddr = start + i * PAGE_SIZE_4K;
            if let Ok(tlb) = pt.map(vaddr, frame, PageSize::Size4K, flags) {
                tlb.ignore();
            } else {
                return false;
            }
        }
        true
    }

    pub(crate) fn unmap_share(
        start: VirtAddr,
        pages: &Arc<[PhysAddr]>,
        pt: &mut PageTable,
    ) -> bool {
        debug!(
            "unmap_share: [{:#x}, {:#x})",
            start,
            start + pages.len() * PAGE_SIZE_4K
        );
        let ref_count = Arc::strong_count(pages);
        //error!("ref count of alloced pages:{}", ref_count);
        for (i, pages) in pages.iter().enumerate() {
            let vaddr = start + i * PAGE_SIZE_4K;
            if let Ok((_, _, tlb)) = pt.unmap(vaddr) {
                tlb.flush();
                if ref_count == 1 {
                    //error!("ref_count==1,dealloc");
                    dealloc_frame(*pages);
                }
            } else {
                return false;
            }
        }

        true
    }

    // pub(crate) fn handle_page_fault_alloc(
    //     vaddr: VirtAddr,
    //     orig_flags: MappingFlags,
    //     pt: &mut PageTable,
    //     populate: bool,
    // ) -> bool {
    //     if populate {
    //         false // Populated mappings should not trigger page faults.
    //     } else if let Some(frame) = alloc_frame(true) {
    //         // Allocate a physical frame lazily and map it to the fault address.
    //         // `vaddr` does not need to be aligned. It will be automatically
    //         // aligned during `pt.map` regardless of the page size.
    //         pt.map(vaddr, frame, PageSize::Size4K, orig_flags)
    //             .map(|tlb| tlb.flush())
    //             .is_ok()
    //     } else {
    //         false
    //     }
    // }
}
