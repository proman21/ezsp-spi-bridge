pub fn randomize_data(buf: &mut [u8]) {
    let mut reg: u8 = 0x42;
    for item in buf {
        *item ^= reg;
        reg = (reg >> 1) ^ ((reg & 0x01) * 0xB8)
    }
}

#[cfg(test)]
mod tests {
    use crate::ash::randomizer::randomize_data;

    #[test]
    fn it_computes_the_correct_sequence() {
        let mut buf = [0u8; 5];
        randomize_data(&mut buf);
        assert_eq!(buf, [0x42, 0x21, 0xA8, 0x54, 0x2A])
    }
}
