//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, current_user_token, current_page_table, current_syscall_cnt_ref};
use crate::mm::{PageTable, PhysAddr, PhysPageNum, VirtAddr, VirtPageNum, PTEFlags, translated_byte_buffer};
use crate::timer::get_time_us;
use crate::config::PAGE_SIZE;

const MICRO_PER_SEC: usize = 1_000_000;
const MMAP_POOL_START: usize = 0x87000000;
static mut MMAP_OFFSET: usize = 0;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let buffers = translated_byte_buffer(current_user_token(), ts as *const u8, core::mem::size_of::<TimeVal>());
    let time = get_time_us();
    let timeval = TimeVal {
        sec: time / MICRO_PER_SEC,
        usec: time % MICRO_PER_SEC,
    };

    let ptr = &timeval as *const TimeVal as *const u8;
    for (dst, idx) in buffers.into_iter().flatten().zip(0..core::mem::size_of::<TimeVal>()) {
        unsafe {
            *dst = *ptr.add(idx);
        };
    }

    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match _trace_request {
        // Read a byte from *id, ignore data
        0 => {
            let virt_addr = VirtAddr(id);
            let page_table = PageTable::from_token(current_user_token());
            if let Some(pte) = page_table.translate(virt_addr.floor()) {
                if !pte.is_valid() || !pte.readable() || (pte.flags() & PTEFlags::U).is_empty() {
                    return -1;
                }

                return pte.ppn().get_bytes_array()[virt_addr.page_offset()] as isize;
            }

            -1
        }

        // Write a byte `data` to *id
        1 => {
            let virt_addr = VirtAddr(id);
            let page_table = PageTable::from_token(current_user_token());
            if let Some(pte) = page_table.translate(virt_addr.floor()) {
                if !pte.is_valid() || !pte.writable() || (pte.flags() & PTEFlags::U).is_empty() {
                    return -1;
                }

                pte.ppn().get_bytes_array()[virt_addr.page_offset()] = data as u8;
                return 0;
            }

            -1
        }

        2=> {
            if id > 512 {
                return -1;
            }

            *current_syscall_cnt_ref(id as u32) as isize
        }

        _ => -1
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    // Check arguments sanity
    let start_va = VirtAddr(start);

    // start MUST be aligned
    if !start_va.aligned() {
        return -1;
    }

    // prot MUST be valid
    if (prot & 0x7 == 0) || (prot & !0x7 != 0) {
        return -1;
    }

    let mut perm = PTEFlags::empty();
    perm |= PTEFlags::U | PTEFlags::V;
    if prot & 0x1 != 0 {
        perm |= PTEFlags::R;
    }
    if prot & 0x2 != 0 {
        perm |= PTEFlags::W;
    }
    if prot & 0x4 != 0 {
        perm |= PTEFlags::X;
    }

    let mut pagetable = current_page_table();
    let pages_cnt = (len + PAGE_SIZE - 1) / PAGE_SIZE;
    let start_vpn: VirtPageNum = start_va.into();

    for i in 0..pages_cnt {
        let vpn = VirtPageNum(usize::from(start_vpn) + i);
        if let Some(pte) = pagetable.translate(vpn) {
            if pte.is_valid() {
                return -1;
            }
        }
        //let ppn = frame_alloc().unwrap().ppn;
        let ppn = PhysPageNum(usize::from(PhysPageNum::from(PhysAddr(MMAP_POOL_START + unsafe { MMAP_OFFSET }))) + i);
        pagetable.map(vpn, ppn, perm);
        unsafe { MMAP_OFFSET += PAGE_SIZE };
    }

    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");

    let virt_addr = VirtAddr(start);
    if !virt_addr.aligned() {
        return -1;
    }

    let pages_cnt = (len + PAGE_SIZE - 1) / PAGE_SIZE;
    let mut pagetable = PageTable::from_token(current_user_token());
    let start_vpn: VirtPageNum = virt_addr.into();
    for i in 0..pages_cnt {
        let vpn = VirtPageNum(usize::from(start_vpn) + i);
        if let Some(pte) = pagetable.translate(vpn) {
            if pte.is_valid() {
                pagetable.unmap(vpn);
            } else {
                return -1;
            }
        } else {
            return -1;
        }
    }

    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
