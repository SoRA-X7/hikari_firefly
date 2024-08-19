mod mem;
mod search;

pub struct HikariFireflyBot {
    root_state: search::Generation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(42, 42);
    }
}
