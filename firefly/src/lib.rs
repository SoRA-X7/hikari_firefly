mod mem;
mod movegen;
mod search;

pub struct HikariFireflyBot {
    root_state: search::Generation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
