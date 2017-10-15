fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod test {
    #[test]
    fn do_test() {
        assert!(1 + 1 == 2);
    }
}
