pub mod console;
pub mod markdown;

pub(crate) fn title_case(value: &str) -> String {
    value
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::title_case;

    #[test]
    fn title_case_handles_empty_string() {
        assert_eq!(title_case(""), "");
    }

    #[test]
    fn title_case_handles_hyphens() {
        assert_eq!(title_case("named-function"), "Named Function");
        assert_eq!(title_case("named-functions"), "Named Functions");
    }
}
