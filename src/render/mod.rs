pub mod console;
pub mod markdown;

pub(crate) fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::title_case;

    #[test]
    fn title_case_handles_empty_string() {
        assert_eq!(title_case(""), "");
    }
}
