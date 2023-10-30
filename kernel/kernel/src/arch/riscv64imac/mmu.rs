use riscv::cpu::{SStatus, SStatusFlags, Satp, SatpData, SatpMode};
use riscv::mem::ptrs::PhysMutPtr;
use riscv::pt::PageTable;

pub unsafe fn use_pagetable(root: PhysMutPtr<PageTable>) {
    // enable MXR (make Executable readable) bit
    // enable SUM (premit Supervisor User Memory access) bit
    unsafe {
        SStatus::set(SStatusFlags::MXR & SStatusFlags::SUM);
    }

    log::trace!("enabling new pagetable {:p}", root);

    // Setup Root Page table in satp register
    unsafe {
        Satp::write(SatpData {
            mode: SatpMode::Sv39,
            asid: 0,
            ppn: root.raw() as u64 >> 12,
        });
    }
}
