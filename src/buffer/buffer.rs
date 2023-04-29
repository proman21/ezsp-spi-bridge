use std::{
    borrow::Borrow,
    iter::Enumerate,
    ops::{Deref, RangeBounds, RangeFrom},
};

use bytes::{buf::IntoIter, Buf, Bytes};
use nom::{Compare, InputIter, InputLength, InputTake, Slice};

use super::Inner;

#[derive(Debug, Clone, Default)]
pub struct Buffer(Inner<Bytes>);

impl Buffer {
    pub const fn new() -> Buffer {
        Buffer(Inner::new(Bytes::new()))
    }

    pub const fn from_static(bytes: &'static [u8]) -> Buffer {
        Buffer(Inner::new(Bytes::from_static(bytes)))
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    pub fn copy_from_slice(data: &[u8]) -> Buffer {
        Buffer(Inner::new(Bytes::copy_from_slice(data)))
    }

    pub fn slice(&self, range: impl RangeBounds<usize>) -> Buffer {
        Buffer(Inner::new(self.0.borrow().slice(range)))
    }

    pub fn slice_ref(&self, subset: &[u8]) -> Buffer {
        Buffer(Inner::new(self.0.borrow().slice_ref(subset)))
    }

    pub fn split_off(&mut self, at: usize) -> Self {
        Buffer(self.0.split_mut(|b| b.split_off(at)))
    }

    pub fn split_to(&mut self, at: usize) -> Self {
        Buffer(self.0.split_mut(|b| b.split_to(at)))
    }

    pub fn truncate(&mut self, len: usize) {
        self.0.get_mut().truncate(len);
    }

    pub fn clear(&mut self) {
        self.0.get_mut().clear();
    }

    pub fn into_inner(self) -> Bytes {
        self.0.into_inner()
    }
}

impl Buf for Buffer {
    fn remaining(&self) -> usize {
        self.0.borrow().remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.0.borrow().chunk()
    }

    fn advance(&mut self, cnt: usize) {
        self.0.get_mut().advance(cnt);
    }

    fn copy_to_bytes(&mut self, len: usize) -> Bytes {
        self.0.get_mut().copy_to_bytes(len)
    }
}

impl Slice<RangeFrom<usize>> for Buffer {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        // This usage of Slice is different from BufferMut, as we are OK if the parent Buffer overlaps with its children
        self.slice(range)
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
        self.0.borrow().clone().into_iter()
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
        let inner = unsafe { self.0.split(|b| b.split_to(count)) };
        Buffer(inner)
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let inner = unsafe { self.0.split(|b| b.split_to(count)) };
        let prefix = Buffer(inner);
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

impl Borrow<[u8]> for Buffer {
    fn borrow(&self) -> &[u8] {
        self.0.borrow().borrow()
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        self.0.borrow().as_ref()
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> From<T> for Buffer
where
    T: Into<Bytes>,
{
    fn from(value: T) -> Self {
        Buffer(Inner::new(value.into()))
    }
}

impl FromIterator<u8> for Buffer {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        Buffer::from(Bytes::from_iter(iter))
    }
}

unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}
