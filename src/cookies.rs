use tldextract::{TldExtractor, TldOption};

use reqwest::header::{self, HeaderValue};

use rookie::{firefox, common::enums::CookieToString, common::enums::Cookie};

#[derive(Default)]
pub struct CookieJarWrapper {
}

impl CookieJarWrapper {
    pub fn new() -> Self {
        Self{}
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
    let domain_offset = match url_domain.find(cookie_domain_noprefix) {
        Some(offset) => offset,
        None => 0
    };
    
    // If domain_offset is 0 (or less?), then no
    let last_char_before_cookie_domain_is_periodt = if domain_offset <= 0 {
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
        println!("Throwing away new cookie from {}", url.as_str())
    }
    fn cookies(&self, url: &url::Url) -> Option<HeaderValue> {
        let extractor: TldExtractor = TldOption::default().build();
        let tldinfo = extractor.extract(url.as_str()).unwrap();    
        let together = format!("{}.{}", tldinfo.domain.unwrap(), tldinfo.suffix.unwrap());

        let cookies = firefox(Some(vec![together.clone().into()])).unwrap();

        let s = cookies.into_iter().filter_map(
            |cookie|
            {
                if cookie_matches_url(&cookie, &url) {
                    Some(cookie)
                } else {
                    None
                }
            }
        ).collect::<Vec<_>>()
        .to_string();

        if s.is_empty() {
            return None;
        }

        let header = header::HeaderValue::from_str(&s).unwrap();
        Some(header)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;
    use rookie::common::enums::Cookie;

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
}
