use rookie::common::enums::Cookie;
use rookie::{chrome, edge, firefox};
use std::fmt;
use std::str::FromStr;
use log::{debug, info, warn, error};

#[cfg(target_os = "macos")]
use rookie::safari;

/// Trait defining the interface for browser-specific cookie fetching
pub trait BrowserStrategy: Send + Sync {
    /// Fetch cookies for the specified domains
    fn fetch_cookies(&self, domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError>;

    /// Check if this browser is available on the system
    fn is_available(&self) -> bool;

    /// Get the name of this browser
    fn browser_name(&self) -> &'static str;
}

/// Enum representing supported browser types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserType {
    Chrome,
    Firefox,
    Safari,
    Edge,
}

impl BrowserType {
    /// Get all supported browser types
    pub fn all() -> Vec<BrowserType> {
        vec![
            BrowserType::Chrome,
            BrowserType::Firefox,
            BrowserType::Safari,
            BrowserType::Edge,
        ]
    }

    /// Get the string representation of the browser type
    pub fn as_str(&self) -> &'static str {
        match self {
            BrowserType::Chrome => "chrome",
            BrowserType::Firefox => "firefox",
            BrowserType::Safari => "safari",
            BrowserType::Edge => "edge",
        }
    }
}

impl fmt::Display for BrowserType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for BrowserType {
    type Err = BrowserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "chrome" => Ok(BrowserType::Chrome),
            "firefox" => Ok(BrowserType::Firefox),
            "safari" => Ok(BrowserType::Safari),
            "edge" => Ok(BrowserType::Edge),
            _ => Err(BrowserError::UnsupportedBrowser(s.to_string())),
        }
    }
}

/// Comprehensive error types for browser operations
#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("Browser '{0}' is not supported. Available browsers: {}", 
            BrowserType::all().iter().map(|b| b.as_str()).collect::<Vec<_>>().join(", "))]
    UnsupportedBrowser(String),

    #[error("Browser '{0}' is not available or installed")]
    BrowserNotAvailable(String),

    #[error("No supported browsers found. Please install one of: {}", 
            BrowserType::all().iter().map(|b| b.as_str()).collect::<Vec<_>>().join(", "))]
    NoBrowsersAvailable,

    #[error("Failed to fetch cookies from {browser}: {message}")]
    CookieFetchError { browser: String, message: String },

    #[error("Invalid browser configuration: {0}")]
    InvalidConfiguration(String),
}

impl BrowserError {
    /// Create a cookie fetch error with browser context
    pub fn cookie_fetch_error(browser: &str, message: impl fmt::Display) -> Self {
        BrowserError::CookieFetchError {
            browser: browser.to_string(),
            message: message.to_string(),
        }
    }

    /// Get user-friendly error message with suggestions
    pub fn user_friendly_message(&self) -> String {
        match self {
            BrowserError::UnsupportedBrowser(browser) => {
                Self::format_unsupported_browser_message(browser)
            }
            BrowserError::BrowserNotAvailable(browser) => {
                Self::format_browser_not_available_message(browser)
            }
            BrowserError::NoBrowsersAvailable => {
                Self::format_no_browsers_available_message()
            }
            BrowserError::CookieFetchError { browser, message } => {
                Self::format_cookie_fetch_error_message(browser, message)
            }
            BrowserError::InvalidConfiguration(config) => {
                Self::format_invalid_configuration_message(config)
            }
        }
    }
    /// Format user-friendly message for unsupported browser errors
    fn format_unsupported_browser_message(browser: &str) -> String {
        let available_browsers = BrowserType::all()
            .iter()
            .map(|b| b.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "⛔ Browser '{}' is not supported. \
            Available browsers: {}",
            // 💡 Tip: Use --browser <name> to specify a supported browser.\n\
            // 📖 Example: --browser chrome",
            browser, available_browsers
        )
    }

    /// Format user-friendly message for browser not available errors
    fn format_browser_not_available_message(browser: &str) -> String {
        let installation_tips = match browser {
            "chrome" => "• Download from https://www.google.com/chrome/\n   • Make sure to run Chrome at least once after installation",
            "firefox" => "• Download from https://www.mozilla.org/firefox/\n   • Make sure to run Firefox at least once after installation",
            "safari" => "• Safari is pre-installed on macOS\n   • Make sure to run Safari at least once\n   • Note: Safari is only available on macOS",
            "edge" => "• Download from https://www.microsoft.com/edge/\n   • Make sure to run Edge at least once after installation",
            _ => "• Make sure the browser is installed and has been run at least once",
        };

        let available_browsers = CookieManager::detect_available_browsers()
            .iter()
            .map(|b| b.as_str())
            .collect::<Vec<_>>();

        let fallback_suggestion = if !available_browsers.is_empty() {
            format!(
                "\n🔄 Available alternatives: {}\n\
                💡 Tip: Try --browser {} instead",
                available_browsers.join(", "),
                available_browsers[0]
            )
        } else {
            String::new()
        };

        format!(
            "⛔ Browser '{}' is not available or installed.\n\n\
            🔧 Installation help:\n   {}\n{}",
            browser, installation_tips, fallback_suggestion
        )
    }

    /// Format user-friendly message for no browsers available errors
    fn format_no_browsers_available_message() -> String {
        let all_browsers = BrowserType::all()
            .iter()
            .map(|b| b.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "⛔ No supported browsers found on your system.\n\n\
            📋 Supported browsers: {}\n\n\
            🔧 Installation help:\n\
            • Chrome: https://www.google.com/chrome/\n\
            • Firefox: https://www.mozilla.org/firefox/\n\
            • Safari: Pre-installed on macOS\n\
            • Edge: https://www.microsoft.com/edge/\n\n\
            💡 Tip: After installing a browser, run it at least once to create cookie storage.",
            all_browsers
        )
    }

    /// Format user-friendly message for cookie fetch errors
    fn format_cookie_fetch_error_message(browser: &str, message: &str) -> String {
        let common_solutions = match message.to_lowercase() {
            msg if msg.contains("database") && msg.contains("lock") => {
                "• Close all browser windows and try again\n   • The browser's cookie database might be locked"
            }
            msg if msg.contains("permission") || msg.contains("access") => {
                "• Check file permissions for browser data directory\n   • Try running with appropriate permissions"
            }
            msg if msg.contains("not found") || msg.contains("no such file") => {
                "• Make sure the browser has been run at least once\n   • Browser profile might not exist yet"
            }
            _ => "• Try closing the browser and running the command again\n   • Check if the browser profile exists"
        };

        let available_browsers = CookieManager::detect_available_browsers()
            .iter()
            .filter(|b| b.as_str() != browser)
            .map(|b| b.as_str())
            .collect::<Vec<_>>();

        let alternative_suggestion = if !available_browsers.is_empty() {
            format!(
                "\n🔄 Try a different browser:\n   • Available: {}\n   • Example: --browser {}",
                available_browsers.join(", "),
                available_browsers[0]
            )
        } else {
            String::new()
        };

        format!(
            "⛔ Failed to fetch cookies from {}.\n\n\
            🔍 Error details: {}\n\n\
            🔧 Common solutions:\n   {}\n{}",
            browser, message, common_solutions, alternative_suggestion
        )
    }

