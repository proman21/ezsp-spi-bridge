use std::{
    borrow::Borrow,
    cell::UnsafeCell,
    io::Read,
    iter::{Copied, Enumerate},
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut, RangeFrom},
    slice::{from_raw_parts_mut, Iter},
};

use bytes::{buf::UninitSlice, Buf, BufMut, Bytes, BytesMut};
use nom::{Compare, CompareResult, IResult, InputIter, InputLength, InputTake, Needed, Slice};

#[derive(Debug, Default)]
pub struct Buffer<'a> {
    inner: UnsafeCell<BytesMut>,
    _phantom: PhantomData<&'a ()>,
}

pub type ParserResult<'a, O> = IResult<Buffer<'a>, O>;

impl<'a> Buffer<'a> {
    fn construct(inner: UnsafeCell<BytesMut>) -> Buffer<'a> {
        Buffer {
            inner,
            _phantom: PhantomData,
        }
    }

    fn get(&self) -> &'a BytesMut {
        unsafe { &*self.inner.get() }
    }

    #[allow(clippy::mut_from_ref)]
    fn get_mut(&'a self) -> &'a mut BytesMut {
        unsafe { &mut *self.inner.get() }
    }

    fn into_inner(self) -> BytesMut {
        self.inner.into_inner()
    }

    pub fn capacity(&self) -> usize {
        self.get().capacity()
    }

    pub fn clear(&mut self) {
        self.get_mut().clear();
    }

    pub fn extend_from_slice(&mut self, extend: &[u8]) {
        self.get_mut().extend_from_slice(extend);
    }

    pub fn is_empty(&self) -> bool {
        self.get().is_empty()
    }

    pub fn split_to(&mut self, at: usize) -> Self {
        Buffer::from(self.get_mut().split_to(at))
    }

    pub fn split_off(&mut self, at: usize) -> Self {
        Buffer::from(self.get_mut().split_off(at))
    }

    pub fn split(&mut self) -> Buffer {
        Buffer::from(self.get_mut().split())
    }

    pub fn freeze(self) -> Bytes {
        self.into_inner().freeze()
    }

    pub fn len(&self) -> usize {
        self.get().len()
    }

    pub unsafe fn set_len(&mut self, len: usize) {
        self.get_mut().set_len(len);
    }

    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        self.get_mut().spare_capacity_mut()
    }

    pub fn fill_from_reader<R: Read>(&mut self, mut reader: R) -> std::io::Result<usize> {
        let spare_cap = self.spare_capacity_mut();
        let read_buf =
            unsafe { from_raw_parts_mut(spare_cap.as_mut_ptr() as *mut u8, spare_cap.len()) };

        let read = reader.read(read_buf)?;

        unsafe { self.set_len(self.len() + read) }
        Ok(read)
    }
}

impl Clone for Buffer<'_> {
    fn clone(&self) -> Self {
        Buffer::from(self.get().clone())
    }
}

impl Buf for Buffer<'_> {
    fn remaining(&self) -> usize {
        self.get().remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.get().chunk()
    }

    fn advance(&mut self, cnt: usize) {
        self.get_mut().advance(cnt)
    }

    fn copy_to_bytes(&mut self, len: usize) -> Bytes {
        self.get_mut().copy_to_bytes(len)
    }
}

unsafe impl BufMut for Buffer<'_> {
    fn remaining_mut(&self) -> usize {
        self.get().remaining_mut()
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.get_mut().advance_mut(cnt)
    }

    fn chunk_mut(&mut self) -> &mut UninitSlice {
        self.get_mut().chunk_mut()
    }

    fn put<T: Buf>(&mut self, src: T)
    where
        Self: Sized,
    {
        self.get_mut().put(src)
    }

    fn put_slice(&mut self, src: &[u8]) {
        self.get_mut().put_slice(src)
    }

    fn put_bytes(&mut self, val: u8, cnt: usize) {
        self.get_mut().put_bytes(val, cnt)
    }
}

impl From<BytesMut> for Buffer<'_> {
    fn from(value: BytesMut) -> Self {
        Buffer::construct(UnsafeCell::new(value))
    }
}

impl<'a> From<&'a [u8]> for Buffer<'a> {
    fn from(value: &'a [u8]) -> Self {
        Buffer::from(BytesMut::from(value))
    }
}

impl Deref for Buffer<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl DerefMut for Buffer<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl Borrow<[u8]> for Buffer<'_> {
    fn borrow(&self) -> &[u8] {
        self.get().borrow()
    }
}

impl Slice<RangeFrom<usize>> for Buffer<'_> {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        Buffer::from(self.get_mut().split_off(range.start))
    }
}

impl<'a> InputIter for Buffer<'a> {
    type Item = u8;

    type Iter = Enumerate<<Self as InputIter>::IterElem>;

    type IterElem = Copied<Iter<'a, u8>>;

    fn iter_indices(&self) -> Self::Iter {
        self.get().iter().copied().enumerate()
    }

    fn iter_elements(&self) -> Self::IterElem {
        self.get().iter().copied()
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.iter().copied().position(predicate)
    }

    fn slice_index(&self, count: usize) -> Result<usize, Needed> {
        if self.len() >= count {
            Ok(count)
        } else {
            Err(Needed::new(count - self.len()))
        }
    }
}

impl InputLength for Buffer<'_> {
    fn input_len(&self) -> usize {
        self.len()
    }
}

impl InputTake for Buffer<'_> {
    fn take(&self, count: usize) -> Self {
        Buffer::from(self.get_mut().split_to(count))
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        (Buffer::from(self.get_mut().split_to(count)), self.clone())
    }
}

impl<T: AsRef<[u8]>> Compare<T> for Buffer<'_> {
    fn compare(&self, t: T) -> CompareResult {
        (self.get().as_ref()).compare(t.as_ref())
    }

    fn compare_no_case(&self, t: T) -> CompareResult {
        (self.get().as_ref()).compare_no_case(t.as_ref())
    }
}
