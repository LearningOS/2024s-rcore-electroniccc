//! Process management syscalls
use core::ptr;
use crate::mm::VirtAddr;
use crate::task::current_user_token;
use crate::task::get_task_status;
use crate::task::get_sys_call_times;
use crate::mm::vaddr_to_pddr_u8;
use crate::timer::get_time_us;
use crate::task::get_task_run_time;
use crate::task::task_mm_map;
use crate::task::task_unmap;

use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus,
    },
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
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

fn mem_cpy_to_user_ph(user_ph_addr: *mut u8, kernel_addr: *const u8, len: usize) {
    let token = current_user_token();
    unsafe {
        for i in 0..len {
            ptr::write(vaddr_to_pddr_u8(token, user_ph_addr.add(i)), *kernel_addr.add(i));
        }
    }
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    let time_val = TimeVal{
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let time_val_ptr: *const TimeVal = &time_val;
    mem_cpy_to_user_ph(ts as *mut u8, time_val_ptr as *const u8, 16);
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let kti = TaskInfo {
        status: get_task_status(),
        syscall_times: get_sys_call_times(),
        time: get_task_run_time(),
    };
    let kti_ptr: *const TaskInfo = &kti;
    mem_cpy_to_user_ph(ti as *mut u8, kti_ptr as *const u8, 2016);
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    if start % 4096 != 0 {
        println!("kernel: sys_mmap page start is not a multiple  of 4096");
        return -1;
    }
    if port & !0x7 != 0 {
        println!("kernel: sys_mmap prot is not valid, 1");
        return -1;
    }
    if port & 0x7 == 0 {
        println!("kernel: sys_mmap prot is not valid, 2");
        return -1;
    }
    if !task_mm_map(VirtAddr::from(start), VirtAddr::from(start + len), port) {
        println!("kernel: sys_mmap area mapped");
        return -1;
    }
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    // trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if start % 4096 != 0 {
        println!("kernel: sys_munmap page start is not a multiple  of 4096");
        return -1;
    }
    if !task_unmap(VirtAddr::from(start), VirtAddr::from(start + len)) {
        return -1;
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
