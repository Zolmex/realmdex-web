use crate::types::ServerInput;

#[derive(Debug, PartialEq)]
pub struct ValidationError(pub String);

pub fn validate_server_input(input: &ServerInput) -> Result<(), ValidationError> {
    if input.name.is_empty() {
        return Err(ValidationError("name is required".into()));
    }
    if input.name.len() > 100 {
        return Err(ValidationError("name must be 100 chars or fewer".into()));
    }
    if !input.host.starts_with("http://") && !input.host.starts_with("https://") {
        return Err(ValidationError("host must start with http:// or https://".into()));
    }
    if let Some(ref icon) = input.icon_path {
        if icon.len() > 500 {
            return Err(ValidationError("icon_path must be 500 chars or fewer".into()));
        }
    }
    if let Some(ref link) = input.discord_link {
        if link.len() > 500 {
            return Err(ValidationError("discord_link must be 500 chars or fewer".into()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Category;

    fn valid_input() -> ServerInput {
        ServerInput {
            name: "Test Server".into(),
            host: "https://example.com/players".into(),
            category: Category::Pserver,
            icon_path: None,
            discord_link: None,
            is_wip: false,
            polled: true,
        }
    }

    #[test]
    fn accepts_valid_input() {
        assert!(validate_server_input(&valid_input()).is_ok());
    }

    #[test]
    fn rejects_empty_name() {
        let mut input = valid_input();
        input.name = String::new();
        assert_eq!(
            validate_server_input(&input),
            Err(ValidationError("name is required".into()))
        );
    }

    #[test]
    fn rejects_long_name() {
        let mut input = valid_input();
        input.name = "x".repeat(101);
        assert_eq!(
            validate_server_input(&input),
            Err(ValidationError("name must be 100 chars or fewer".into()))
        );
    }

    #[test]
    fn rejects_javascript_host() {
        let mut input = valid_input();
        input.host = "javascript:alert(1)".into();
        assert_eq!(
            validate_server_input(&input),
            Err(ValidationError("host must start with http:// or https://".into()))
        );
    }

    #[test]
    fn rejects_data_uri_host() {
        let mut input = valid_input();
        input.host = "data:text/html,<script>alert(1)</script>".into();
        assert_eq!(
            validate_server_input(&input),
            Err(ValidationError("host must start with http:// or https://".into()))
        );
    }

    #[test]
    fn rejects_file_host() {
        let mut input = valid_input();
        input.host = "file:///etc/passwd".into();
        assert_eq!(
            validate_server_input(&input),
            Err(ValidationError("host must start with http:// or https://".into()))
        );
    }

    #[test]
    fn accepts_http_host() {
        let mut input = valid_input();
        input.host = "http://localhost:9001/players".into();
        assert!(validate_server_input(&input).is_ok());
    }

    #[test]
    fn rejects_long_icon_path() {
        let mut input = valid_input();
        input.icon_path = Some("x".repeat(501));
        assert_eq!(
            validate_server_input(&input),
            Err(ValidationError("icon_path must be 500 chars or fewer".into()))
        );
    }

    #[test]
    fn rejects_long_discord_link() {
        let mut input = valid_input();
        input.discord_link = Some("x".repeat(501));
        assert_eq!(
            validate_server_input(&input),
            Err(ValidationError("discord_link must be 500 chars or fewer".into()))
        );
    }
}