    /// Format user-friendly message for invalid configuration errors
    fn format_invalid_configuration_message(config: &str) -> String {
        let available_browsers = BrowserType::all()
            .iter()
            .map(|b| b.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "⛔ Invalid browser configuration: {}\n\n\
            📋 Valid options: {}\n\n\
            💡 Tips:\n\
            • Use --browser <name> to specify a browser\n\
            • Browser names are case-insensitive\n\
            • Example: --browser chrome",
            config, available_browsers
        )
    }

    /// Get a brief error message without formatting (for logging)
    pub fn brief_message(&self) -> String {
        match self {
            BrowserError::UnsupportedBrowser(browser) => {
                format!("Unsupported browser: {}", browser)
            }
            BrowserError::BrowserNotAvailable(browser) => {
                format!("Browser not available: {}", browser)
            }
            BrowserError::NoBrowsersAvailable => {
                "No browsers available".to_string()
            }
            BrowserError::CookieFetchError { browser, message } => {
                format!("Cookie fetch failed for {}: {}", browser, message)
            }
            BrowserError::InvalidConfiguration(config) => {
                format!("Invalid configuration: {}", config)
            }
        }
    }

    /// Get suggestions for resolving the error
    pub fn get_suggestions(&self) -> Vec<String> {
        match self {
            BrowserError::UnsupportedBrowser(_) => {
                let available = BrowserType::all()
                    .iter()
                    .map(|b| format!("--browser {}", b.as_str()))
                    .collect();
                available
            }
            BrowserError::BrowserNotAvailable(browser) => {
                let mut suggestions = vec![format!("Install {} browser", browser)];
                let available = CookieManager::detect_available_browsers();
                if !available.is_empty() {
                    suggestions.push(format!("Use --browser {}", available[0].as_str()));
                }
                suggestions
            }
            BrowserError::NoBrowsersAvailable => {
                vec![
                    "Install Chrome, Firefox, Safari, or Edge".to_string(),
                    "Run the browser at least once after installation".to_string(),
                ]
            }
            BrowserError::CookieFetchError { browser, .. } => {
                let mut suggestions = vec![format!("Close {} and try again", browser)];
                let available = CookieManager::detect_available_browsers()
                    .iter()
                    .filter(|b| b.as_str() != browser)
                    .map(|b| format!("Try --browser {}", b.as_str()))
                    .collect::<Vec<_>>();
                suggestions.extend(available);
                suggestions
            }
            BrowserError::InvalidConfiguration(_) => {
                BrowserType::all()
                    .iter()
                    .map(|b| format!("Use --browser {}", b.as_str()))
                    .collect()
            }
        }
    }
}

/// Firefox browser strategy implementation
pub struct FirefoxStrategy;

impl FirefoxStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Check if Firefox profile directory exists
    fn firefox_profile_exists() -> bool {
        // Firefox profiles are typically stored in:
        // Linux: ~/.mozilla/firefox/
        // macOS: ~/Library/Application Support/Firefox/Profiles/
        // Windows: %APPDATA%\Mozilla\Firefox\Profiles\

        if let Some(home_dir) = dirs::home_dir() {
            let firefox_paths = [
                home_dir.join(".mozilla").join("firefox"),
                home_dir
                    .join("Library")
                    .join("Application Support")
                    .join("Firefox")
                    .join("Profiles"),
                home_dir
                    .join("AppData")
                    .join("Roaming")
                    .join("Mozilla")
                    .join("Firefox")
                    .join("Profiles"),
            ];

            firefox_paths
                .iter()
                .any(|path| path.exists() && path.is_dir())
        } else {
            false
        }
    }
}

impl BrowserStrategy for FirefoxStrategy {
    fn fetch_cookies(&self, domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
        debug!("Attempting to fetch cookies from Firefox for domains: {:?}", domains);
        match firefox(Some(domains.clone())) {
            Ok(cookies) => {
                info!("Successfully fetched {} cookies from Firefox for domains: {:?}", 
                      cookies.len(), domains);
                debug!("Firefox cookies: {:?}", cookies.iter().map(|c| format!("{}={}", c.name, "[REDACTED]")).collect::<Vec<_>>());
                Ok(cookies)
            }
            Err(e) => {
                error!("Failed to fetch cookies from Firefox for domains {:?}: {}", domains, e);
                Err(BrowserError::cookie_fetch_error("firefox", e))
            }
        }
    }

    fn is_available(&self) -> bool {
        let available = Self::firefox_profile_exists();
        debug!("Firefox availability check: {}", available);
        available
    }

    fn browser_name(&self) -> &'static str {
        "firefox"
    }
}

/// Chrome browser strategy implementation
pub struct ChromeStrategy;

impl ChromeStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Check if Chrome cookie database exists
    fn chrome_cookies_exist() -> bool {
        // Chrome cookies are typically stored in:
        // Linux: ~/.config/google-chrome/Default/Cookies
        // macOS: ~/Library/Application Support/Google/Chrome/Default/Cookies
        // Windows: %LOCALAPPDATA%\Google\Chrome\User Data\Default\Cookies

        if let Some(home_dir) = dirs::home_dir() {
            let chrome_paths = [
                home_dir
                    .join(".config")
                    .join("google-chrome")
                    .join("Default")
                    .join("Cookies"),
                home_dir
                    .join("Library")
                    .join("Application Support")
                    .join("Google")
                    .join("Chrome")
                    .join("Default")
                    .join("Cookies"),
                home_dir
                    .join("AppData")
                    .join("Local")
                    .join("Google")
                    .join("Chrome")
                    .join("User Data")
                    .join("Default")
                    .join("Cookies"),
            ];

            chrome_paths
                .iter()
                .any(|path| path.exists() && path.is_file())
        } else {
            false
        }
    }
}

