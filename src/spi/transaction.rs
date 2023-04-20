use super::inner::Inner;

pub struct Transaction<'a> {
    inner: &'a mut Inner,
}
