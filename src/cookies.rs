use tldextract::{TldExtractor, TldOption};

use reqwest::header::{self, HeaderValue};

use rookie::{common::enums::CookieToString, common::enums::Cookie};
use crate::browser::CookieManager;
use log::{debug, warn};

pub struct CookieJarWrapper {
    cookie_manager: CookieManager,
}

impl CookieJarWrapper {
    pub fn new(cookie_manager: CookieManager) -> Self {
        Self { cookie_manager }
    }
}

pub fn cookie_matches_url(cookie: &Cookie, url: &url::Url) -> bool {
    // Here's how we match cookies to URLs:
    // 1. The cookie should have a path, and the URL should start with that path
    // 2. The cookie should have a domain, and
    //    a. The cookie domain and URL domain should be identical; or
    //    b. The URL domain should end with the cookie domain and have a single dot '.' before it
    //
    // To clarify 2b:
    //
    // Cookie domain        URL domain          Result
    // -----------------------------------------------
    // here.foo.com         here.foo.com        OK (domains are identical)
    //
    //                            cookie domain
    //                            ┌──────────┐
    // here.foo.com         there.here.foo.com  OK (URL domain ends with cookie doman and there's a '.' before it)
    //                           └─ dot in front of cookie domain section, so we're ok
    //
    //                            cookie domain
    //                            ┌──────────┐
    // here.foo.com              where.foo.com       NO (URL domain ends with cookie domain but there's not a '.' before it)
    //                           └─ no dot in front of cookie domain section, so we're not ok
    let cookie_domain_noprefix = match cookie.domain.strip_prefix(".") {
        Some(cookie_domain) => cookie_domain,
        None => cookie.domain.as_str()
    };

    let url_domain = url.domain().unwrap();
    let domain_offset: usize = url_domain.find(cookie_domain_noprefix).unwrap_or_default();
    
    // If domain_offset is 0 (or less?), then no
    let last_char_before_cookie_domain_is_periodt = if domain_offset == 0 {
        false
    } else {
        // If domain_offset > 0, then
        match url_domain.chars().nth(domain_offset-1) {
            // If the character before domain_offset is a '.', then yes
            Some(char) => char == '.',
            // Otherwise, no
            None => false
        }
    };

    let url_path_matches = url.path().starts_with(cookie.path.as_str());
    let cookie_domain_is_url_domain = cookie.domain == url_domain;
    let url_domain_ends_with_cookie_domain = url_domain.ends_with(cookie_domain_noprefix);
    // We need to make sure the URL path starts with the cookie path
    if url_path_matches &&
        // If the cookie domain and the URL domain are identical, we pass
        (cookie_domain_is_url_domain ||
            // If the URL domain ends with the cookie domain AND the last character before the
            // cookie domain appears in the URL domain is a dot, we pass
            (url_domain_ends_with_cookie_domain && last_char_before_cookie_domain_is_periodt)
        ) {
        true
    } else {
        false
    }
}

impl reqwest::cookie::CookieStore for CookieJarWrapper {
    fn set_cookies(&self, _cookie_headers: &mut dyn Iterator<Item = &reqwest::header::HeaderValue>, url: &url::Url) {
        debug!("Discarding incoming cookie for URL: {}", url.as_str());
        // Note: We don't store incoming cookies, only read existing browser cookies
    }
    
