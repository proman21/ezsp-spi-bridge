use std::{
    borrow::{Borrow, BorrowMut},
    iter::Enumerate,
    mem::MaybeUninit,
    ops::{Deref, DerefMut, RangeFrom},
};

use bytes::{
    buf::{IntoIter, UninitSlice},
    Buf, BufMut, Bytes, BytesMut,
};
use nom::{Compare, InputIter, InputLength, InputTake, Slice};

use super::Inner;

#[derive(Debug, Clone, Default)]
pub struct BufferMut(Inner<BytesMut>);

impl BufferMut {
    pub fn with_capacity(capacity: usize) -> BufferMut {
        BufferMut(Inner::new(BytesMut::with_capacity(capacity)))
    }

    pub fn new() -> BufferMut {
        BufferMut(Inner::new(BytesMut::new()))
    }

    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.0.borrow().capacity()
    }

    pub fn freeze(self) -> Bytes {
        self.0.into_inner().freeze()
    }

    pub fn zeroed(len: usize) -> BufferMut {
        BufferMut(Inner::new(BytesMut::zeroed(len)))
    }

    pub fn split_off(&mut self, at: usize) -> BufferMut {
        BufferMut(self.0.split_mut(|b| b.split_off(at)))
    }

    pub fn split(&mut self) -> BufferMut {
        BufferMut(self.0.split_mut(|b| b.split()))
    }

    pub fn split_to(&mut self, at: usize) -> BufferMut {
        BufferMut(self.0.split_mut(|b| b.split_to(at)))
    }

    pub fn truncate(&mut self, len: usize) {
        self.0.get_mut().truncate(len);
    }

    pub fn clear(&mut self) {
        self.0.get_mut().clear();
    }

    pub fn resize(&mut self, new_len: usize, value: u8) {
        self.0.get_mut().resize(new_len, value);
    }

    pub unsafe fn set_len(&mut self, len: usize) {
        self.0.get_mut().set_len(len);
    }

    pub fn reserve(&mut self, additional: usize) {
        self.0.get_mut().reserve(additional);
    }

    pub fn extend_from_slice(&mut self, extend: &[u8]) {
        self.0.get_mut().extend_from_slice(extend);
    }

    pub fn unsplit(&mut self, other: BufferMut) {
        self.0.get_mut().unsplit(other.into_inner())
    }

    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        self.0.get_mut().spare_capacity_mut()
    }

    pub fn into_inner(self) -> BytesMut {
        self.0.into_inner()
    }
}

impl Buf for BufferMut {
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

unsafe impl BufMut for BufferMut {
    fn remaining_mut(&self) -> usize {
        self.0.borrow().remaining_mut()
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.0.get_mut().advance_mut(cnt)
    }

    fn chunk_mut(&mut self) -> &mut UninitSlice {
        self.0.get_mut().chunk_mut()
    }

    fn put<T: Buf>(&mut self, src: T)
    where
        Self: Sized,
    {
        self.0.get_mut().put(src)
    }

    fn put_slice(&mut self, src: &[u8]) {
        self.0.get_mut().put_slice(src)
    }

    fn put_bytes(&mut self, val: u8, cnt: usize) {
        self.0.get_mut().put_bytes(val, cnt)
    }
}

impl Slice<RangeFrom<usize>> for BufferMut {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        // We want the semantics of `slice` to be similar to `InputTake`, in
        // that the parent and its children point to mutually exclusive ranges.
        let inner = unsafe { self.0.split(|b| b.split_off(range.start)) };
        BufferMut(inner)
    }
}

impl InputLength for BufferMut {
    fn input_len(&self) -> usize {
        self.len()
    }
}

impl InputIter for BufferMut {
    type Item = u8;

    type Iter = Enumerate<Self::IterElem>;

    type IterElem = IntoIter<BytesMut>;

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

impl InputTake for BufferMut {
    fn take(&self, count: usize) -> Self {
        let inner = unsafe { self.0.split(|b| b.split_to(count)) };
        BufferMut(inner)
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let inner = unsafe { self.0.split(|b| b.split_to(count)) };
        let prefix = BufferMut(inner);
        (self.clone(), prefix)
    }
}

impl<T> Compare<T> for BufferMut
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

impl AsRef<[u8]> for BufferMut {
    fn as_ref(&self) -> &[u8] {
        self.0.borrow().as_ref()
    }
}

impl AsMut<[u8]> for BufferMut {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.get_mut().as_mut()
    }
}

impl Borrow<[u8]> for BufferMut {
    fn borrow(&self) -> &[u8] {
        self.0.borrow().borrow()
    }
}

impl BorrowMut<[u8]> for BufferMut {
    fn borrow_mut(&mut self) -> &mut [u8] {
        self.0.get_mut().borrow_mut()
    }
}

impl Deref for BufferMut {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BufferMut {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for BufferMut
where
    T: Into<BytesMut>,
{
    fn from(value: T) -> Self {
        BufferMut(Inner::new(value.into()))
    }
}

impl FromIterator<u8> for BufferMut {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        BufferMut::from(BytesMut::from_iter(iter))
    }
}

impl<'a> FromIterator<&'a u8> for BufferMut {
    fn from_iter<T: IntoIterator<Item = &'a u8>>(iter: T) -> Self {
        BufferMut::from(BytesMut::from_iter(iter))
    }
}

unsafe impl Send for BufferMut {}
unsafe impl Sync for BufferMut {}
