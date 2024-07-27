use super::{
    process::Process,
    switch::switch,
    thread::{Priority, Status, Thread},
};
use crate::{interrupts::no_interrupts, println, sync::mutex::SpinMutex};
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
    /// Creates a new Scheduler instance with an idle thread.
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

    /// Adds a new thread to the scheduler's ready queue.
    pub fn add_thread(&mut self, thread: Thread) {
        let prio = thread.priority;
        let thread = Arc::new(SpinMutex::new(thread));
        self.ready_queue[prio as usize].push_back(thread);
    }

    /// Returns the current thread being executed.
    pub fn get_current_thread(&self) -> Arc<SpinMutex<Thread>> {
        self.current_thread.clone()
    }

    /// Schedules the next thread to run.
    pub fn schedule(&mut self) {
        // Get current thread info
        let (mut current_sp, current_prio, current_status) = {
            let locked = self.current_thread.lock();
            (locked.stack_pointer, locked.priority, locked.status)
        };

        // Get the next thread to run
        let next_thread = self.get_next_thread();
        let next_sp = {
            let mut next_locked = next_thread.lock();
            next_locked.status = Status::Running;
            next_locked.stack_pointer
        };

        // Update the current thread status and move it to the ready queue if it was running
        if current_status == Status::Running {
            {
                let mut current_locked = self.current_thread.lock();
                current_locked.status = Status::Ready;
            }
            self.ready_queue[current_prio as usize].push_back(self.current_thread.clone());
        }

        // Switch to the next thread
        self.current_thread = Arc::clone(&next_thread);

        switch(&mut current_sp, &next_sp);
    }

    /// Reschedules the next thread to run, disabling interrupts during the operation.
    pub fn reschedule(&mut self) {
        no_interrupts(|| self.schedule());
    }

    /// Returns the next thread to be scheduled, based on priority and status.
    fn get_next_thread(&mut self) -> Arc<SpinMutex<Thread>> {
        for prio in 0..self.ready_queue.len() {
            while let Some(thread) = self.ready_queue[prio].pop_front() {
                if thread.lock().status == Status::Ready {
                    return thread.clone();
                }
            }
        }
        self.idle_thread.clone()
    }
}
