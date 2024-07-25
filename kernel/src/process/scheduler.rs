use super::{
    process::{Priority, Process, Thread, ThreadStatus},
    switch::switch_to_task,
};
use crate::println;
use alloc::collections::VecDeque;

pub static mut SCHEDULER: Option<Scheduler> = None;

extern "C" {
    fn context_switch(old_stack: *mut u64, new_stack: *const u64);
}

extern "C" fn idle_thread() {
    loop {
        println!("Idle thread running");
    }
}

pub struct Scheduler {
    current_thread: Thread,
    idle_thread: Thread,
    ready_queue: [VecDeque<Thread>; 4],
}

impl Scheduler {
    pub fn new() -> Self {
        let idle_process = Process::create_kernel_process(idle_thread, Priority::Idle);
        let idle_thread = idle_process.borrow().threads[0].borrow().clone();
        let mut ready_queue: [VecDeque<Thread>; 4] = Default::default();
        ready_queue[Priority::Idle as usize].push_back(idle_thread.clone());

        Scheduler {
            current_thread: idle_thread.clone(),
            idle_thread: idle_thread.clone(),
            ready_queue,
        }
    }

    pub fn add_thread(&mut self, thread: Thread) {
        let priority = thread.priority;
        self.ready_queue[priority as usize].push_back(thread);
    }

    pub fn get_next_thread(&mut self) -> Thread {
        for priority in 0..self.ready_queue.len() {
            while let Some(thread) = self.ready_queue[priority].pop_front() {
                if thread.status == ThreadStatus::Ready {
                    return thread.clone();
                }
            }
        }
        self.idle_thread.clone()
    }

    pub fn schedule(&mut self) {
        let (mut current_stack_pointer, current_prio, current_status) = {
            let borrowed = self.current_thread.clone();
            (borrowed.stack_pointer, borrowed.priority, borrowed.status)
        };

        let mut next_thread = self.get_next_thread();
        next_thread.status = ThreadStatus::Running;

        if current_status == ThreadStatus::Running {
            self.current_thread.status = ThreadStatus::Ready;
            self.ready_queue[current_prio as usize].push_back(self.current_thread.clone());
        }

        self.current_thread = next_thread.clone();

        unsafe {
            // println!(
            //     "Switching threads from {:#x} to {:#x}",
            //     current_stack_pointer, next_thread.stack_pointer
            // );
            switch_to_task(&mut current_stack_pointer, &next_thread.stack_pointer);
        }
    }
}
