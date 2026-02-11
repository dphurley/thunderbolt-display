#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SequenceNumber(u32);

impl SequenceNumber {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn value(self) -> u32 {
        self.0
    }

    pub fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::SequenceNumber;

    #[test]
    fn next_wraps_on_overflow() {
        let start = SequenceNumber::new(u32::MAX);
        let next = start.next();
        assert_eq!(next.value(), 0);
    }
}
