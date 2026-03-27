const MAX_USERNAME_LEN: usize = 64;

pub fn validate_username(username: &str) -> Result<(), String> {
    if username.trim().is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    if username.len() > MAX_USERNAME_LEN {
        return Err(format!(
            "Username '{}' is too long (max {} characters)",
            username, MAX_USERNAME_LEN
        ));
    }
    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-' || c == '?')
    {
        return Err(format!(
            "Invalid username '{}': only alphanumeric, dots, underscores, hyphens and ? are allowed",
            username
        ));
    }
    Ok(())
}

pub fn validate_proxy(proxy: &str) -> Result<(), String> {
    if proxy.is_empty() {
        return Ok(());
    }
    if !proxy
        .chars()
        .all(|c| c.is_alphanumeric() || ".:/-_@[]".contains(c))
    {
        return Err("Invalid proxy URL: contains forbidden characters".to_string());
    }
    if !(proxy.starts_with("http://")
        || proxy.starts_with("https://")
        || proxy.starts_with("socks4://")
        || proxy.starts_with("socks5://"))
    {
        return Err(
            "Invalid proxy URL: must start with http://, https://, socks4:// or socks5://"
                .to_string(),
        );
    }
    Ok(())
}

pub fn validate_site_name(site: &str) -> Result<(), String> {
    if site.is_empty() {
        return Err("Site name cannot be empty".to_string());
    }
    if !site
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ' ' || c == '_')
    {
        return Err(format!(
            "Invalid site name '{}': only alphanumeric, dots, hyphens, underscores and spaces are allowed",
            site
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_usernames() {
        assert!(validate_username("john_doe").is_ok());
        assert!(validate_username("user.name").is_ok());
        assert!(validate_username("user-name").is_ok());
        assert!(validate_username("user123").is_ok());
        assert!(validate_username("user?name").is_ok()); // sherlock wildcard char
    }

    #[test]
    fn invalid_usernames() {
        assert!(validate_username("").is_err());
        assert!(validate_username("   ").is_err());
        assert!(validate_username("user name").is_err()); // space
        assert!(validate_username("user;drop").is_err()); // semicolon
        assert!(validate_username("user&cmd").is_err()); // ampersand
        assert!(validate_username(&"a".repeat(65)).is_err()); // too long
    }

    #[test]
    fn valid_proxies() {
        assert!(validate_proxy("").is_ok()); // empty = no proxy
        assert!(validate_proxy("http://127.0.0.1:8080").is_ok());
        assert!(validate_proxy("https://proxy.example.com").is_ok());
        assert!(validate_proxy("socks5://127.0.0.1:9050").is_ok());
        assert!(validate_proxy("socks4://10.0.0.1:1080").is_ok());
    }

    #[test]
    fn invalid_proxies() {
        assert!(validate_proxy("ftp://server.com").is_err()); // bad scheme
        assert!(validate_proxy("not-a-url").is_err()); // no scheme
        assert!(validate_proxy("http://host;rm -rf").is_err()); // injection
    }

    #[test]
    fn valid_site_names() {
        assert!(validate_site_name("GitHub").is_ok());
        assert!(validate_site_name("Stack Overflow").is_ok());
        assert!(validate_site_name("dev.to").is_ok());
        assert!(validate_site_name("my-site_v2").is_ok());
    }

    #[test]
    fn invalid_site_names() {
        assert!(validate_site_name("").is_err());
        assert!(validate_site_name("site;drop").is_err());
        assert!(validate_site_name("site&cmd").is_err());
    }
}
