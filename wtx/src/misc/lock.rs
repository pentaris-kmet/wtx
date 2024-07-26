use alloc::{rc::Rc, sync::Arc};
use core::{
  cell::{RefCell, RefMut},
  future::poll_fn,
  ops::DerefMut,
  task::Poll,
};

/// An asynchronous mutual exclusion primitive useful for protecting shared data.
pub trait Lock {
  /// See [`LockGuard`].
  type Guard<'guard>: DerefMut<Target = Self::Resource>
  where
    Self: 'guard;
  /// Resource behind the lock.
  type Resource;

  /// Generic way to build a lock.
  fn new(resource: Self::Resource) -> Self;

  /// Locks this element, causing the current task to yield until the lock has been acquired. When
  /// the lock has been acquired, returns a guard.
  fn lock(&self) -> impl Future<Output = Self::Guard<'_>>;
}

impl<T> Lock for Arc<T>
where
  T: Lock,
{
  type Guard<'guard> = T::Guard<'guard>
  where
    Self: 'guard;
  type Resource = T::Resource;

  #[inline]
  fn new(resource: Self::Resource) -> Self {
    Arc::new(T::new(resource))
  }

  #[inline]
  async fn lock(&self) -> Self::Guard<'_> {
    (**self).lock().await
  }
}

impl<T> Lock for RefCell<T> {
  type Guard<'guard> = RefMut<'guard, T>
  where
    Self: 'guard;
  type Resource = T;

  #[inline]
  fn new(resource: T) -> Self {
    RefCell::new(resource)
  }

  #[inline]
  fn lock(&self) -> impl Future<Output = Self::Guard<'_>> {
    poll_fn(
      |_| {
        if let Ok(elem) = self.try_borrow_mut() {
          Poll::Ready(elem)
        } else {
          Poll::Pending
        }
      },
    )
  }
}

impl<T> Lock for Rc<T>
where
  T: Lock,
{
  type Guard<'guard> = T::Guard<'guard>
  where
    Self: 'guard;
  type Resource = T::Resource;

  #[inline]
  fn new(resource: Self::Resource) -> Self {
    Rc::new(T::new(resource))
  }

  #[inline]
  async fn lock(&self) -> Self::Guard<'_> {
    (**self).lock().await
  }
}

/// Synchronous counterpart of [Lock].
pub trait SyncLock {
  /// See [`LockGuard`].
  type Guard<'guard>: DerefMut<Target = Self::Resource>
  where
    Self: 'guard;
  /// Resource behind the lock.
  type Resource;

  /// Generic way to build a lock.
  fn new(resource: Self::Resource) -> Self;

  /// Locks this element, causing the current thread to yield until the lock has been acquired. When
  /// the lock has been acquired, returns a guard.
  fn lock(&self) -> Self::Guard<'_>;
}

impl<T> SyncLock for Arc<T>
where
  T: SyncLock,
{
  type Guard<'guard> = T::Guard<'guard>
  where
    Self: 'guard;
  type Resource = T::Resource;

  #[inline]
  fn new(resource: Self::Resource) -> Self {
    Arc::new(T::new(resource))
  }

  #[inline]
  fn lock(&self) -> Self::Guard<'_> {
    (**self).lock()
  }
}

impl<T> SyncLock for Rc<T>
where
  T: SyncLock,
{
  type Guard<'guard> = T::Guard<'guard>
  where
    Self: 'guard;
  type Resource = T::Resource;

  #[inline]
  fn new(resource: Self::Resource) -> Self {
    Rc::new(T::new(resource))
  }

  #[inline]
  fn lock(&self) -> Self::Guard<'_> {
    (**self).lock()
  }
}

#[cfg(feature = "embassy-sync")]
mod embassy {
  use crate::misc::Lock;
  use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    mutex::{Mutex, MutexGuard},
  };

  impl<M, T> Lock for Mutex<M, T>
  where
    M: RawMutex,
  {
    type Guard<'guard> = MutexGuard<'guard, M, Self::Resource>
    where
      Self: 'guard;
    type Resource = T;

    #[inline]
    fn new(resource: Self::Resource) -> Self {
      Mutex::new(resource)
    }

    #[inline]
    async fn lock(&self) -> Self::Guard<'_> {
      (*self).lock().await
    }
  }
}

#[cfg(feature = "parking_lot")]
mod parking_lot {
  use crate::misc::SyncLock;
  use parking_lot::{Mutex, MutexGuard};

  impl<T> SyncLock for Mutex<T> {
    type Guard<'guard> = MutexGuard<'guard, Self::Resource>
    where
      Self: 'guard;
    type Resource = T;

    #[inline]
    fn new(resource: Self::Resource) -> Self {
      Mutex::new(resource)
    }

    #[inline]
    fn lock(&self) -> Self::Guard<'_> {
      (*self).lock()
    }
  }
}

#[cfg(feature = "std")]
mod std {
  use crate::misc::SyncLock;
  use std::sync::{Mutex, MutexGuard, TryLockError};

  impl<T> SyncLock for Mutex<T> {
    type Guard<'guard> = MutexGuard<'guard, Self::Resource>
    where
      Self: 'guard;
    type Resource = T;

    #[inline]
    fn new(resource: Self::Resource) -> Self {
      Mutex::new(resource)
    }

    #[inline]
    fn lock(&self) -> Self::Guard<'_> {
      loop {
        let rslt = (*self).try_lock();
        match rslt {
          Err(TryLockError::Poisoned(elem)) => return elem.into_inner(),
          Err(TryLockError::WouldBlock) => {}
          Ok(elem) => return elem,
        }
      }
    }
  }
}

#[cfg(feature = "tokio")]
mod tokio {
  use crate::misc::Lock;
  use tokio::sync::{Mutex, MutexGuard};

  impl<T> Lock for Mutex<T> {
    type Guard<'guard> = MutexGuard<'guard, Self::Resource>
    where
      Self: 'guard;
    type Resource = T;

    #[inline]
    fn new(resource: Self::Resource) -> Self {
      Mutex::new(resource)
    }

    #[inline]
    async fn lock(&self) -> Self::Guard<'_> {
      (*self).lock().await
    }
  }
}
