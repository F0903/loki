use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
/// Wrapper for types that may include things like pointers, but are known to be safe to send across threads.
pub struct UnsafeSendWrapper<T>(pub T);

unsafe impl<T> Send for UnsafeSendWrapper<T> {}
unsafe impl<T> Sync for UnsafeSendWrapper<T> {}

impl<T> UnsafeSendWrapper<T> {
    pub fn take_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for UnsafeSendWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for UnsafeSendWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
