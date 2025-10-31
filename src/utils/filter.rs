/// Splits a search query value into terms to be
/// case insensitively matched
///
/// testTerm = test, Term
/// TestTerm = Test, Term
/// test1term test, 1, term
/// test term = test, term
/// test#term = test, #, test
/// ..etc
pub fn split_search_terms(value: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut prev_char: Option<char> = None;

    for c in value.chars() {
        if let Some(pc) = prev_char {
            let split = (pc.is_lowercase() && c.is_uppercase()) // camelCase
            || (pc.is_alphabetic() != c.is_alphabetic()) // letter -> number
            || (pc.is_numeric() != c.is_numeric())       // number -> letter
            || (!pc.is_alphanumeric() && c.is_alphanumeric()) // punctuation -> word
            || (pc.is_alphanumeric() && !c.is_alphanumeric()); // word -> punctuation

            if split && !current.is_empty() {
                words.push(current.clone());
                current.clear();
            }
        }

        current.push(c);
        prev_char = Some(c);
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}

#[cfg(test)]
mod test {
    use crate::utils::filter::split_search_terms;

    #[test]
    fn test_case_change() {
        assert_eq!(
            split_search_terms("testTerm"),
            vec!["test".to_string(), "Term".to_string()]
        );
        assert_eq!(
            split_search_terms("testTerm"),
            vec!["test".to_string(), "Term".to_string()]
        );
        assert_eq!(
            split_search_terms("testTermSecond"),
            vec!["test".to_string(), "Term".to_string(), "Second".to_string()]
        );
        assert_eq!(
            split_search_terms("TestTermSecond"),
            vec!["Test".to_string(), "Term".to_string(), "Second".to_string()]
        );
    }

    #[test]
    fn test_alphabetic_to_numeric() {
        assert_eq!(
            split_search_terms("test12Term"),
            vec!["test".to_string(), "12".to_string(), "Term".to_string()]
        );
        assert_eq!(
            split_search_terms("test1term"),
            vec!["test".to_string(), "1".to_string(), "term".to_string()]
        );

        assert_eq!(
            split_search_terms("1testTerm"),
            vec!["1".to_string(), "test".to_string(), "Term".to_string()]
        );
        assert_eq!(
            split_search_terms("testTermSecond19"),
            vec![
                "test".to_string(),
                "Term".to_string(),
                "Second".to_string(),
                "19".to_string()
            ]
        );
        assert_eq!(
            split_search_terms("Test1Term5Second"),
            vec![
                "Test".to_string(),
                "1".to_string(),
                "Term".to_string(),
                "5".to_string(),
                "Second".to_string()
            ]
        );
    }

    #[test]
    fn test_punctuation() {
        assert_eq!(
            split_search_terms("test12#Term"),
            vec![
                "test".to_string(),
                "12".to_string(),
                "#".to_string(),
                "Term".to_string()
            ]
        );
        assert_eq!(
            split_search_terms("test1term#"),
            vec![
                "test".to_string(),
                "1".to_string(),
                "term".to_string(),
                "#".to_string()
            ]
        );

        assert_eq!(
            split_search_terms("1#testTerm"),
            vec![
                "1".to_string(),
                "#".to_string(),
                "test".to_string(),
                "Term".to_string()
            ]
        );
        assert_eq!(
            split_search_terms("testTermSecond19##"),
            vec![
                "test".to_string(),
                "Term".to_string(),
                "Second".to_string(),
                "19".to_string(),
                "##".to_string(),
            ]
        );
        assert_eq!(
            split_search_terms("Test1Term5Second_Test"),
            vec![
                "Test".to_string(),
                "1".to_string(),
                "Term".to_string(),
                "5".to_string(),
                "Second".to_string(),
                "_".to_string(),
                "Test".to_string()
            ]
        );
        assert_eq!(
            split_search_terms("Test.1Term5Second_Test"),
            vec![
                "Test".to_string(),
                ".".to_string(),
                "1".to_string(),
                "Term".to_string(),
                "5".to_string(),
                "Second".to_string(),
                "_".to_string(),
                "Test".to_string()
            ]
        );
    }
}
