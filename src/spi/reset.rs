use super::inner::Inner;

pub struct Reset<'a> {
    spi: &'a mut Inner,
}
