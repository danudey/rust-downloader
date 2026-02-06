use std::{fs::File, process::exit};
use std::sync::Arc;
use std::io::copy;
use std::thread::{self, JoinHandle};

use clap::Parser;
use clap::crate_version;
use log::{debug, info, warn, error};

use reqwest::header::{self};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use url::Url;

use content_disposition::{parse_content_disposition, DispositionType};

mod browser;
mod cookies;

use browser::{BrowserType, BrowserError, CookieManager};

/// Validate and parse browser argument
fn validate_browser_argument(browser_arg: Option<String>) -> Result<Option<BrowserType>, BrowserError> {
    match browser_arg {
        Some(browser_str) => {
            match browser_str.parse::<BrowserType>() {
                Ok(browser_type) => Ok(Some(browser_type)),
                Err(e) => Err(e),
            }
        }
        None => Ok(None),
    }
}

#[derive(Parser, Debug)]
struct Cli {
    /// The URL to download from
    #[arg(required = true)]
    urls: Vec<String>,
    
    /// Browser to use for cookies (chrome, firefox, safari, edge)
    #[arg(long, short, value_name = "BROWSER")]
    browser: Option<String>,
}

fn download_file(urls: Vec<String>, browser_type: Option<BrowserType>) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Starting download_file with {} URLs and browser type: {:?}", urls.len(), browser_type);
    let mut failed_download = false;

    // Create CookieManager based on browser selection
    let _cookie_manager = match browser_type {
        Some(browser) => {
            info!("User specified browser: {}", browser);
            // User specified a browser, try to use it
            match CookieManager::new(browser.clone()) {
                Ok(manager) => {
                    info!("Successfully created CookieManager with {} browser", manager.browser_name());
                    debug!("Using {} browser for cookies", manager.browser_name());
                    Some(manager)
                }
                Err(e) => {
                    warn!("Failed to create CookieManager with {}: {}", browser, e.brief_message());
                    eprintln!("Warning: {}", e.user_friendly_message());
                    eprintln!("Falling back to auto-detection...");
                    match CookieManager::with_auto_detection() {
                        Ok(manager) => {
                            info!("Fallback auto-detection successful: {}", manager.browser_name());
                            debug!("Using {} browser for cookies", manager.browser_name());
                            Some(manager)
                        }
                        Err(fallback_err) => {
                            warn!("Fallback auto-detection failed: {}", fallback_err.brief_message());
                            eprintln!("Warning: {}", fallback_err.user_friendly_message());
                            None
                        }
                    }
                }
            }
        }
        None => {
            debug!("No browser specified, using fallback with Firefox preference");
            // No browser specified, use auto-detection for backward compatibility
            // Default to Firefox first for backward compatibility, then auto-detect
            match CookieManager::with_fallback(Some(BrowserType::Firefox)) {
                Ok(manager) => {
                    info!("Fallback CookieManager created with: {}", manager.browser_name());
                    debug!("Using {} browser for cookies", manager.browser_name());
                    Some(manager)
                }
                Err(e) => {
                    warn!("Fallback CookieManager creation failed: {}", e.brief_message());
                    None
                }
            }
        }
    };

    // Set our progress bar components
    let style = ProgressStyle::with_template("{prefix:.blue} {wide_bar:.blue/white} {percent}% • {bytes:.green}/{total_bytes:.green} • {binary_bytes_per_sec:>11.red} • eta {eta:>5.cyan}  ")
    .unwrap()
    .progress_chars("━╸━");

    let finish_style = ProgressStyle::with_template("{prefix:.blue} {wide_bar:.blue/white} {percent}% • {total_bytes:.green} • {binary_bytes_per_sec:>11.red} • elapsed {elapsed:>4.cyan}  ")
    .unwrap()
    .progress_chars("━╸━");


    let mut headers = header::HeaderMap::new();
    let user_agent = format!("rust-downloader/{} (https://github.com/danudey/rust-downloader)", crate_version!()).into_bytes();
    headers.insert(header::ACCEPT, header::HeaderValue::from_static("*/*"));
    headers.insert(header::USER_AGENT, header::HeaderValue::from_bytes(&user_agent).unwrap());
    
    let errstyle = ProgressStyle::with_template("{prefix:.red} [error] {msg:} ").unwrap();
    let multiprog = Arc::new(MultiProgress::new());
    let mut handles: Vec<JoinHandle<_>> = vec![];

    // Use the CookieManager that was created earlier in the function
    let cookie_store = match _cookie_manager {
        Some(cookie_manager) => {
            let cookiejar_wrapper = cookies::CookieJarWrapper::new(cookie_manager);
            Some(std::sync::Arc::new(cookiejar_wrapper))
        }
        None => {
            // No cookie manager available, continue without cookies
            None
        }
    };

    for url in urls {
        // Parse our URL out so we can get a destination filename
        let parsed_url  = Url::parse(&url)?;
        let mut path_segments = parsed_url
            .path_segments()
            .ok_or("URL does not contain path segments; cannot derive a base path")?;
        let url_filename = path_segments
            .next_back()
            .ok_or("URL path is empty; cannot determine filename from URL")?;

        let client = match &cookie_store {
            Some(store) => {
                reqwest::blocking::Client::builder()
                    .cookie_provider(std::sync::Arc::clone(store))
                    .connection_verbose(true)
                    .build()
                    .unwrap()
            }
            None => {
                reqwest::blocking::Client::builder()
                .connection_verbose(true)
                    .build()
                    .unwrap()
            }
        };

        let headers = headers.clone();

        // Make our HTTP request and get our response (headers)
        let request = client
            .get(url.clone())
            .headers(headers.clone())
            .build()
            .unwrap();
        let response = match client.execute(request) {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to query URL: {}", e.with_url(parsed_url));
                continue;
            },
        };

        // Instantiate our progress bar
        let pb: ProgressBar = multiprog.add(ProgressBar::new(0).with_style(style.clone()));

        // Bail out if some bad stuff happened

        if response.status().is_server_error() {
            let errstr = format!("{}: got server error from {}: {}", parsed_url.as_str(), response.status().as_str(), response.status().canonical_reason().unwrap());
            pb.set_style(errstyle.clone());
            pb.finish_with_message(errstr);
            failed_download = true;
            continue;
        } else if response.status().is_client_error() {
            let errstr = format!("{}: server reported client error for {}: {}", parsed_url.as_str(), response.status().as_str(), response.status().canonical_reason().unwrap());
            pb.set_style(errstyle.clone());
            pb.finish_with_message(errstr);
            failed_download = true;
            continue;
        }

        // Check the Content-Length header if we got one; otherwise, set it to zero
        let content_length: u64 = response.content_length().unwrap_or_default();

        pb.set_length(content_length );

        let disposition = match response.headers().get("Content-Disposition") {
            Some(value) => value.to_str().unwrap(),
            None => ""
        };

        let disparsed = parse_content_disposition(disposition);
        let output_filename = if disparsed.disposition == DispositionType::Attachment {
            disparsed.filename_full().unwrap_or(url_filename.to_string())
        } else {
            url_filename.to_string()
        };

        if output_filename.trim().is_empty() {
            let errstr = format!("{}: no filename could be detected from the URL or Content-Disposition headers", parsed_url.as_str());
            pb.set_style(errstyle.clone());
            pb.finish_with_message(errstr);
            failed_download = true;
            continue;
        }

        // Set the prefix to our filename so we can display it
        pb.set_prefix(String::from(url_filename));

        // Now we create our output file...
        let mut dest = File::create(url_filename).map_err(|e| format!("Failed to create file: {}", e))?;

        let finish = finish_style.clone();
        let handle = thread::spawn(move || {
            // ...and write the data to it as we get it
            let _ = copy(&mut pb.wrap_read(response), &mut dest).map_err(|e| format!("Failed to copy content: {}", e));
            pb.set_style(finish);
            pb.finish();
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }

    if failed_download {
        exit(1);
    }

    Ok(())
}