impl BrowserStrategy for ChromeStrategy {
    fn fetch_cookies(&self, domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
        debug!("Attempting to fetch cookies from Chrome for domains: {:?}", domains);
        match chrome(Some(domains.clone())) {
            Ok(cookies) => {
                info!("Successfully fetched {} cookies from Chrome for domains: {:?}", 
                      cookies.len(), domains);
                debug!("Chrome cookies: {:?}", cookies.iter().map(|c| format!("{}={}", c.name, "[REDACTED]")).collect::<Vec<_>>());
                Ok(cookies)
            }
            Err(e) => {
                error!("Failed to fetch cookies from Chrome for domains {:?}: {}", domains, e);
                Err(BrowserError::cookie_fetch_error("chrome", e))
            }
        }
    }

    fn is_available(&self) -> bool {
        let available = Self::chrome_cookies_exist();
        debug!("Chrome availability check: {}", available);
        available
    }

    fn browser_name(&self) -> &'static str {
        "chrome"
    }
}

/// Safari browser strategy implementation
pub struct SafariStrategy;

impl SafariStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Check if Safari cookie store exists (macOS only)
    fn safari_cookies_exist() -> bool {
        // Safari cookies are stored in:
        // macOS: ~/Library/Cookies/Cookies.binarycookies

        if cfg!(target_os = "macos") {
            if let Some(home_dir) = dirs::home_dir() {
                let safari_cookies_path = home_dir
                    .join("Library")
                    .join("Cookies")
                    .join("Cookies.binarycookies");
                safari_cookies_path.exists() && safari_cookies_path.is_file()
            } else {
                false
            }
        } else {
            false // Safari is only available on macOS
        }
    }
}

impl BrowserStrategy for SafariStrategy {
    fn fetch_cookies(&self, domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
        #[cfg(target_os = "macos")]
        {
            debug!("Attempting to fetch cookies from Safari for domains: {:?}", domains);
            match safari(Some(domains.clone())) {
                Ok(cookies) => {
                    info!("Successfully fetched {} cookies from Safari for domains: {:?}", 
                          cookies.len(), domains);
                    debug!("Safari cookies: {:?}", cookies.iter().map(|c| format!("{}={}", c.name, "[REDACTED]")).collect::<Vec<_>>());
                    Ok(cookies)
                }
                Err(e) => {
                    error!("Failed to fetch cookies from Safari for domains {:?}: {}", domains, e);
                    Err(BrowserError::cookie_fetch_error("safari", e))
                }
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            warn!("Safari cookie fetch attempted on non-macOS platform for domains: {:?}", domains);
            Err(BrowserError::BrowserNotAvailable(
                "Safari is only available on macOS".to_string(),
            ))
        }
    }

    fn is_available(&self) -> bool {
        let available = Self::safari_cookies_exist();
        debug!("Safari availability check: {}", available);
        available
    }

    fn browser_name(&self) -> &'static str {
        "safari"
    }
}

/// Edge browser strategy implementation
pub struct EdgeStrategy;

impl EdgeStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Check if Edge cookie database exists
    fn edge_cookies_exist() -> bool {
        // Edge cookies are typically stored in:
        // Linux: ~/.config/microsoft-edge/Default/Cookies
        // macOS: ~/Library/Application Support/Microsoft Edge/Default/Cookies
        // Windows: %LOCALAPPDATA%\Microsoft\Edge\User Data\Default\Cookies

        if let Some(home_dir) = dirs::home_dir() {
            let edge_paths = [
                home_dir
                    .join(".config")
                    .join("microsoft-edge")
                    .join("Default")
                    .join("Cookies"),
                home_dir
                    .join("Library")
                    .join("Application Support")
                    .join("Microsoft Edge")
                    .join("Default")
                    .join("Cookies"),
                home_dir
                    .join("AppData")
                    .join("Local")
                    .join("Microsoft")
                    .join("Edge")
                    .join("User Data")
                    .join("Default")
                    .join("Cookies"),
            ];

            edge_paths
                .iter()
                .any(|path| path.exists() && path.is_file())
        } else {
            false
        }
    }
}

impl BrowserStrategy for EdgeStrategy {
    fn fetch_cookies(&self, domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
        debug!("Attempting to fetch cookies from Edge for domains: {:?}", domains);
        match edge(Some(domains.clone())) {
            Ok(cookies) => {
                info!("Successfully fetched {} cookies from Edge for domains: {:?}", 
                      cookies.len(), domains);
                debug!("Edge cookies: {:?}", cookies.iter().map(|c| format!("{}={}", c.name, "[REDACTED]")).collect::<Vec<_>>());
                Ok(cookies)
            }
            Err(e) => {
                error!("Failed to fetch cookies from Edge for domains {:?}: {}", domains, e);
                Err(BrowserError::cookie_fetch_error("edge", e))
            }
        }
    }

    fn is_available(&self) -> bool {
        let available = Self::edge_cookies_exist();
        debug!("Edge availability check: {}", available);
        available
    }

    fn browser_name(&self) -> &'static str {
        "edge"
    }
}

/// Cookie manager that uses the strategy pattern for browser selection
pub struct CookieManager {
    strategy: Box<dyn BrowserStrategy>,
}

impl CookieManager {
    /// Create a new CookieManager with explicit browser selection
    pub fn new(browser_type: BrowserType) -> Result<Self, BrowserError> {
        debug!("Creating CookieManager with explicit browser selection: {}", browser_type);
        
        let strategy: Box<dyn BrowserStrategy> = match browser_type {
            BrowserType::Chrome => Box::new(ChromeStrategy::new()),
            BrowserType::Firefox => Box::new(FirefoxStrategy::new()),
            BrowserType::Safari => Box::new(SafariStrategy::new()),
            BrowserType::Edge => Box::new(EdgeStrategy::new()),
        };

        // Check if the selected browser is available
        if !strategy.is_available() {
            warn!("Selected browser {} is not available", browser_type);
            return Err(BrowserError::BrowserNotAvailable(
                browser_type.as_str().to_string(),
            ));
        }

        info!("Successfully created CookieManager with {} browser", browser_type);
        Ok(Self { strategy })
    }

    /// Create a new CookieManager with auto-detection
    pub fn with_auto_detection() -> Result<Self, BrowserError> {
        debug!("Starting browser auto-detection");
        let available_browsers = Self::detect_available_browsers();
        
        if available_browsers.is_empty() {
            warn!("No browsers available during auto-detection");
            return Err(BrowserError::NoBrowsersAvailable);
        }

        info!("Auto-detection found {} available browsers: {:?}", 
              available_browsers.len(), available_browsers);

        // Use the first available browser from the priority order
        let browser_type = available_browsers[0].clone();
        info!("Auto-detection selected: {}", browser_type);
        Self::new(browser_type)
    }

