//! Synchronization extension traits for the panic-elimination sweep.
//!
//! `.lock().unwrap()` on a `Mutex`/`RwLock` panics if a previous holder panicked
//! while holding the guard (poisoning). In a long-lived daemon that turns one
//! handler's panic into a cascade that bricks every future lock. Recovering the
//! guard from the `PoisonError` is almost always the right call for the bus /
//! cache / counter state this codebase locks, and it removes a `.unwrap()` from
//! the daemon path (AGENTS.md → Functional Style & Safe Refactoring).
//!
//! Behaviour is **identical to `.lock().unwrap()` on the happy path**; the only
//! difference is on poison, where these recover the inner guard instead of
//! panicking. Use them anywhere a poisoned lock should not cascade.

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Lock a [`Mutex`], recovering the guard even if the lock was poisoned.
pub trait LockRecover<T: ?Sized> {
    /// Like `.lock().unwrap()` but recovers the guard on poison.
    fn lock_recover(&self) -> MutexGuard<'_, T>;
}

impl<T: ?Sized> LockRecover<T> for Mutex<T> {
    fn lock_recover(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Read/write an [`RwLock`], recovering the guard on poison (see [`LockRecover`]).
pub trait RwLockRecover<T: ?Sized> {
    fn read_recover(&self) -> RwLockReadGuard<'_, T>;
    fn write_recover(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T: ?Sized> RwLockRecover<T> for RwLock<T> {
    fn read_recover(&self) -> RwLockReadGuard<'_, T> {
        self.read().unwrap_or_else(|e| e.into_inner())
    }
    fn write_recover(&self) -> RwLockWriteGuard<'_, T> {
        self.write().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn mutex_lock_recover_happy_path() {
        let m = Mutex::new(41);
        *m.lock_recover() += 1;
        assert_eq!(*m.lock_recover(), 42);
    }

    #[test]
    fn mutex_lock_recover_after_poison_keeps_data() {
        let m = Arc::new(Mutex::new(vec![1, 2, 3]));
        let m2 = Arc::clone(&m);
        // Poison the mutex: panic while holding the guard.
        let _ = std::thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("poison it");
        })
        .join();
        assert!(m.lock().is_err(), "mutex should be poisoned");
        // lock_recover still returns the guard with the data intact.
        assert_eq!(*m.lock_recover(), vec![1, 2, 3]);
    }

    #[test]
    fn rwlock_recover_read_and_write() {
        let l = RwLock::new(10);
        *l.write_recover() = 20;
        assert_eq!(*l.read_recover(), 20);
    }

    #[test]
    fn rwlock_recover_after_poison() {
        let l = Arc::new(RwLock::new(7));
        let l2 = Arc::clone(&l);
        let _ = std::thread::spawn(move || {
            let _g = l2.write().unwrap();
            panic!("poison it");
        })
        .join();
        assert!(l.read().is_err(), "rwlock should be poisoned");
        assert_eq!(*l.read_recover(), 7);
    }
}
