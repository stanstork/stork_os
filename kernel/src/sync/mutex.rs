use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

// SpinMutex is a mutual exclusion primitive that uses a spinlock mechanism.
// T: ?Sized allows SpinMutex to be used with dynamically sized types.
pub struct SpinMutex<T: ?Sized> {
    // AtomicBool is used for the lock state, allowing atomic operations on the lock.
    lock: AtomicBool,
    // UnsafeCell allows mutable access to data even when the SpinMutex is immutable.
    data: UnsafeCell<T>,
}

// SpinMutexGuard is a guard object that provides access to the data protected by the SpinMutex
// and ensures the lock is released when the guard is dropped.
pub struct SpinMutexGuard<'a, T: ?Sized + 'a> {
    // Reference to the lock state in the SpinMutex.
    lock: &'a AtomicBool,
    // Mutable reference to the data protected by the SpinMutex.
    data: &'a mut T,
}

// Unsafe impls to ensure thread safety:
// Sync allows SpinMutex to be shared between threads.
// Send allows SpinMutex to be transferred between threads.
unsafe impl<T: ?Sized + Send> Sync for SpinMutex<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinMutex<T> {}

impl<T> SpinMutex<T> {
    // Creates a new SpinMutex with the given data and an unlocked state.
    pub const fn new(data: T) -> SpinMutex<T> {
        SpinMutex {
            lock: AtomicBool::new(false), // false = unlocked
            data: UnsafeCell::new(data),
        }
    }

    // Acquires the lock, spinning (busy-waiting) until it becomes available.
    // Returns a SpinMutexGuard that provides access to the protected data.
    pub fn lock(&self) -> SpinMutexGuard<T> {
        // Attempt to acquire the lock by setting `locked` to true.
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Busy wait (spin) until the lock might be available.
            while self.lock.load(Ordering::Relaxed) {}
        }
        // Lock acquired, return a SpinMutexGuard.
        SpinMutexGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
        }
    }
}

// Implements Deref for SpinMutexGuard to provide read-only access to the protected data.
impl<'a, T: ?Sized> Deref for SpinMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

// Implements DerefMut for SpinMutexGuard to provide mutable access to the protected data.
impl<'a, T: ?Sized> DerefMut for SpinMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

// Implements Drop for SpinMutexGuard to release the lock when the guard goes out of scope.
impl<'a, T: ?Sized> Drop for SpinMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release); // Release the lock.
    }
}