    /// Detect all available browsers in priority order (Chrome, Firefox, Safari, Edge)
    pub fn detect_available_browsers() -> Vec<BrowserType> {
        debug!("Starting browser detection process");
        let browser_priority = [
            BrowserType::Chrome,
            BrowserType::Firefox,
            BrowserType::Safari,
            BrowserType::Edge,
        ];

        let mut available_browsers = Vec::new();

        for browser_type in &browser_priority {
            debug!("Checking availability of {}", browser_type);
            let strategy: Box<dyn BrowserStrategy> = match browser_type {
                BrowserType::Chrome => Box::new(ChromeStrategy::new()),
                BrowserType::Firefox => Box::new(FirefoxStrategy::new()),
                BrowserType::Safari => Box::new(SafariStrategy::new()),
                BrowserType::Edge => Box::new(EdgeStrategy::new()),
            };

            if strategy.is_available() {
                debug!("Browser {} is available", browser_type);
                available_browsers.push(browser_type.clone());
            } else {
                debug!("Browser {} is not available", browser_type);
            }
        }

        info!("Browser detection completed. Available browsers: {:?}", available_browsers);
        available_browsers
    }

    /// Create a new CookieManager with fallback logic
    /// Tries the preferred browser first, then falls back to auto-detection
    pub fn with_fallback(preferred_browser: Option<BrowserType>) -> Result<Self, BrowserError> {
        debug!("Creating CookieManager with fallback logic, preferred: {:?}", preferred_browser);
        
        // If a preferred browser is specified, try it first
        if let Some(browser_type) = preferred_browser {
            debug!("Trying preferred browser: {}", browser_type);
            match Self::new(browser_type.clone()) {
                Ok(manager) => {
                    info!("Successfully created CookieManager with preferred browser: {}", browser_type);
                    return Ok(manager);
                }
                Err(BrowserError::BrowserNotAvailable(_)) => {
                    warn!("Preferred browser {} not available, falling back to auto-detection", browser_type);
                    // Fall back to auto-detection if preferred browser is not available
                }
                Err(e) => {
                    error!("Error with preferred browser {}: {}", browser_type, e.brief_message());
                    return Err(e); // Return other errors immediately
                }
            }
        }

        // Fall back to auto-detection
        debug!("Falling back to auto-detection");
        Self::with_auto_detection()
    }

    /// Fetch cookies for the specified domain using the selected browser strategy
    pub fn fetch_cookies_for_domain(&self, domain: String) -> Result<Vec<Cookie>, BrowserError> {
        debug!("Fetching cookies for domain: {} using {}", domain, self.browser_name());
        let result = self.strategy.fetch_cookies(vec![domain.clone()]);
        match &result {
            Ok(cookies) => {
                info!("Successfully fetched {} cookies for domain: {}", cookies.len(), domain);
            }
            Err(e) => {
                warn!("Failed to fetch cookies for domain {}: {}", domain, e.brief_message());
            }
        }
        result
    }

    /// Get the name of the currently selected browser
    pub fn browser_name(&self) -> &str {
        self.strategy.browser_name()
    }

