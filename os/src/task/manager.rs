//!Implementation of [`TaskManager`]
use super::{TaskControlBlock, TaskStatus, current_task};
use crate::config::MAX_SYSCALL_NUM;
use crate::mm::{MapPermission, VirtAddr};
use crate::sync::UPSafeCell;
use crate::timer::get_time_us;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // self.ready_queue.pop_front()
        if self.ready_queue.is_empty() {
            println!("fetch none");
            return None;
        }
        let mut ret_idx = 0;
        for (idx, item) in self.ready_queue.iter().enumerate() {
            if idx == ret_idx {
                continue;
            }
            let inner = item.inner_exclusive_access();
            let ret_inner = self.ready_queue[ret_idx].inner_exclusive_access();
            if inner.stride < ret_inner.stride {
                ret_idx = idx;
            }
        }

        self.ready_queue.remove(ret_idx)
    }

    /// abc
    pub fn get_sys_call_times(&self) -> [u32; MAX_SYSCALL_NUM] {
        let task = current_task().unwrap();
        let inner = task.inner_exclusive_access();

        inner.sys_call_times.clone()
    }

    /// abc
    pub fn inc_sys_call_times(&self, call_id: usize) {
        let task = current_task().unwrap();
        let mut inner = task.inner_exclusive_access();
        inner.sys_call_times[call_id] += 1;
    }

    /// abc
    pub fn get_task_start_time(&self) -> usize {
        let task = current_task().unwrap();
        let inner = task.inner_exclusive_access();

        inner.start_time
    }

    /// abc
    pub fn get_task_status(&self) -> TaskStatus {
        let task = current_task().unwrap();
        let inner = task.inner_exclusive_access();

        inner.task_status
    }

    /// abc
    pub fn mem_map(&self, vstart: VirtAddr, vend: VirtAddr, prop: usize) -> bool {
        let task = current_task().unwrap();
        let mut inner = task.inner_exclusive_access();
        let mut pers = MapPermission::empty();
        if prop & 0x1 == 0x1 {
            pers |= MapPermission::R;
        }
        if prop & 0x2 == 0x2 {
            pers |= MapPermission::W;
        }
        if prop & 0x4 == 0x4 {
            pers |= MapPermission::X;
        }
        pers |= MapPermission::U;
        if !vstart.aligned() {
            return false;
        }
        if inner.memory_set.is_mapped(vstart, vend) {
            return false;
        }
        inner.memory_set.insert_framed_area(vstart, vend, pers);
        true
    }

    /// abc
    pub fn mem_unmap(&self, vstart: VirtAddr, vend: VirtAddr) -> bool {
        let task = current_task().unwrap();
        let mut inner = task.inner_exclusive_access();
        inner.memory_set.remove_frame(vstart, vend)
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}

/// get sys call times
pub fn get_sys_call_times() -> [u32; MAX_SYSCALL_NUM] {
    TASK_MANAGER.exclusive_access().get_sys_call_times()
}

/// inc sys call tiems
pub fn inc_sys_call_times(call_id: usize) {
    TASK_MANAGER.exclusive_access().inc_sys_call_times(call_id);
}

/// get task status
pub fn get_task_status() -> TaskStatus {
    TASK_MANAGER.exclusive_access().get_task_status()
}

/// get task run time
pub fn get_task_run_time() -> usize {
    let cur_time = get_time_us();
    let task_start = TASK_MANAGER.exclusive_access().get_task_start_time();

    (cur_time - task_start) / 1000
}

/// abc
pub fn task_mm_map(vstart: VirtAddr, vend: VirtAddr, prop: usize) -> bool {
    TASK_MANAGER.exclusive_access().mem_map(vstart, vend, prop)
}

/// abc
pub fn task_unmap(vstart: VirtAddr, vend: VirtAddr) -> bool {
    TASK_MANAGER.exclusive_access().mem_unmap(vstart, vend)
}
