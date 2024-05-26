use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        process_inner.lk_available[id+16] = 1;
        for i in 0..32 {
            process_inner.lk_max[i][id+16] = 1;
            // process_inner.lk_needed[i][id+16] = 1;
        }
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        let iid = process_inner.mutex_list.len() - 1;
        process_inner.lk_available[iid+16] = 1;
        for i in 0..32 {
            process_inner.lk_max[i][iid+16] = 1;
            // process_inner.lk_needed[i][iid+16] = 1;
        }
        iid as isize
    }
}

fn deadlock_detect(lk_available_: &Vec<usize>, lk_needed: &Vec<Vec<usize>>, lk_allocation: &Vec<Vec<usize>>) -> bool {
    let mut lk_available = lk_available_.clone();
    let num_processes = lk_needed.len();
    let num_resources = lk_available.len();
    let mut finish = vec![false; num_processes];
    
    loop {
        let mut progress = false;

        for i in 0..num_processes {
            if !finish[i] {
                let mut can_finish = true;

                for j in 0..num_resources {
                    if i < 4 && j < 4 {
                        println!("needed[{}][{}]={}, available[{}]={}", i, j, lk_needed[i][j], j, lk_available[j]);
                    }
                    if lk_needed[i][j] > lk_available[j] {
                        can_finish = false;
                        println!("can not finish i={}, j={}", i, j);
                        break;
                    }
                }

                if can_finish {
                    for j in 0..num_resources {
                        lk_available[j] += lk_allocation[i][j];
                    }
                    finish[i] = true;
                    progress = true;
                }
            }
        }

        if !progress {
            break;
        }
    }

    let ret = !finish.iter().all(|&f| f);
    println!("deadlock_detect ret: {}", ret);
    ret
}

/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_task()
    .unwrap()
    .inner_exclusive_access()
    .res
    .as_ref()
    .unwrap()
    .tid;
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    process_inner.lk_needed[tid][mutex_id+16] += 1;
    if process_inner.lk_detect && deadlock_detect(&process_inner.lk_available, &process_inner.lk_needed, &process_inner.lk_allocation) {
        return -0xdead;
    }
    process_inner.lk_available[mutex_id+16] -= 1;
    process_inner.lk_allocation[tid][mutex_id+16] += 1;
    // process_inner.lk_needed[tid][mutex_id+16] = process_inner.lk_max[tid][mutex_id+16] - process_inner.lk_allocation[tid][mutex_id+16];

    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_task()
    .unwrap()
    .inner_exclusive_access()
    .res
    .as_ref()
    .unwrap()
    .tid;
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    process_inner.lk_available[mutex_id+16] += 1;
    process_inner.lk_allocation[tid][mutex_id+16] -= 1;
    process_inner.lk_needed[tid][mutex_id+16] -= 1;
    // process_inner.lk_needed[tid][mutex_id+16] = process_inner.lk_max[tid][mutex_id+16] - process_inner.lk_allocation[tid][mutex_id+16];

    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    process_inner.lk_available[id] = res_count;
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_task()
    .unwrap()
    .inner_exclusive_access()
    .res
    .as_ref()
    .unwrap()
    .tid;
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    // process_inner.lk_available[sem_id] += 1;
    // process_inner.lk_allocation[tid][sem_id] -= 1;

    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.lk_available[sem_id] += 1;
    process_inner.lk_allocation[tid][sem_id] -= 1;

    0
}

/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let tid = current_task()
    .unwrap()
    .inner_exclusive_access()
    .res
    .as_ref()
    .unwrap()
    .tid;
    println!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down, sem_id: {}",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid,
        sem_id
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    process_inner.lk_needed[tid][sem_id] += 1;
    if process_inner.lk_detect && deadlock_detect(
        &process_inner.lk_available,
        &process_inner.lk_needed,
        &process_inner.lk_allocation) {
        return -0xdead;
    }

    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.down();

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.lk_available[sem_id] -= 1;
    process_inner.lk_allocation[tid][sem_id] += 1;
    process_inner.lk_needed[tid][sem_id] -= 1;

    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect NOT IMPLEMENTED");
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.lk_detect = enabled == 1;
    0
}