    /// Create a CookieManager with a custom strategy (for testing)
    #[cfg(test)]
    pub fn with_strategy(strategy: Box<dyn BrowserStrategy>) -> Self {
        Self { strategy }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_type_from_str_valid() {
        assert_eq!(
            "chrome".parse::<BrowserType>().unwrap(),
            BrowserType::Chrome
        );
        assert_eq!(
            "firefox".parse::<BrowserType>().unwrap(),
            BrowserType::Firefox
        );
        assert_eq!(
            "safari".parse::<BrowserType>().unwrap(),
            BrowserType::Safari
        );
        assert_eq!("edge".parse::<BrowserType>().unwrap(), BrowserType::Edge);
    }

    #[test]
    fn test_browser_type_from_str_case_insensitive() {
        assert_eq!(
            "CHROME".parse::<BrowserType>().unwrap(),
            BrowserType::Chrome
        );
        assert_eq!(
            "Firefox".parse::<BrowserType>().unwrap(),
            BrowserType::Firefox
        );
        assert_eq!(
            "SAFARI".parse::<BrowserType>().unwrap(),
            BrowserType::Safari
        );
        assert_eq!("Edge".parse::<BrowserType>().unwrap(), BrowserType::Edge);
    }

    #[test]
    fn test_browser_type_from_str_invalid() {
        let result = "invalid".parse::<BrowserType>();
        assert!(result.is_err());
        match result.unwrap_err() {
            BrowserError::UnsupportedBrowser(browser) => {
                assert_eq!(browser, "invalid");
            }
            _ => panic!("Expected UnsupportedBrowser error"),
        }
    }

    #[test]
    fn test_browser_type_display() {
        assert_eq!(BrowserType::Chrome.to_string(), "chrome");
        assert_eq!(BrowserType::Firefox.to_string(), "firefox");
        assert_eq!(BrowserType::Safari.to_string(), "safari");
        assert_eq!(BrowserType::Edge.to_string(), "edge");
    }

    #[test]
    fn test_browser_type_as_str() {
        assert_eq!(BrowserType::Chrome.as_str(), "chrome");
        assert_eq!(BrowserType::Firefox.as_str(), "firefox");
        assert_eq!(BrowserType::Safari.as_str(), "safari");
        assert_eq!(BrowserType::Edge.as_str(), "edge");
    }

    #[test]
    fn test_browser_type_all() {
        let all_browsers = BrowserType::all();
        assert_eq!(all_browsers.len(), 4);
        assert!(all_browsers.contains(&BrowserType::Chrome));
        assert!(all_browsers.contains(&BrowserType::Firefox));
        assert!(all_browsers.contains(&BrowserType::Safari));
        assert!(all_browsers.contains(&BrowserType::Edge));
    }

    #[test]
    fn test_browser_error_unsupported_browser_message() {
        let error = BrowserError::UnsupportedBrowser("invalid".to_string());
        let message = error.to_string();
        assert!(message.contains("invalid"));
        assert!(message.contains("chrome"));
        assert!(message.contains("firefox"));
        assert!(message.contains("safari"));
        assert!(message.contains("edge"));
    }

    #[test]
    fn test_browser_error_no_browsers_available_message() {
        let error = BrowserError::NoBrowsersAvailable;
        let message = error.to_string();
        assert!(message.contains("No supported browsers found"));
        assert!(message.contains("chrome"));
        assert!(message.contains("firefox"));
        assert!(message.contains("safari"));
        assert!(message.contains("edge"));
    }

    #[test]
    fn test_browser_error_cookie_fetch_error() {
        let error = BrowserError::cookie_fetch_error("chrome", "Database locked");
        match error {
            BrowserError::CookieFetchError { browser, message } => {
                assert_eq!(browser, "chrome");
                assert_eq!(message, "Database locked");
            }
            _ => panic!("Expected CookieFetchError"),
        }
    }

    #[test]
    fn test_format_unsupported_browser_message() {
        let message = BrowserError::format_unsupported_browser_message("invalid");
        assert!(message.contains("Available browsers: chrome, firefox, safari, edge"));
    }

    #[test]
    fn test_format_browser_not_available_message_chrome() {
        let message = BrowserError::format_browser_not_available_message("chrome");
        assert!(message.contains("⛔ Browser 'chrome' is not available"));
        assert!(message.contains("🔧 Installation help:"));
        assert!(message.contains("https://www.google.com/chrome/"));
        assert!(message.contains("Make sure to run Chrome at least once"));
    }

    #[test]
    fn test_format_browser_not_available_message_firefox() {
        let message = BrowserError::format_browser_not_available_message("firefox");
        assert!(message.contains("⛔ Browser 'firefox' is not available"));
        assert!(message.contains("https://www.mozilla.org/firefox/"));
        assert!(message.contains("Make sure to run Firefox at least once"));
    }

    #[test]
    fn test_format_browser_not_available_message_safari() {
        let message = BrowserError::format_browser_not_available_message("safari");
        assert!(message.contains("⛔ Browser 'safari' is not available"));
        assert!(message.contains("Safari is pre-installed on macOS"));
        assert!(message.contains("Note: Safari is only available on macOS"));
    }

    #[test]
    fn test_format_browser_not_available_message_edge() {
        let message = BrowserError::format_browser_not_available_message("edge");
        assert!(message.contains("⛔ Browser 'edge' is not available"));
        assert!(message.contains("https://www.microsoft.com/edge/"));
        assert!(message.contains("Make sure to run Edge at least once"));
    }

    #[test]
    fn test_format_no_browsers_available_message() {
        let message = BrowserError::format_no_browsers_available_message();
        assert!(message.contains("⛔ No supported browsers found"));
        assert!(message.contains("📋 Supported browsers: chrome, firefox, safari, edge"));
        assert!(message.contains("🔧 Installation help:"));
        assert!(message.contains("Chrome: https://www.google.com/chrome/"));
        assert!(message.contains("Firefox: https://www.mozilla.org/firefox/"));
        assert!(message.contains("Safari: Pre-installed on macOS"));
        assert!(message.contains("Edge: https://www.microsoft.com/edge/"));
        assert!(message.contains("💡 Tip: After installing a browser"));
    }

    #[test]
    fn test_format_cookie_fetch_error_message_database_lock() {
        let message = BrowserError::format_cookie_fetch_error_message("chrome", "Database is locked");
        assert!(message.contains("⛔ Failed to fetch cookies from chrome"));
        assert!(message.contains("🔍 Error details: Database is locked"));
        assert!(message.contains("🔧 Common solutions:"));
        assert!(message.contains("Close all browser windows"));
        assert!(message.contains("database might be locked"));
    }

    #[test]
    fn test_format_cookie_fetch_error_message_permission() {
        let message = BrowserError::format_cookie_fetch_error_message("firefox", "Permission denied");
        assert!(message.contains("⛔ Failed to fetch cookies from firefox"));
        assert!(message.contains("Permission denied"));
        assert!(message.contains("Check file permissions"));
        assert!(message.contains("Try running with appropriate permissions"));
    }

    #[test]
    fn test_format_cookie_fetch_error_message_not_found() {
        let message = BrowserError::format_cookie_fetch_error_message("safari", "File not found");
        assert!(message.contains("⛔ Failed to fetch cookies from safari"));
        assert!(message.contains("File not found"));
        assert!(message.contains("Make sure the browser has been run at least once"));
        assert!(message.contains("Browser profile might not exist"));
    }

    #[test]
    fn test_format_cookie_fetch_error_message_generic() {
        let message = BrowserError::format_cookie_fetch_error_message("edge", "Unknown error");
        assert!(message.contains("⛔ Failed to fetch cookies from edge"));
        assert!(message.contains("Unknown error"));
        assert!(message.contains("Try closing the browser"));
        assert!(message.contains("Check if the browser profile exists"));
    }

    #[test]
    fn test_format_invalid_configuration_message() {
        let message = BrowserError::format_invalid_configuration_message("bad config");
        assert!(message.contains("⛔ Invalid browser configuration: bad config"));
        assert!(message.contains("📋 Valid options: chrome, firefox, safari, edge"));
        assert!(message.contains("💡 Tips:"));
        assert!(message.contains("Use --browser <name>"));
        assert!(message.contains("Browser names are case-insensitive"));
        assert!(message.contains("Example: --browser chrome"));
    }

    #[test]
    fn test_brief_message() {
        let unsupported = BrowserError::UnsupportedBrowser("invalid".to_string());
        assert_eq!(unsupported.brief_message(), "Unsupported browser: invalid");

        let not_available = BrowserError::BrowserNotAvailable("chrome".to_string());
        assert_eq!(not_available.brief_message(), "Browser not available: chrome");

        let no_browsers = BrowserError::NoBrowsersAvailable;
        assert_eq!(no_browsers.brief_message(), "No browsers available");

        let fetch_error = BrowserError::cookie_fetch_error("firefox", "Database error");
        assert_eq!(fetch_error.brief_message(), "Cookie fetch failed for firefox: Database error");

        let config_error = BrowserError::InvalidConfiguration("bad config".to_string());
        assert_eq!(config_error.brief_message(), "Invalid configuration: bad config");
    }

    #[test]
    fn test_get_suggestions_unsupported_browser() {
        let error = BrowserError::UnsupportedBrowser("invalid".to_string());
        let suggestions = error.get_suggestions();
        assert_eq!(suggestions.len(), 4);
        assert!(suggestions.contains(&"--browser chrome".to_string()));
        assert!(suggestions.contains(&"--browser firefox".to_string()));
        assert!(suggestions.contains(&"--browser safari".to_string()));
        assert!(suggestions.contains(&"--browser edge".to_string()));
    }

    #[test]
    fn test_get_suggestions_browser_not_available() {
        let error = BrowserError::BrowserNotAvailable("chrome".to_string());
        let suggestions = error.get_suggestions();
        assert!(suggestions.len() >= 1);
        assert!(suggestions.contains(&"Install chrome browser".to_string()));
        // May contain additional suggestions based on available browsers
    }

    #[test]
    fn test_get_suggestions_no_browsers_available() {
        let error = BrowserError::NoBrowsersAvailable;
        let suggestions = error.get_suggestions();
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions.contains(&"Install Chrome, Firefox, Safari, or Edge".to_string()));
        assert!(suggestions.contains(&"Run the browser at least once after installation".to_string()));
    }

