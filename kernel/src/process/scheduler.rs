use super::{
    process::{Priority, Process, Thread, ThreadStatus},
    switch::switch,
};
use crate::{println, sync::mutex::SpinMutex};
use alloc::{collections::VecDeque, sync::Arc};

pub static mut SCHEDULER: Option<Scheduler> = None;

extern "C" fn idle_thread() {
    loop {
        println!("Idle thread running");
    }
}

pub struct Scheduler {
    current_thread: Arc<SpinMutex<Thread>>,
    idle_thread: Arc<SpinMutex<Thread>>,
    ready_queue: [VecDeque<Arc<SpinMutex<Thread>>>; 4],
}

impl Scheduler {
    pub fn new() -> Self {
        let idle_process = Process::create_kernel_process(idle_thread, Priority::Idle);
        let idle_thread = Arc::new(SpinMutex::new(
            idle_process.borrow().threads[0].borrow().clone(),
        ));
        let mut ready_queue: [VecDeque<Arc<SpinMutex<Thread>>>; 4] = Default::default();
        ready_queue[Priority::Idle as usize].push_back(idle_thread.clone());

        Scheduler {
            current_thread: idle_thread.clone(),
            idle_thread: idle_thread.clone(),
            ready_queue,
        }
    }

    pub fn add_thread(&mut self, thread: Thread) {
        let priority = thread.priority;
        let thread = Arc::new(SpinMutex::new(thread));
        self.ready_queue[priority as usize].push_back(thread);
    }

    pub fn get_next_thread(&mut self) -> Arc<SpinMutex<Thread>> {
        for priority in 0..self.ready_queue.len() {
            while let Some(thread) = self.ready_queue[priority].pop_front() {
                if thread.lock().status == ThreadStatus::Ready {
                    return thread.clone();
                }
            }
        }
        self.idle_thread.clone()
    }

    pub fn schedule(&mut self) {
        // Get current thread info
        let (mut current_stack_pointer, current_prio, current_status) = {
            let locked = self.current_thread.lock();
            (locked.stack_pointer, locked.priority, locked.status)
        };

        // Get the next thread to run
        let next_thread = self.get_next_thread();
        let next_stack_pointer = {
            let mut next_locked = next_thread.lock();
            next_locked.status = ThreadStatus::Running;
            next_locked.stack_pointer
        };

        // Update the current thread status and move it to the ready queue if it was running
        if current_status == ThreadStatus::Running {
            {
                let mut current_locked = self.current_thread.lock();
                current_locked.status = ThreadStatus::Ready;
            }
            self.ready_queue[current_prio as usize].push_back(self.current_thread.clone());
        }

        // Switch to the next thread
        self.current_thread = Arc::clone(&next_thread);

        switch(&mut current_stack_pointer, &next_stack_pointer);
    }
}
