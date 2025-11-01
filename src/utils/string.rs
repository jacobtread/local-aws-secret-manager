use std::fmt::{Display, Write};

pub fn join_iter_string<I: Display>(mut iterator: impl Iterator<Item = I>, sep: &str) -> String {
    match iterator.next() {
        None => String::new(),
        Some(first_elt) => {
            // estimate lower bound of capacity needed
            let (lower, _) = iterator.size_hint();
            let mut result = String::with_capacity(sep.len() * lower);
            write!(&mut result, "{}", first_elt).unwrap();
            iterator.for_each(|elt| {
                result.push_str(sep);
                write!(&mut result, "{}", elt).unwrap();
            });
            result
        }
    }
}
