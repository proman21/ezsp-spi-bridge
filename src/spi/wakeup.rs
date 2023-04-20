use super::inner::Inner;

pub struct Wakeup<'a> {
    inner: &'a mut Inner,
}
