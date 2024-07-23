use crate::println;

use super::process::{Priority, Process, Thread, ThreadStatus};
use alloc::{collections::VecDeque, rc::Rc};
use core::cell::RefCell;

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
    current_thread: Option<Rc<RefCell<Thread>>>,
    idle_thread: Rc<RefCell<Thread>>,
    ready_queue: [VecDeque<Rc<RefCell<Thread>>>; 4],
}

impl Scheduler {
    pub fn new() -> Self {
        let idle_process = Process::create_kernel_process(idle_thread, Priority::Idle);
        let idle_thread = idle_process.borrow().threads[0].clone();
        let mut ready_queue: [VecDeque<Rc<RefCell<Thread>>>; 4] = Default::default();
        ready_queue[Priority::Idle as usize].push_back(idle_thread.clone());

        Scheduler {
            current_thread: Some(idle_thread.clone()),
            idle_thread,
            ready_queue,
        }
    }

    pub fn add_thread(&mut self, thread: Rc<RefCell<Thread>>) {
        let priority = thread.borrow().priority;
        self.ready_queue[priority as usize].push_back(thread);
    }

    pub fn get_next_thread(&mut self) -> Rc<RefCell<Thread>> {
        for priority in 0..self.ready_queue.len() {
            while let Some(thread) = self.ready_queue[priority].pop_front() {
                if thread.borrow().status == ThreadStatus::Ready {
                    return thread.clone();
                }
            }
        }
        self.idle_thread.clone()
    }

    pub fn schedule(&mut self) {
        let (current_stack_pointer, current_prio, current_status) = {
            let borrowed = self.current_thread.as_mut().unwrap().borrow_mut();
            (borrowed.stack_pointer, borrowed.priority, borrowed.status)
        };

        let next_thread = self.get_next_thread();
        next_thread.borrow_mut().status = ThreadStatus::Running;

        if current_status == ThreadStatus::Running {
            self.current_thread.as_mut().unwrap().borrow_mut().status = ThreadStatus::Ready;
            self.ready_queue[current_prio as usize]
                .push_back(self.current_thread.as_mut().unwrap().clone());
        }

        self.current_thread = Some(next_thread.clone());

        unsafe {
            println!(
                "Switching threads from {:#x} to {:#x}",
                current_stack_pointer,
                next_thread.borrow().stack_pointer
            );
            context_switch(
                current_stack_pointer as *mut u64,
                &next_thread.borrow().stack_pointer as *const u64,
            );
        }
    }
}