    fn cookies(&self, url: &url::Url) -> Option<HeaderValue> {
        debug!("Fetching cookies for URL: {}", url.as_str());
        
        let extractor: TldExtractor = TldOption::default().build();
        let tldinfo = match extractor.extract(url.as_str()) {
            Ok(info) => info,
            Err(_) => {
                        warn!("Failed to extract TLD information from URL: {}", url.as_str());
                        return None;
                    }
        };
        
        let domain = match tldinfo.domain {
            Some(domain) => domain,
            None => {
                warn!("Failed to extract domain from URL: {}", url.as_str());
                return None;
            }
        };
        
        let suffix = match tldinfo.suffix {
            Some(suffix) => suffix,
            None => {
                warn!("Failed to extract suffix from URL: {}", url.as_str());
                return None;
            }
        };
        
        let together = format!("{}.{}", domain, suffix);
        debug!("Extracted domain for cookie lookup: {}", together);

        // Use the injected CookieManager instead of hardcoded Firefox
        let cookies = match self.cookie_manager.fetch_cookies_for_domain(together.clone()) {
            Ok(cookies) => {
                debug!("Retrieved {} cookies from browser for domain: {}", cookies.len(), together);
                cookies
            }
            Err(e) => {
                warn!("Failed to fetch cookies for domain {}: {}", together, e.brief_message());
                return None;
            }
        };

        let matching_cookies: Vec<_> = cookies.into_iter().filter_map(
            |cookie|
            {
                if cookie_matches_url(&cookie, url) {
                    debug!("Cookie {} matches URL {}", cookie.name, url.as_str());
                    Some(cookie)
                } else {
                    debug!("Cookie {} does not match URL {} (domain: {}, path: {})", 
                           cookie.name, url.as_str(), cookie.domain, cookie.path);
                    None
                }
            }
        ).collect();

        if matching_cookies.is_empty() {
            debug!("No matching cookies found for URL: {}", url.as_str());
            return None;
        }

        let cookie_header = matching_cookies.to_string();
        debug!("Sending {} matching cookies for URL: {} (cookie names: {:?})", 
               matching_cookies.len(), 
               url.as_str(),
               matching_cookies.iter().map(|c| &c.name).collect::<Vec<_>>());

        let header = header::HeaderValue::from_str(&cookie_header).unwrap();
        Some(header)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;
    use rookie::common::enums::Cookie;
    use crate::browser::{BrowserStrategy, BrowserError, CookieManager};
    use reqwest::cookie::CookieStore;

    fn make_cookie(domain: &str, path: &str) -> Cookie {
        Cookie {
            domain: domain.to_string(),
            path: path.to_string(),
            name: "test".to_string(),
            value: "dummy".to_string(),
            http_only: false,
            secure: false,
            same_site: 0,
            expires: None,
        }
    }

    // Mock browser strategy for testing CookieJarWrapper
    struct MockBrowserStrategy {
        cookie_templates: Vec<(String, String)>, // (domain, path) pairs
        should_error: bool,
        error_message: String,
    }

    impl MockBrowserStrategy {
        fn new(cookie_templates: Vec<(String, String)>) -> Self {
            Self {
                cookie_templates,
                should_error: false,
                error_message: String::new(),
            }
        }

        fn with_error(error_message: &str) -> Self {
            Self {
                cookie_templates: Vec::new(),
                should_error: true,
                error_message: error_message.to_string(),
            }
        }

        fn create_cookies(&self) -> Vec<Cookie> {
            self.cookie_templates
                .iter()
                .map(|(domain, path)| make_cookie(domain, path))
                .collect()
        }
    }

    impl BrowserStrategy for MockBrowserStrategy {
        fn fetch_cookies(&self, _domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
            if self.should_error {
                Err(BrowserError::cookie_fetch_error("mock", &self.error_message))
            } else {
                Ok(self.create_cookies())
            }
        }

        fn is_available(&self) -> bool {
            true
        }

        fn browser_name(&self) -> &'static str {
            "mock"
        }
    }

    fn create_mock_cookie_manager(cookie_templates: Vec<(String, String)>) -> CookieManager {
        CookieManager::with_strategy(Box::new(MockBrowserStrategy::new(cookie_templates)))
    }

    fn create_error_cookie_manager(error_message: &str) -> CookieManager {
        CookieManager::with_strategy(Box::new(MockBrowserStrategy::with_error(error_message)))
    }

    #[test]
    fn cookie_matches_url_exact_domain_and_path() {
        let cookie = make_cookie("example.com", "/foo");
        let url = Url::parse("https://example.com/foo/bar").unwrap();
        assert!(cookie_matches_url(&cookie, &url));
    }

    #[test]
    fn cookie_matches_url_subdomain_with_dot() {
        let cookie = make_cookie(".example.com", "/");
        let url = Url::parse("https://sub.example.com/").unwrap();
        assert!(cookie_matches_url(&cookie, &url));
    }

    #[test]
    fn test_cookie_does_not_match_wrong_path() {
        let cookie = make_cookie("example.com", "/foo");
        let url = Url::parse("https://example.com/bar").unwrap();
        assert!(!cookie_matches_url(&cookie, &url));
    }

    #[test]
    fn test_cookie_does_not_match_wrong_domain() {
        let cookie = make_cookie("example.com", "/");
        let url = Url::parse("https://other.com/").unwrap();
        assert!(!cookie_matches_url(&cookie, &url));
    }

    #[test]
    fn cookie_matches_url_subdomain_with_dot_and_path() {
        let cookie = make_cookie(".example.com", "/foo");
        let url = Url::parse("https://sub.example.com/foo/bar").unwrap();
        assert!(cookie_matches_url(&cookie, &url));
    }

    #[test]
    fn test_cookie_does_not_match_subdomain_without_dot() {
        let cookie = make_cookie("example.com", "/");
        let url = Url::parse("https://sub.fexample.com/").unwrap();
        assert!(!cookie_matches_url(&cookie, &url));
    }

