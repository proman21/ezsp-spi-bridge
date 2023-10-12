use std::{
    cell::UnsafeCell,
    iter::Enumerate,
    ops::{Deref, DerefMut, RangeFrom},
};

use bytes::{buf::IntoIter, Bytes};
use nom::{Compare, InputIter, InputLength, InputTake, Slice};

/// Wrapper around a Bytes struct that implements the necessary traits to use
/// with the nom parser library.
#[derive(Debug, Default)]
pub struct Buffer(UnsafeCell<Bytes>);

impl Buffer {
    fn borrow(&self) -> &Bytes {
        unsafe { &*self.0.get() }
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn borrow_mut(&self) -> &mut Bytes {
        &mut *self.0.get()
    }

    pub const fn new() -> Self {
        Buffer(UnsafeCell::new(Bytes::new()))
    }

    pub const fn from_static(bytes: &'static [u8]) -> Self {
        Buffer(UnsafeCell::new(Bytes::from_static(bytes)))
    }

    pub fn into_inner(self) -> Bytes {
        self.0.into_inner()
    }
}

impl From<Bytes> for Buffer {
    fn from(value: Bytes) -> Self {
        Self(value.into())
    }
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        Self(self.borrow().clone().into())
    }
}

impl Deref for Buffer {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        self.borrow()
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_mut()
    }
}

impl Slice<RangeFrom<usize>> for Buffer {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        // This usage of Slice is different from BufferMut, as we are OK if the parent Buffer overlaps with its children
        Self::from(self.deref().slice(range))
    }
}

impl InputIter for Buffer {
    type Item = u8;

    type Iter = Enumerate<Self::IterElem>;

    type IterElem = IntoIter<Bytes>;

    fn iter_indices(&self) -> Self::Iter {
        self.iter_elements().enumerate()
    }

    fn iter_elements(&self) -> Self::IterElem {
        self.borrow().clone().into_iter()
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.iter_elements().position(predicate)
    }

    fn slice_index(&self, count: usize) -> Result<usize, nom::Needed> {
        if self.len() >= count {
            Ok(count)
        } else {
            Err(nom::Needed::new(count - self.len()))
        }
    }
}

impl InputLength for Buffer {
    fn input_len(&self) -> usize {
        self.len()
    }
}

impl InputTake for Buffer {
    fn take(&self, count: usize) -> Self {
        let inner = unsafe { self.borrow_mut().split_to(count) };
        Self(inner.into())
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let inner = unsafe { self.borrow_mut().split_to(count) };
        let prefix = Self(inner.into());
        (self.clone(), prefix)
    }
}

impl<T> Compare<T> for Buffer
where
    T: AsRef<[u8]>,
{
    fn compare(&self, t: T) -> nom::CompareResult {
        (self.as_ref()).compare(t.as_ref())
    }

    fn compare_no_case(&self, t: T) -> nom::CompareResult {
        (self.as_ref()).compare_no_case(t.as_ref())
    }
}