    #[test]
    fn test_get_suggestions_cookie_fetch_error() {
        let error = BrowserError::cookie_fetch_error("chrome", "Database locked");
        let suggestions = error.get_suggestions();
        assert!(suggestions.len() >= 1);
        assert!(suggestions.contains(&"Close chrome and try again".to_string()));
        // May contain additional browser suggestions
    }

    #[test]
    fn test_get_suggestions_invalid_configuration() {
        let error = BrowserError::InvalidConfiguration("bad config".to_string());
        let suggestions = error.get_suggestions();
        assert_eq!(suggestions.len(), 4);
        assert!(suggestions.contains(&"Use --browser chrome".to_string()));
        assert!(suggestions.contains(&"Use --browser firefox".to_string()));
        assert!(suggestions.contains(&"Use --browser safari".to_string()));
        assert!(suggestions.contains(&"Use --browser edge".to_string()));
    }

    // Tests for logging behavior
    #[test]
    fn test_logging_browser_strategy_availability_check() {
        // Test that availability checks are logged
        let firefox_strategy = FirefoxStrategy::new();
        let _available = firefox_strategy.is_available();
        // Note: We can't easily test log output in unit tests without a custom logger
        // But we can verify the methods don't panic and complete successfully
        
        let chrome_strategy = ChromeStrategy::new();
        let _available = chrome_strategy.is_available();
        
        let safari_strategy = SafariStrategy::new();
        let _available = safari_strategy.is_available();
        
        let edge_strategy = EdgeStrategy::new();
        let _available = edge_strategy.is_available();
    }

    #[test]
    fn test_logging_cookie_manager_creation() {
        // Test that cookie manager creation is logged
        for browser_type in BrowserType::all() {
            let result = CookieManager::new(browser_type.clone());
            // The result will depend on actual browser availability
            // But we can verify the method completes without panicking
            match result {
                Ok(_manager) => {
                    // Success case - logging should have occurred
                }
                Err(_e) => {
                    // Error case - logging should have occurred
                }
            }
        }
    }

    #[test]
    fn test_logging_auto_detection() {
        // Test that auto-detection process is logged
        let result = CookieManager::with_auto_detection();
        // The result will depend on actual browser availability
        // But we can verify the method completes without panicking
        match result {
            Ok(_manager) => {
                // Success case - logging should have occurred
            }
            Err(_e) => {
                // Error case - logging should have occurred
            }
        }
    }

    #[test]
    fn test_logging_fallback_logic() {
        // Test that fallback logic is logged
        let result = CookieManager::with_fallback(Some(BrowserType::Chrome));
        // The result will depend on actual browser availability
        // But we can verify the method completes without panicking
        match result {
            Ok(_manager) => {
                // Success case - logging should have occurred
            }
            Err(_e) => {
                // Error case - logging should have occurred
            }
        }
    }

    #[test]
    fn test_logging_cookie_fetch_with_mock_strategy() {
        // Test that cookie fetching is logged using mock strategy
        struct LoggingTestStrategy {
            should_succeed: bool,
        }

        impl BrowserStrategy for LoggingTestStrategy {
            fn fetch_cookies(&self, domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
                if self.should_succeed {
                    Ok(vec![Cookie {
                        domain: domains.get(0).unwrap_or(&"example.com".to_string()).clone(),
                        path: "/".to_string(),
                        name: "test_cookie".to_string(),
                        value: "test_value".to_string(),
                        http_only: false,
                        secure: false,
                        same_site: 0,
                        expires: None,
                    }])
                } else {
                    Err(BrowserError::cookie_fetch_error("test", "Mock error for logging test"))
                }
            }

            fn is_available(&self) -> bool {
                true
            }

            fn browser_name(&self) -> &'static str {
                "test"
            }
        }

        // Test successful cookie fetch logging
        let success_manager = CookieManager::with_strategy(Box::new(LoggingTestStrategy { should_succeed: true }));
        let result = success_manager.fetch_cookies_for_domain("example.com".to_string());
        assert!(result.is_ok());

        // Test failed cookie fetch logging
        let error_manager = CookieManager::with_strategy(Box::new(LoggingTestStrategy { should_succeed: false }));
        let result = error_manager.fetch_cookies_for_domain("example.com".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_logging_browser_detection() {
        // Test that browser detection process is logged
        let available_browsers = CookieManager::detect_available_browsers();
        // The result will depend on actual browser availability
        // But we can verify the method completes without panicking
        assert!(available_browsers.len() <= 4); // Should not exceed the number of supported browsers
    }

    #[test]
    fn test_logging_sensitive_data_protection() {
        // Test that sensitive cookie data is not logged in production
        struct SensitiveDataTestStrategy;

        impl BrowserStrategy for SensitiveDataTestStrategy {
            fn fetch_cookies(&self, _domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
                Ok(vec![Cookie {
                    domain: "example.com".to_string(),
                    path: "/".to_string(),
                    name: "session_token".to_string(),
                    value: "super_secret_value_12345".to_string(),
                    http_only: true,
                    secure: true,
                    same_site: 1,
                    expires: None,
                }])
            }

            fn is_available(&self) -> bool {
                true
            }

            fn browser_name(&self) -> &'static str {
                "sensitive_test"
            }
        }

        let manager = CookieManager::with_strategy(Box::new(SensitiveDataTestStrategy));
        let result = manager.fetch_cookies_for_domain("example.com".to_string());
        
        // Verify the cookie fetch works
        assert!(result.is_ok());
        let cookies = result.unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name, "session_token");
        assert_eq!(cookies[0].value, "super_secret_value_12345");
        