    // CookieJarWrapper tests with different browser strategies
    #[test]
    fn test_cookie_jar_wrapper_with_matching_cookies() {
        let cookie_templates = vec![
            ("example.com".to_string(), "/".to_string()),
            ("test.com".to_string(), "/api".to_string()),
        ];
        let cookie_manager = create_mock_cookie_manager(cookie_templates);
        let jar = CookieJarWrapper::new(cookie_manager);

        let url = Url::parse("https://example.com/page").unwrap();
        let result = jar.cookies(&url);

        assert!(result.is_some());
        let header_value = result.unwrap();
        let header_str = header_value.to_str().unwrap();
        assert!(header_str.contains("test=dummy"));
    }

    #[test]
    fn test_cookie_jar_wrapper_with_no_matching_cookies() {
        let cookie_templates = vec![
            ("other.com".to_string(), "/".to_string()),
            ("different.com".to_string(), "/api".to_string()),
        ];
        let cookie_manager = create_mock_cookie_manager(cookie_templates);
        let jar = CookieJarWrapper::new(cookie_manager);

        let url = Url::parse("https://example.com/page").unwrap();
        let result = jar.cookies(&url);

        assert!(result.is_none());
    }

    #[test]
    fn test_cookie_jar_wrapper_with_path_filtering() {
        let cookie_templates = vec![
            ("example.com".to_string(), "/api".to_string()),
            ("example.com".to_string(), "/admin".to_string()),
        ];
        let cookie_manager = create_mock_cookie_manager(cookie_templates);
        let jar = CookieJarWrapper::new(cookie_manager);

        // Should match /api path
        let api_url = Url::parse("https://example.com/api/users").unwrap();
        let api_result = jar.cookies(&api_url);
        assert!(api_result.is_some());

        // Should not match /admin path when requesting /public
        let public_url = Url::parse("https://example.com/public").unwrap();
        let public_result = jar.cookies(&public_url);
        assert!(public_result.is_none());
    }

    #[test]
    fn test_cookie_jar_wrapper_with_subdomain_cookies() {
        let cookie_templates = vec![
            (".example.com".to_string(), "/".to_string()),
            ("specific.example.com".to_string(), "/".to_string()),
        ];
        let cookie_manager = create_mock_cookie_manager(cookie_templates);
        let jar = CookieJarWrapper::new(cookie_manager);

        // Should match subdomain with dot prefix
        let subdomain_url = Url::parse("https://sub.example.com/page").unwrap();
        let result = jar.cookies(&subdomain_url);
        assert!(result.is_some());
    }

    #[test]
    fn test_cookie_jar_wrapper_with_cookie_manager_error() {
        let cookie_manager = create_error_cookie_manager("Database locked");
        let jar = CookieJarWrapper::new(cookie_manager);

        let url = Url::parse("https://example.com/page").unwrap();
        let result = jar.cookies(&url);

        // Should return None when cookie manager fails
        assert!(result.is_none());
    }

    #[test]
    fn test_cookie_jar_wrapper_with_empty_cookie_list() {
        let cookie_templates = vec![];
        let cookie_manager = create_mock_cookie_manager(cookie_templates);
        let jar = CookieJarWrapper::new(cookie_manager);

        let url = Url::parse("https://example.com/page").unwrap();
        let result = jar.cookies(&url);

        assert!(result.is_none());
    }

    #[test]
    fn test_cookie_jar_wrapper_preserves_cookie_matching_logic() {
        // Test that existing cookie matching logic works with all browser sources
        let cookie_templates = vec![
            ("example.com".to_string(), "/foo".to_string()),
            (".example.com".to_string(), "/bar".to_string()),
            ("other.com".to_string(), "/".to_string()),
        ];
        let cookie_manager = create_mock_cookie_manager(cookie_templates);
        let jar = CookieJarWrapper::new(cookie_manager);

        // Test exact domain match
        let exact_url = Url::parse("https://example.com/foo/test").unwrap();
        let exact_result = jar.cookies(&exact_url);
        assert!(exact_result.is_some());

        // Test subdomain match with dot prefix
        let subdomain_url = Url::parse("https://sub.example.com/bar/test").unwrap();
        let subdomain_result = jar.cookies(&subdomain_url);
        assert!(subdomain_result.is_some());

        // Test no match for different domain
        let different_url = Url::parse("https://unrelated.com/").unwrap();
        let different_result = jar.cookies(&different_url);
        assert!(different_result.is_none());
    }
}
