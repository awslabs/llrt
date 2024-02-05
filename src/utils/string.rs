pub trait JoinToString<T> {
    fn join_to_string<F>(&mut self, separator: &str, f: F) -> String
    where
        F: FnMut(&T) -> &str;
}

impl<T, I> JoinToString<T> for I
where
    I: Iterator<Item = T>,
{
    fn join_to_string<F>(&mut self, separator: &str, mut f: F) -> String
    where
        F: FnMut(&T) -> &str,
    {
        let mut result = String::new();

        if let Some(first_item) = self.next() {
            result.push_str(f(&first_item));

            for item in self {
                result.push_str(separator);
                result.push_str(f(&item));
            }
        }

        result
    }
}
