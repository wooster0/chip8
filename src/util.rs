/// An iterator over the bits of a byte as `bool`s, from left to right, or right to left with `rev`.
///
/// ```
/// let mut bits = Bits::new(0b0110_1001);
///
/// assert_eq!(bits.next(), Some(true));
/// assert_eq!(bits.next(), Some(false));
/// assert_eq!(bits.next(), Some(false));
/// assert_eq!(bits.next(), Some(true));
/// assert_eq!(bits.next(), Some(false));
/// assert_eq!(bits.next(), Some(true));
/// assert_eq!(bits.next(), Some(true));
/// assert_eq!(bits.next(), Some(false));
/// assert_eq!(bits.next(), None);
/// ```
pub struct Bits {
    byte: u8,
    index: u8,
}

impl Bits {
    pub fn new(byte: u8) -> Self {
        Self { byte, index: 0 }
    }
}

impl Iterator for Bits {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        let bit = if let Some(rhs) = (u8::BITS as u8 - 1).checked_sub(self.index) {
            (self.byte >> rhs) & 1
        } else {
            return None;
        };

        self.index += 1;

        Some(bit == 1)
    }
}

impl DoubleEndedIterator for Bits {
    fn next_back(&mut self) -> Option<Self::Item> {
        let bit = if let Some(lhs) = self.byte.checked_shr(self.index as u32) {
            lhs & 1
        } else {
            return None;
        };

        self.index += 1;

        Some(bit == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bits() {
        let mut bits = Bits::new(0b0110_1001);

        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(false));

        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), None);

        let mut bits = Bits::new(0b1100_1100).rev();

        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(true));

        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), None);

        let mut bits = Bits::new(0b0000_0000);

        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(false));

        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), None);

        let mut bits = Bits::new(0b1111_1111);

        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(true));

        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), None);

        let mut bits = Bits::new(0b0101_0101);

        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(true));

        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), None);

        let mut bits = Bits::new(0b1010_1010);

        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(false));

        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), None);
    }
}
