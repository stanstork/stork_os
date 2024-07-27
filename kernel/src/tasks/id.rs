use core::sync::atomic::{AtomicUsize, Ordering};

pub trait IdAllocator<T> {
    fn next() -> T;
}

impl IdAllocator<Pid> for Pid {
    fn next() -> Pid {
        let next_id = MAX_PID.fetch_add(1, Ordering::SeqCst);
        Pid(next_id)
    }
}

impl IdAllocator<Tid> for Tid {
    fn next() -> Tid {
        let next_id = MAX_TID.fetch_add(1, Ordering::SeqCst);
        Tid(next_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Pid(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Tid(usize);

static MAX_PID: AtomicUsize = AtomicUsize::new(1);
static MAX_TID: AtomicUsize = AtomicUsize::new(1);
