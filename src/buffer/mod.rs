#[allow(clippy::module_inception)]
mod buffer;
mod buffer_mut;

use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

pub use self::buffer::Buffer;
pub use self::buffer_mut::BufferMut;

#[derive(Debug)]
pub struct Inner<T>(UnsafeCell<T>);

impl<T> Inner<T> {
    pub const fn new(inner: T) -> Inner<T> {
        Inner(UnsafeCell::new(inner))
    }

    pub fn borrow(&self) -> &T {
        unsafe { &*self.0.get() }
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn borrow_mut(&self) -> &mut T {
        &mut *self.0.get()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.0.get_mut()
    }

    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }

    pub unsafe fn split<F>(&self, mut f: F) -> Self
    where
        F: FnMut(&mut T) -> T,
    {
        let b = self.borrow_mut();
        Inner::new(f(b))
    }

    pub fn split_mut<F>(&mut self, mut f: F) -> Self
    where
        F: FnMut(&mut T) -> T,
    {
        Inner::new(f(self.get_mut()))
    }
}

impl<T> From<T> for Inner<T> {
    fn from(inner: T) -> Inner<T> {
        Inner::new(inner)
    }
}

impl<T> Default for Inner<T>
where
    T: Default,
{
    fn default() -> Self {
        Inner::new(Default::default())
    }
}

impl<T> Clone for Inner<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Inner::new(self.borrow().clone())
    }
}

impl<T> Deref for Inner<T>
where
    T: Deref,
{
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        self.borrow().deref()
    }
}

impl<T> DerefMut for Inner<T>
where
    T: DerefMut,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut().deref_mut()
    }
}

unsafe impl<T> Send for Inner<T> where T: Send {}
unsafe impl<T> Sync for Inner<T> where T: Sync {}
