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
    if let Some(host) = extract_host(&input.host) {
        if is_private_host(host) {
            return Err(ValidationError("host must not point to a private/internal address".into()));
        }
    }
    if let Some(ref icon) = input.icon_path {
        if icon.len() > 500 {
            return Err(ValidationError("icon_path must be 500 chars or fewer".into()));
        }
        if !icon.starts_with('/') && !icon.starts_with("https://") && !icon.starts_with("http://") {
            return Err(ValidationError("icon_path must start with /, http://, or https://".into()));
        }
    }
    if let Some(ref link) = input.discord_link {
        if link.len() > 500 {
            return Err(ValidationError("discord_link must be 500 chars or fewer".into()));
        }
        if !link.starts_with("https://") && !link.starts_with("http://") {
            return Err(ValidationError("discord_link must start with http:// or https://".into()));
        }
    }
    Ok(())
}

fn extract_host(url: &str) -> Option<&str> {
    let after_scheme = url.strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    let authority = after_scheme.split('/').next()?;
    // strip port
    Some(authority.rsplit_once(':')
        .map(|(h, _)| h)
        .unwrap_or(authority))
}

fn is_private_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    if h == "localhost" || h == "[::1]" || h == "0.0.0.0" {
        return true;
    }
    let octets: Vec<u8> = h.split('.').filter_map(|s| s.parse().ok()).collect();
    if octets.len() == 4 {
        return matches!(
            (octets[0], octets[1]),
            (127, _) | (10, _) | (169, 254) | (192, 168) | (0, _)
        ) || (octets[0] == 172 && (16..=31).contains(&octets[1]));
    }
    false
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
        input.host = "http://203.0.113.1:9001/players".into();
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

    #[test]
    fn rejects_localhost_host() {
        let mut input = valid_input();
        input.host = "http://localhost:8080/stats".into();
        assert_eq!(
            validate_server_input(&input),
            Err(ValidationError("host must not point to a private/internal address".into()))
        );
    }

    #[test]
    fn rejects_private_ip_host() {
        for addr in &[
            "http://127.0.0.1/stats",
            "http://10.0.0.1/stats",
            "http://172.16.0.1/stats",
            "http://192.168.1.1/stats",
            "http://169.254.169.254/latest/meta-data/",
            "http://0.0.0.0/stats",
        ] {
            let mut input = valid_input();
            input.host = addr.to_string();
            assert!(
                validate_server_input(&input).is_err(),
                "{addr} should be rejected"
            );
        }
    }

    #[test]
    fn accepts_public_ip_host() {
        let mut input = valid_input();
        input.host = "http://198.244.151.113:2052/realmdex/stats".into();
        assert!(validate_server_input(&input).is_ok());
    }

    #[test]
    fn rejects_javascript_discord_link() {
        let mut input = valid_input();
        input.discord_link = Some("javascript:alert(1)".into());
        assert_eq!(
            validate_server_input(&input),
            Err(ValidationError("discord_link must start with http:// or https://".into()))
        );
    }

    #[test]
    fn rejects_javascript_icon_path() {
        let mut input = valid_input();
        input.icon_path = Some("javascript:alert(1)".into());
        assert_eq!(
            validate_server_input(&input),
            Err(ValidationError("icon_path must start with /, http://, or https://".into()))
        );
    }

    #[test]
    fn accepts_local_icon_path() {
        let mut input = valid_input();
        input.icon_path = Some("/icons/server.png".into());
        assert!(validate_server_input(&input).is_ok());
    }
}