        // Note: In actual logging, the cookie value should be redacted as "[REDACTED]"
        // This test verifies the functionality works, but we can't easily test log output
    }

    // Firefox Strategy Tests
    #[test]
    fn test_firefox_strategy_new() {
        let strategy = FirefoxStrategy::new();
        assert_eq!(strategy.browser_name(), "firefox");
    }

    #[test]
    fn test_firefox_strategy_browser_name() {
        let strategy = FirefoxStrategy::new();
        assert_eq!(strategy.browser_name(), "firefox");
    }

    #[test]
    fn test_firefox_strategy_availability() {
        let strategy = FirefoxStrategy::new();
        // This test will depend on the actual system, but we can test the method exists
        let _is_available = strategy.is_available();
        // We can't assert a specific value since it depends on the system
    }

    // Chrome Strategy Tests
    #[test]
    fn test_chrome_strategy_new() {
        let strategy = ChromeStrategy::new();
        assert_eq!(strategy.browser_name(), "chrome");
    }

    #[test]
    fn test_chrome_strategy_browser_name() {
        let strategy = ChromeStrategy::new();
        assert_eq!(strategy.browser_name(), "chrome");
    }

    #[test]
    fn test_chrome_strategy_availability() {
        let strategy = ChromeStrategy::new();
        // This test will depend on the actual system, but we can test the method exists
        let _is_available = strategy.is_available();
        // We can't assert a specific value since it depends on the system
    }

    // Safari Strategy Tests
    #[test]
    fn test_safari_strategy_new() {
        let strategy = SafariStrategy::new();
        assert_eq!(strategy.browser_name(), "safari");
    }

    #[test]
    fn test_safari_strategy_browser_name() {
        let strategy = SafariStrategy::new();
        assert_eq!(strategy.browser_name(), "safari");
    }

    #[test]
    fn test_safari_strategy_availability() {
        let strategy = SafariStrategy::new();
        let is_available = strategy.is_available();

        // Safari should only be available on macOS
        if cfg!(target_os = "macos") {
            // On macOS, availability depends on whether Safari cookies exist
            let _availability = is_available; // Could be true or false
        } else {
            // On non-macOS systems, Safari should not be available
            assert!(!is_available);
        }
    }

    #[test]
    fn test_safari_strategy_fetch_cookies_non_macos() {
        let strategy = SafariStrategy::new();

        if !cfg!(target_os = "macos") {
            let result = strategy.fetch_cookies(vec!["example.com".to_string()]);
            assert!(result.is_err());
            match result.unwrap_err() {
                BrowserError::BrowserNotAvailable(msg) => {
                    assert!(msg.contains("Safari is only available on macOS"));
                }
                _ => panic!("Expected BrowserNotAvailable error"),
            }
        }
    }

    // Edge Strategy Tests
    #[test]
    fn test_edge_strategy_new() {
        let strategy = EdgeStrategy::new();
        assert_eq!(strategy.browser_name(), "edge");
    }

    #[test]
    fn test_edge_strategy_browser_name() {
        let strategy = EdgeStrategy::new();
        assert_eq!(strategy.browser_name(), "edge");
    }

    #[test]
    fn test_edge_strategy_availability() {
        let strategy = EdgeStrategy::new();
        // This test will depend on the actual system, but we can test the method exists
        let _is_available = strategy.is_available();
        // We can't assert a specific value since it depends on the system
    }

    // Test that all strategies implement BrowserStrategy trait
    #[test]
    fn test_all_strategies_implement_browser_strategy() {
        let firefox: Box<dyn BrowserStrategy> = Box::new(FirefoxStrategy::new());
        let chrome: Box<dyn BrowserStrategy> = Box::new(ChromeStrategy::new());
        let safari: Box<dyn BrowserStrategy> = Box::new(SafariStrategy::new());
        let edge: Box<dyn BrowserStrategy> = Box::new(EdgeStrategy::new());

        assert_eq!(firefox.browser_name(), "firefox");
        assert_eq!(chrome.browser_name(), "chrome");
        assert_eq!(safari.browser_name(), "safari");
        assert_eq!(edge.browser_name(), "edge");
    }

    // CookieManager Tests
    #[test]
    fn test_cookie_manager_new_with_available_browser() {
        // This test will depend on what browsers are actually available on the system
        // We'll test the logic by trying each browser type
        for browser_type in BrowserType::all() {
            let result = CookieManager::new(browser_type.clone());
            
            // The result should either be Ok (if browser is available) or 
            // Err(BrowserNotAvailable) if browser is not available
            match result {
                Ok(manager) => {
                    assert_eq!(manager.browser_name(), browser_type.as_str());
                }
                Err(BrowserError::BrowserNotAvailable(browser)) => {
                    assert_eq!(browser, browser_type.as_str());
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }
    }

    #[test]
    fn test_cookie_manager_with_auto_detection() {
        let result = CookieManager::with_auto_detection();
        
        // The result should either be Ok (if any browser is available) or 
        // Err(NoBrowsersAvailable) if no browsers are available
        match result {
            Ok(manager) => {
                // Should be one of the supported browsers
                let browser_name = manager.browser_name();
                assert!(["chrome", "firefox", "safari", "edge"].contains(&browser_name));
            }
            Err(BrowserError::NoBrowsersAvailable) => {
                // This is acceptable if no browsers are available on the system
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_cookie_manager_browser_name() {
        // Test with each browser type if available
        for browser_type in BrowserType::all() {
            if let Ok(manager) = CookieManager::new(browser_type.clone()) {
                assert_eq!(manager.browser_name(), browser_type.as_str());
            }
        }
    }

    #[test]
    fn test_cookie_manager_fetch_cookies_for_domain() {
        // Try to create a manager with auto-detection
        if let Ok(manager) = CookieManager::with_auto_detection() {
            // Test that the method exists and can be called
            // We can't test the actual cookie fetching without real browser data
            let result = manager.fetch_cookies_for_domain("example.com".to_string());
            
            // The result should be either Ok with cookies or an error
            // We can't assert specific values since it depends on actual browser state
            match result {
                Ok(_cookies) => {
                    // Success case - cookies were fetched
                }
                Err(BrowserError::CookieFetchError { browser: _, message: _ }) => {
                    // Expected error case - cookie fetching failed
                }
                Err(e) => panic!("Unexpected error type: {:?}", e),
            }
        }
    }

    // Mock strategy for testing CookieManager logic without depending on actual browsers
    struct MockBrowserStrategy {
        name: &'static str,
        available: bool,
        should_error: bool,
    }

    impl MockBrowserStrategy {
        fn new(name: &'static str, available: bool, should_error: bool) -> Self {
            Self {
                name,
                available,
                should_error,
            }
        }
    }

    impl BrowserStrategy for MockBrowserStrategy {
        fn fetch_cookies(&self, _domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
            if self.should_error {
                Err(BrowserError::cookie_fetch_error(self.name, "Mock error"))
            } else {
                Ok(vec![])
            }
        }

        fn is_available(&self) -> bool {
            self.available
        }

        fn browser_name(&self) -> &'static str {
            self.name
        }
    }

    #[test]
    fn test_cookie_manager_with_mock_strategy() {
        // Test CookieManager behavior with mock strategies
        let mock_strategy = MockBrowserStrategy::new("mock", true, false);
        let manager = CookieManager {
            strategy: Box::new(mock_strategy),
        };

        assert_eq!(manager.browser_name(), "mock");
        
        let result = manager.fetch_cookies_for_domain("example.com".to_string());
        assert!(result.is_ok());
        
        let cookies = result.unwrap();
        assert_eq!(cookies.len(), 0); // Mock returns empty vec
    }

    #[test]
    fn test_cookie_manager_with_mock_strategy_error() {
        // Test CookieManager error handling with mock strategy
        let mock_strategy = MockBrowserStrategy::new("mock", true, true);
        let manager = CookieManager {
            strategy: Box::new(mock_strategy),
        };

        let result = manager.fetch_cookies_for_domain("example.com".to_string());
        assert!(result.is_err());
        
        match result.unwrap_err() {
            BrowserError::CookieFetchError { browser, message } => {
                assert_eq!(browser, "mock");
                assert_eq!(message, "Mock error");
            }
            _ => panic!("Expected CookieFetchError"),
        }
    }

    // Auto-detection tests
    #[test]
    fn test_detect_available_browsers() {
        let available_browsers = CookieManager::detect_available_browsers();
        
        // Should return a vector (could be empty if no browsers are available)
        // Each browser in the list should be one of the supported types
        for browser in &available_browsers {
            assert!(BrowserType::all().contains(browser));
        }
        
        // Should be in priority order (Chrome, Firefox, Safari, Edge)
        let mut expected_order = Vec::new();
        for browser_type in [BrowserType::Chrome, BrowserType::Firefox, BrowserType::Safari, BrowserType::Edge] {
            let strategy: Box<dyn BrowserStrategy> = match browser_type {
                BrowserType::Chrome => Box::new(ChromeStrategy::new()),
                BrowserType::Firefox => Box::new(FirefoxStrategy::new()),
                BrowserType::Safari => Box::new(SafariStrategy::new()),
                BrowserType::Edge => Box::new(EdgeStrategy::new()),
            };
            
            if strategy.is_available() {
                expected_order.push(browser_type);
            }
        }
        
        assert_eq!(available_browsers, expected_order);
    }

    #[test]
    fn test_cookie_manager_with_fallback_preferred_available() {
        // Test fallback when preferred browser is available
        for browser_type in BrowserType::all() {
            let strategy: Box<dyn BrowserStrategy> = match browser_type {
                BrowserType::Chrome => Box::new(ChromeStrategy::new()),
                BrowserType::Firefox => Box::new(FirefoxStrategy::new()),
                BrowserType::Safari => Box::new(SafariStrategy::new()),
                BrowserType::Edge => Box::new(EdgeStrategy::new()),
            };
            
            if strategy.is_available() {
                let result = CookieManager::with_fallback(Some(browser_type.clone()));
                match result {
                    Ok(manager) => {
                        assert_eq!(manager.browser_name(), browser_type.as_str());
                    }
                    Err(e) => panic!("Unexpected error for available browser {}: {:?}", browser_type, e),
                }
            }
        }
    }

    #[test]
    fn test_cookie_manager_with_fallback_no_preference() {
        // Test fallback with no preferred browser (should behave like auto-detection)
        let result_fallback = CookieManager::with_fallback(None);
        let result_auto = CookieManager::with_auto_detection();
        
        match (result_fallback, result_auto) {
            (Ok(manager_fallback), Ok(manager_auto)) => {
                // Both should select the same browser (first available in priority order)
                assert_eq!(manager_fallback.browser_name(), manager_auto.browser_name());
            }
            (Err(BrowserError::NoBrowsersAvailable), Err(BrowserError::NoBrowsersAvailable)) => {
                // Both should fail with the same error if no browsers are available
            }
            _ => panic!("Fallback and auto-detection should behave the same when no preference is given"),
        }
    }

    #[test]
    fn test_cookie_manager_with_fallback_preferred_unavailable() {
        // This test is tricky because we need to test with an unavailable browser
        // We'll create a scenario by testing all browsers and finding one that's not available
        let available_browsers = CookieManager::detect_available_browsers();
        let all_browsers = BrowserType::all();
        
        // Find a browser that's not available
        let unavailable_browser = all_browsers.iter().find(|&browser| !available_browsers.contains(browser));
        
        if let Some(unavailable_browser) = unavailable_browser {
            let result = CookieManager::with_fallback(Some(unavailable_browser.clone()));
            
            if available_browsers.is_empty() {
                // If no browsers are available, should get NoBrowsersAvailable
                match result {
                    Err(BrowserError::NoBrowsersAvailable) => {}
                    _ => panic!("Expected NoBrowsersAvailable when no browsers are available"),
                }
            } else {
                // If other browsers are available, should fall back to auto-detection
                match result {
                    Ok(manager) => {
                        // Should not be the unavailable browser
                        assert_ne!(manager.browser_name(), unavailable_browser.as_str());
                        // Should be one of the available browsers
                        let browser_name = manager.browser_name();
                        assert!(["chrome", "firefox", "safari", "edge"].contains(&browser_name));
                    }
                    Err(e) => panic!("Unexpected error during fallback: {:?}", e),
                }
            }
        }
    }

    #[test]
    fn test_auto_detection_priority_order() {
        // Test that auto-detection follows the correct priority order
        let available_browsers = CookieManager::detect_available_browsers();
        
        if !available_browsers.is_empty() {
            let result = CookieManager::with_auto_detection();
            match result {
                Ok(manager) => {
                    // The selected browser should be the first in the available browsers list
                    assert_eq!(manager.browser_name(), available_browsers[0].as_str());
                }
                Err(e) => panic!("Auto-detection failed despite available browsers: {:?}", e),
            }
        }
    }

    #[test]
    fn test_auto_detection_comprehensive_error_handling() {
        // Test that auto-detection handles the case where no browsers are available
        // This is hard to test directly, but we can test the logic
        
        let available_browsers = CookieManager::detect_available_browsers();
        let result = CookieManager::with_auto_detection();
        
        if available_browsers.is_empty() {
            match result {
                Err(BrowserError::NoBrowsersAvailable) => {
                    // Expected behavior when no browsers are available
                }
                _ => panic!("Expected NoBrowsersAvailable error when no browsers are detected"),
            }
        } else {
            match result {
                Ok(_) => {
                    // Expected behavior when browsers are available
                }
                Err(e) => panic!("Unexpected error when browsers are available: {:?}", e),
            }
        }
    }
}