fn main() {
    // Initialize logging
    env_logger::init();
        
    let args = Cli::parse();
    debug!("Application started with args: {:?}", args);

    // Validate browser argument if provided
    let browser_type = match validate_browser_argument(args.browser.clone()) {
        Ok(browser) => {
            debug!("Browser argument validation successful: {:?}", browser);
            browser
        }
        Err(e) => {
            error!("{}", e.user_friendly_message());
            exit(1);
        }
    };

    debug!("Starting download process for {} URLs", args.urls.len());
    let result = download_file(args.urls, browser_type);
    match result {
        Ok(()) => {
            debug!("Download process completed successfully");
        }
        Err(e) => {
            error!("Download process failed: {}", e);
            println!("Application error: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parsing_no_browser() {
        let args = Cli::try_parse_from(&["download", "http://example.com"]).unwrap();
        assert_eq!(args.urls, vec!["http://example.com"]);
        assert_eq!(args.browser, None);
    }

    #[test]
    fn test_cli_parsing_with_browser_long() {
        let args = Cli::try_parse_from(&["download", "--browser", "chrome", "http://example.com"]).unwrap();
        assert_eq!(args.urls, vec!["http://example.com"]);
        assert_eq!(args.browser, Some("chrome".to_string()));
    }

    #[test]
    fn test_cli_parsing_with_browser_short() {
        let args = Cli::try_parse_from(&["download", "-b", "firefox", "http://example.com"]).unwrap();
        assert_eq!(args.urls, vec!["http://example.com"]);
        assert_eq!(args.browser, Some("firefox".to_string()));
    }

    #[test]
    fn test_cli_parsing_multiple_urls() {
        let args = Cli::try_parse_from(&[
            "download", 
            "--browser", "safari", 
            "http://example.com", 
            "http://test.com"
        ]).unwrap();
        assert_eq!(args.urls, vec!["http://example.com", "http://test.com"]);
        assert_eq!(args.browser, Some("safari".to_string()));
    }

    #[test]
    fn test_validate_browser_argument_valid() {
        let result = validate_browser_argument(Some("chrome".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(BrowserType::Chrome));

        let result = validate_browser_argument(Some("firefox".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(BrowserType::Firefox));

        let result = validate_browser_argument(Some("safari".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(BrowserType::Safari));

        let result = validate_browser_argument(Some("edge".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(BrowserType::Edge));
    }

    #[test]
    fn test_validate_browser_argument_case_insensitive() {
        let result = validate_browser_argument(Some("CHROME".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(BrowserType::Chrome));

        let result = validate_browser_argument(Some("Firefox".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(BrowserType::Firefox));
    }

    #[test]
    fn test_validate_browser_argument_none() {
        let result = validate_browser_argument(None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_validate_browser_argument_invalid() {
        let result = validate_browser_argument(Some("invalid".to_string()));
        assert!(result.is_err());
        match result.unwrap_err() {
            BrowserError::UnsupportedBrowser { browser } => {
                assert_eq!(browser, "invalid");
            }
            _ => panic!("Expected UnsupportedBrowser error"),
        }
    }

    #[test]
    fn test_validate_browser_argument_empty() {
        let result = validate_browser_argument(Some("".to_string()));
        assert!(result.is_err());
        match result.unwrap_err() {
            BrowserError::UnsupportedBrowser { browser } => {
                assert_eq!(browser, "");
            }
            _ => panic!("Expected UnsupportedBrowser error"),
        }
    }

    #[test]
    fn test_cli_help_contains_browser_options() {
        let help_output = Cli::try_parse_from(&["download", "--help"]);
        assert!(help_output.is_err());
        
        // The help should be in the error message
        let error = help_output.unwrap_err();
        let help_text = error.to_string();
        
        // Check that help text contains browser information
        assert!(help_text.contains("--browser") || help_text.contains("-b"));
        assert!(help_text.contains("chrome") || help_text.contains("firefox") || help_text.contains("safari") || help_text.contains("edge"));
    }

    #[test]
    fn test_cli_parsing_browser_with_equals() {
        let args = Cli::try_parse_from(&["download", "--browser=chrome", "http://example.com"]).unwrap();
        assert_eq!(args.urls, vec!["http://example.com"]);
        assert_eq!(args.browser, Some("chrome".to_string()));
    }

    // Integration tests for complete CLI-to-cookie-fetching flow
    #[test]
    fn test_integration_browser_selection_valid() {
        // Test that valid browser selection works end-to-end
        for browser_name in &["chrome", "firefox", "safari", "edge"] {
            let browser_arg = Some(browser_name.to_string());
            let browser_type = validate_browser_argument(browser_arg);
            
            assert!(browser_type.is_ok(), "Browser {} should be valid", browser_name);
            
            let browser_type = browser_type.unwrap();
            assert!(browser_type.is_some(), "Browser type should be Some for {}", browser_name);
            
            let browser_type = browser_type.unwrap();
            assert_eq!(browser_type.as_str(), *browser_name, "Browser type should match input");
        }
    }

    #[test]
    fn test_integration_browser_selection_invalid() {
        // Test that invalid browser selection fails appropriately
        let invalid_browsers = &["invalid", "ie", "opera", ""];
        
        for invalid_browser in invalid_browsers {
            let browser_arg = Some(invalid_browser.to_string());
            let result = validate_browser_argument(browser_arg);
            
            assert!(result.is_err(), "Invalid browser '{}' should fail validation", invalid_browser);
            
            match result.unwrap_err() {
                BrowserError::UnsupportedBrowser { browser } => {
                    assert_eq!(browser, *invalid_browser);
                }
                _ => panic!("Expected UnsupportedBrowser error for '{}'", invalid_browser),
            }
        }
    }

    #[test]
    fn test_integration_backward_compatibility() {
        // Test that no browser argument works (backward compatibility)
        let result = validate_browser_argument(None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_integration_cli_to_browser_type_flow() {
        // Test complete flow from CLI parsing to browser type validation
        let test_cases = vec![
            ("chrome", BrowserType::Chrome),
            ("firefox", BrowserType::Firefox),
            ("safari", BrowserType::Safari),
            ("edge", BrowserType::Edge),
        ];

        for (browser_str, expected_type) in test_cases {
            // Parse CLI arguments
            let args = Cli::try_parse_from(&[
                "download", 
                "--browser", browser_str, 
                "http://example.com"
            ]).unwrap();
            
            // Validate browser argument
            let browser_type = validate_browser_argument(args.browser).unwrap();
            
            // Verify the result
            assert_eq!(browser_type, Some(expected_type));
        }
    }

    #[test]
    fn test_integration_error_message_format() {
        // Test that error messages are user-friendly
        let result = validate_browser_argument(Some("invalid".to_string()));
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        let user_message = error.user_friendly_message();
        
        // Check that the error message contains helpful information
        assert!(user_message.contains("invalid"));
        assert!(user_message.contains("chrome") || user_message.contains("firefox"));
    }

    #[test]
    fn test_integration_case_insensitive_flow() {
        // Test that case-insensitive browser names work end-to-end
        let test_cases = vec![
            ("CHROME", BrowserType::Chrome),
            ("Firefox", BrowserType::Firefox),
            ("SAFARI", BrowserType::Safari),
            ("Edge", BrowserType::Edge),
        ];

        for (browser_str, expected_type) in test_cases {
            let args = Cli::try_parse_from(&[
                "download", 
                "--browser", browser_str, 
                "http://example.com"
            ]).unwrap();
            
            let browser_type = validate_browser_argument(args.browser).unwrap();
            assert_eq!(browser_type, Some(expected_type));
        }
    }

    // Test the main function error handling (without actually running download_file)
    #[test]
    fn test_main_function_browser_validation() {
        // This test verifies that the main function properly validates browser arguments
        // We can't easily test the full main function, but we can test the validation logic
        
        // Test valid browser
        let valid_result = validate_browser_argument(Some("chrome".to_string()));
        assert!(valid_result.is_ok());
        
        // Test invalid browser
        let invalid_result = validate_browser_argument(Some("invalid".to_string()));
        assert!(invalid_result.is_err());
        
        // Verify error message format
        let error = invalid_result.unwrap_err();
        let message = error.user_friendly_message();
        assert!(message.contains("chrome") || message.contains("firefox"));
    }

    // Integration tests for HTTP requests with cookies from different browsers
    #[test]
    fn test_integration_cookie_jar_wrapper_with_reqwest() {
        use crate::cookies::CookieJarWrapper;
        use reqwest::cookie::CookieStore;
        use url::Url;
        
        // Test that CookieJarWrapper can be used with reqwest
        // We'll use auto-detection to get any available browser
        if let Ok(cookie_manager) = CookieManager::with_auto_detection() {
            let jar = CookieJarWrapper::new(cookie_manager);
            let url = Url::parse("https://example.com").unwrap();
            
            // Test that the cookies method can be called without panicking
            let _result = jar.cookies(&url);
            // We can't assert specific values since it depends on actual browser state
            // But we can verify the method works without errors
        }
    }

    #[test]
    fn test_integration_client_creation_with_cookies() {
        // Test that we can create a reqwest client with cookie support
        if let Ok(cookie_manager) = CookieManager::with_auto_detection() {
            let cookiejar_wrapper = crate::cookies::CookieJarWrapper::new(cookie_manager);
            let cookie_store = std::sync::Arc::new(cookiejar_wrapper);
            
            // Test that we can create a client with the cookie store
            let client_result = reqwest::blocking::Client::builder()
                .cookie_provider(cookie_store)
                .build();
            
            assert!(client_result.is_ok(), "Should be able to create client with cookie store");
        }
    }

    #[test]
    fn test_integration_client_creation_without_cookies() {
        // Test that we can create a reqwest client without cookie support
        let client_result = reqwest::blocking::Client::builder()
            .build();
        
        assert!(client_result.is_ok(), "Should be able to create client without cookies");
    }

    #[test]
    fn test_integration_cookie_manager_error_handling() {
        // Test that cookie manager errors are handled gracefully
        use crate::cookies::CookieJarWrapper;
        use reqwest::cookie::CookieStore;
        use url::Url;
        
        // Create a mock strategy that always errors
        struct ErrorStrategy;
        impl crate::browser::BrowserStrategy for ErrorStrategy {
            fn fetch_cookies(&self, _domains: Vec<String>) -> Result<Vec<rookie::common::enums::Cookie>, crate::browser::BrowserError> {
                Err(crate::browser::BrowserError::cookie_fetch_error("test", "Mock error"))
            }
            fn is_available(&self) -> bool { true }
            fn browser_name(&self) -> &'static str { "test" }
        }
        
        let error_manager = CookieManager::with_strategy(Box::new(ErrorStrategy));
        let jar = CookieJarWrapper::new(error_manager);
        let url = Url::parse("https://example.com").unwrap();
        
        // Should return None when cookie fetching fails, not panic
        let result = jar.cookies(&url);
        assert!(result.is_none(), "Should return None when cookie fetching fails");
    }

    #[test]
    fn test_integration_cookie_filtering_with_different_browsers() {
        // Test that cookie filtering works consistently across different browser strategies
        use crate::cookies::CookieJarWrapper;
        use reqwest::cookie::CookieStore;
        use url::Url;
        use rookie::common::enums::Cookie;
        
        // Create a mock strategy that returns test cookies
        struct TestStrategy;
        impl crate::browser::BrowserStrategy for TestStrategy {
            fn fetch_cookies(&self, _domains: Vec<String>) -> Result<Vec<Cookie>, crate::browser::BrowserError> {
                Ok(vec![
                    Cookie {
                        domain: "example.com".to_string(),
                        path: "/".to_string(),
                        name: "test_cookie".to_string(),
                        value: "test_value".to_string(),
                        http_only: false,
                        secure: false,
                        same_site: 0,
                        expires: None,
                    }
                ])
            }
            fn is_available(&self) -> bool { true }
            fn browser_name(&self) -> &'static str { "test" }
        }
        
        let test_manager = CookieManager::with_strategy(Box::new(TestStrategy));
        let jar = CookieJarWrapper::new(test_manager);
        
        // Test matching URL
        let matching_url = Url::parse("https://example.com/page").unwrap();
        let matching_result = jar.cookies(&matching_url);
        assert!(matching_result.is_some(), "Should return cookies for matching domain");
        
        // Test non-matching URL
        let non_matching_url = Url::parse("https://other.com/page").unwrap();
        let non_matching_result = jar.cookies(&non_matching_url);
        assert!(non_matching_result.is_none(), "Should not return cookies for non-matching domain");
    }
}

