pub enum Error {
    Full,
    Empty,
    LastOfCode
}

impl Error {
    fn cast_usize(&self) -> usize {
        match self {
            &Error::Full => 0,
            &Error::Empty => 1,
            _ => panic!("LastOfCode")
        }
    }
    pub fn name(&self) -> &'static str {
        ["Full", "Empty"][self.cast_usize()]
    }
}
