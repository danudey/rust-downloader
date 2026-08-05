use rookie::common::enums::Cookie;
use rookie::{chrome, chromium, chromium_based, edge, firefox};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[cfg(unix)]
use rookie::config::get_browser_config;
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
    Chromium,
    Firefox,
    Safari,
    Edge,
    /// Brave, either edition, whichever is installed
    Brave,
    /// The standard Brave Browser release
    BraveStandard,
    /// The Brave Origin edition, which keeps its profile in a separate directory
    BraveOrigin,
}

impl BrowserType {
    /// Get all supported browser types
    pub fn all() -> Vec<BrowserType> {
        vec![
            BrowserType::Chrome,
            BrowserType::Chromium,
            BrowserType::Firefox,
            BrowserType::Safari,
            BrowserType::Edge,
            BrowserType::Brave,
            BrowserType::BraveStandard,
            BrowserType::BraveOrigin,
        ]
    }

    /// Get the browser types to check when auto-detecting, in priority order.
    ///
    /// This deliberately lists the two Brave editions rather than the `brave`
    /// alias, so a detected browser always names the edition it came from.
    pub fn detection_priority() -> Vec<BrowserType> {
        vec![
            BrowserType::Chrome,
            BrowserType::Chromium,
            BrowserType::BraveStandard,
            BrowserType::BraveOrigin,
            BrowserType::Firefox,
            BrowserType::Safari,
            BrowserType::Edge,
        ]
    }

    /// Get the string representation of the browser type
    pub fn as_str(&self) -> &'static str {
        match self {
            BrowserType::Chrome => "chrome",
            BrowserType::Chromium => "chromium",
            BrowserType::Firefox => "firefox",
            BrowserType::Safari => "safari",
            BrowserType::Edge => "edge",
            BrowserType::Brave => "brave",
            BrowserType::BraveStandard => "brave-standard",
            BrowserType::BraveOrigin => "brave-origin",
        }
    }

    /// Build the cookie-fetching strategy for this browser type
    pub fn strategy(&self) -> Box<dyn BrowserStrategy> {
        match self {
            BrowserType::Chrome => Box::new(ChromeStrategy::new()),
            BrowserType::Chromium => Box::new(ChromiumStrategy::new()),
            BrowserType::Firefox => Box::new(FirefoxStrategy::new()),
            BrowserType::Safari => Box::new(SafariStrategy::new()),
            BrowserType::Edge => Box::new(EdgeStrategy::new()),
            BrowserType::Brave => Box::new(BraveStrategy::new()),
            BrowserType::BraveStandard => Box::new(BraveStandardStrategy::new()),
            BrowserType::BraveOrigin => Box::new(BraveOriginStrategy::new()),
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
            "chromium" => Ok(BrowserType::Chromium),
            "firefox" => Ok(BrowserType::Firefox),
            "safari" => Ok(BrowserType::Safari),
            "edge" => Ok(BrowserType::Edge),
            "brave" => Ok(BrowserType::Brave),
            "brave-standard" | "brave_standard" | "brave-browser" | "brave_browser" => {
                Ok(BrowserType::BraveStandard)
            }
            "brave-origin" | "brave_origin" => Ok(BrowserType::BraveOrigin),
            _ => Err(BrowserError::UnsupportedBrowser { browser: s.to_string()}),
        }
    }
}

/// Comprehensive error types for browser operations
#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("Browser '{browser}' is not supported. Available browsers: {}", 
            BrowserType::all().iter().map(|b| b.as_str()).collect::<Vec<_>>().join(", ")
        )]
    UnsupportedBrowser { browser: String },

    #[error("Browser '{browser}' is not available or installed")]
    BrowserNotAvailable { browser: String },

    #[error("No supported browsers found. Please install one of: {}", 
            BrowserType::all().iter().map(|b| b.as_str()).collect::<Vec<_>>().join(", "))]
    NoBrowsersAvailable,

    #[error("Failed to fetch cookies from {browser}: {message}")]
    CookieFetchError { browser: String, message: String },
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
            BrowserError::UnsupportedBrowser { browser } => {
                Self::format_unsupported_browser_message(browser)
            }
            BrowserError::BrowserNotAvailable { browser } => {
                Self::format_browser_not_available_message(browser)
            }
            BrowserError::NoBrowsersAvailable => {
                Self::format_no_browsers_available_message()
            }
            BrowserError::CookieFetchError { browser, message } => {
                Self::format_cookie_fetch_error_message(browser, message)
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
            "⛔ Browser '{}' is not available or installed.\n\n{}",
            browser, fallback_suggestion
        )
    }

    /// Format user-friendly message for no browsers available errors
    fn format_no_browsers_available_message() -> String {
        String::from("No supported browsers found on your system.")
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

    /// Get a brief error message without formatting (for logging)
    pub fn brief_message(&self) -> String {
        match self {
            BrowserError::UnsupportedBrowser { browser } => {
                format!("Unsupported browser: {}", browser)
            }
            BrowserError::BrowserNotAvailable { browser } => {
                format!("Browser not available: {}", browser)
            }
            BrowserError::NoBrowsersAvailable => {
                "No browsers available".to_string()
            }
            BrowserError::CookieFetchError { browser, message } => {
                format!("Cookie fetch failed for {}: {}", browser, message)
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

pub struct ChromiumStrategy;

impl ChromiumStrategy {
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
                    .join("chromium")
                    .join("Default")
                    .join("Cookies"),
                home_dir
                    .join("Library")
                    .join("Application Support")
                    .join("Google")
                    .join("Chromium")
                    .join("Default")
                    .join("Cookies"),
                home_dir
                    .join("AppData")
                    .join("Local")
                    .join("Google")
                    .join("Chromium")
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

impl BrowserStrategy for ChromiumStrategy {
    fn fetch_cookies(&self, domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
        debug!("Attempting to fetch cookies from Chromium for domains: {:?}", domains);
        match chromium(Some(domains.clone())) {
            Ok(cookies) => {
                info!("Successfully fetched {} cookies from Chromium for domains: {:?}", 
                      cookies.len(), domains);
                debug!("Chromium cookies: {:?}", cookies.iter().map(|c| format!("{}={}", c.name, "[REDACTED]")).collect::<Vec<_>>());
                Ok(cookies)
            }
            Err(e) => {
                error!("Failed to fetch cookies from Chromium for domains {:?}: {}", domains, e);
                Err(BrowserError::cookie_fetch_error("chromium", e))
            }
        }
    }

    fn is_available(&self) -> bool {
        let available = Self::chrome_cookies_exist();
        debug!("Chromium availability check: {}", available);
        available
    }

    fn browser_name(&self) -> &'static str {
        "chromium"
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
            Err(BrowserError::BrowserNotAvailable {
                browser: "Safari is only available on macOS".to_string()
            })
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

/// A Brave edition, and the packaging-specific names needed to find it
struct BraveEdition {
    /// The directory it keeps its profiles in, under `BraveSoftware`
    product_dir: &'static str,
    /// The Flatpak application id, for editions published as a Flatpak
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    flatpak_app_id: Option<&'static str>,
    /// The snap package name, for editions published as a snap
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    snap_package: Option<&'static str>,
}

/// The standard Brave Browser release
const BRAVE_STANDARD: BraveEdition = BraveEdition {
    product_dir: "Brave-Browser",
    flatpak_app_id: Some("com.brave.Browser"),
    snap_package: Some("brave"),
};

/// The Brave Origin edition, which is only distributed as a native package
const BRAVE_ORIGIN: BraveEdition = BraveEdition {
    product_dir: "Brave-Origin",
    flatpak_app_id: None,
    snap_package: None,
};

/// Non-stable channels install alongside the stable release, with the channel
/// name appended to the product directory (e.g. `Brave-Browser-Beta`). Stable
/// comes first, so a stable install always wins.
const BRAVE_CHANNELS: [&str; 4] = ["", "-Beta", "-Development", "-Nightly"];

/// A located Brave profile directory and the files needed to read its cookies
struct BraveProfile {
    /// The cookie database itself
    cookies: PathBuf,
    /// The `Local State` file holding the encryption key. Only Windows needs
    /// it; the other platforms take the key from the keyring or keychain.
    #[cfg_attr(unix, allow(dead_code))]
    local_state: PathBuf,
}

/// Every directory a Brave edition might keep its profiles in, across every
/// release channel.
fn brave_profile_roots(edition: &BraveEdition) -> Vec<PathBuf> {
    BRAVE_CHANNELS
        .iter()
        .flat_map(|channel| {
            brave_channel_roots(edition, &format!("{}{}", edition.product_dir, channel))
        })
        .collect()
}

/// The directories one channel of a Brave edition might keep its profiles in
///
/// Only this platform's locations are listed; the others cannot match anything.
#[allow(unused_variables)]
fn brave_channel_roots(edition: &BraveEdition, product_dir: &str) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    #[cfg(target_os = "linux")]
    if let Some(home_dir) = dirs::home_dir() {
        roots.push(
            home_dir
                .join(".config")
                .join("BraveSoftware")
                .join(product_dir),
        );

        if let Some(app_id) = edition.flatpak_app_id {
            roots.push(
                home_dir
                    .join(".var")
                    .join("app")
                    .join(app_id)
                    .join("config")
                    .join("BraveSoftware")
                    .join(product_dir),
            );
        }

        // The snap revision number is part of the path, so enumerate them
        if let Some(package) = edition.snap_package
            && let Ok(revisions) = std::fs::read_dir(home_dir.join("snap").join(package))
        {
            roots.extend(revisions.flatten().map(|revision| {
                revision
                    .path()
                    .join(".config")
                    .join("BraveSoftware")
                    .join(product_dir)
            }));
        }
    }

    #[cfg(target_os = "macos")]
    if let Some(home_dir) = dirs::home_dir() {
        roots.push(
            home_dir
                .join("Library")
                .join("Application Support")
                .join("BraveSoftware")
                .join(product_dir),
        );
    }

    // Brave installs under %LOCALAPPDATA%, but a roaming profile can put it
    // under %APPDATA% instead, so check both.
    #[cfg(target_os = "windows")]
    for base in [dirs::data_local_dir(), dirs::data_dir()].into_iter().flatten() {
        roots.push(
            base.join("BraveSoftware")
                .join(product_dir)
                .join("User Data"),
        );
    }

    roots
}

/// The profile directories inside a Brave root, in the order Brave creates
/// them: `Default` first, then any `Profile N` directories, by name.
fn brave_profile_dirs(root: &Path) -> Vec<PathBuf> {
    let mut profiles = vec![root.join("Default")];

    if let Ok(entries) = std::fs::read_dir(root) {
        let mut numbered: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("Profile "))
            })
            .collect();

        numbered.sort();
        profiles.extend(numbered);
    }

    profiles
}

/// Find the cookie database inside one Brave profile root, if there is one
fn find_brave_profile_in(root: &Path) -> Option<BraveProfile> {
    for profile in brave_profile_dirs(root) {
        // Newer Chromium releases moved the cookie database under Network/
        let candidates = [
            profile.join("Network").join("Cookies"),
            profile.join("Cookies"),
        ];

        if let Some(cookies) = candidates.into_iter().find(|path| path.is_file()) {
            debug!("Found Brave cookie database at {}", cookies.display());
            return Some(BraveProfile {
                cookies,
                local_state: root.join("Local State"),
            });
        }
    }

    None
}

/// Find the profile of an installed Brave edition, if there is one
fn find_brave_profile(edition: &BraveEdition) -> Option<BraveProfile> {
    brave_profile_roots(edition)
        .iter()
        .find_map(|root| find_brave_profile_in(root))
}

/// Read cookies from a Brave cookie database at a known path.
///
/// Both editions read cookies this way, so availability and fetching always
/// agree about which profile they are talking about. `rookie::brave` is not
/// used because it does its own path lookup, which knows nothing about Brave
/// Origin and disagrees with this one about channels and profile directories.
fn read_brave_cookies(
    profile: &BraveProfile,
    domains: Vec<String>,
) -> rookie::Result<Vec<Cookie>> {
    // Brave's own config names the keyring entry ("Brave Safe Storage") that
    // holds the decryption key. `rookie::any_browser` would try Chrome's entry
    // first, which prompts for the wrong keychain item on macOS.
    #[cfg(unix)]
    {
        chromium_based(
            get_browser_config("brave"),
            profile.cookies.clone(),
            Some(domains),
        )
    }

    // On Windows the key lives in the profile's own `Local State` file
    #[cfg(windows)]
    {
        chromium_based(
            profile.local_state.clone(),
            profile.cookies.clone(),
            Some(domains),
        )
    }
}

/// Fetch cookies for one Brave edition, reporting errors under `browser_name`
fn fetch_brave_cookies(
    edition: &BraveEdition,
    browser_name: &'static str,
    domains: Vec<String>,
) -> Result<Vec<Cookie>, BrowserError> {
    debug!("Attempting to fetch cookies from {} for domains: {:?}", browser_name, domains);

    let Some(profile) = find_brave_profile(edition) else {
        warn!("No {} profile found while fetching cookies for domains: {:?}", browser_name, domains);
        return Err(BrowserError::BrowserNotAvailable {
            browser: browser_name.to_string(),
        });
    };

    match read_brave_cookies(&profile, domains.clone()) {
        Ok(cookies) => {
            info!("Successfully fetched {} cookies from {} for domains: {:?}",
                  cookies.len(), browser_name, domains);
            debug!("{} cookies: {:?}", browser_name, cookies.iter().map(|c| format!("{}={}", c.name, "[REDACTED]")).collect::<Vec<_>>());
            Ok(cookies)
        }
        Err(e) => {
            error!("Failed to fetch cookies from {} for domains {:?}: {}", browser_name, domains, e);
            Err(BrowserError::cookie_fetch_error(browser_name, e))
        }
    }
}

/// Standard Brave Browser strategy implementation
pub struct BraveStandardStrategy;

impl BraveStandardStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl BrowserStrategy for BraveStandardStrategy {
    fn fetch_cookies(&self, domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
        fetch_brave_cookies(&BRAVE_STANDARD, "brave-standard", domains)
    }

    fn is_available(&self) -> bool {
        let available = find_brave_profile(&BRAVE_STANDARD).is_some();
        debug!("Brave (standard) availability check: {}", available);
        available
    }

    fn browser_name(&self) -> &'static str {
        "brave-standard"
    }
}

/// Brave Origin strategy implementation
///
/// Brave Origin stores its profile under `BraveSoftware/Brave-Origin` rather
/// than `BraveSoftware/Brave-Browser`, which the cookie library does not know
/// about.
pub struct BraveOriginStrategy;

impl BraveOriginStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl BrowserStrategy for BraveOriginStrategy {
    fn fetch_cookies(&self, domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
        fetch_brave_cookies(&BRAVE_ORIGIN, "brave-origin", domains)
    }

    fn is_available(&self) -> bool {
        let available = find_brave_profile(&BRAVE_ORIGIN).is_some();
        debug!("Brave Origin availability check: {}", available);
        available
    }

    fn browser_name(&self) -> &'static str {
        "brave-origin"
    }
}

/// Brave strategy that picks whichever edition is installed
///
/// The standard release wins if both are present.
pub struct BraveStrategy {
    edition: Option<Box<dyn BrowserStrategy>>,
}

impl BraveStrategy {
    pub fn new() -> Self {
        let standard = BraveStandardStrategy::new();
        let origin = BraveOriginStrategy::new();

        let edition: Option<Box<dyn BrowserStrategy>> = if standard.is_available() {
            Some(Box::new(standard))
        } else if origin.is_available() {
            Some(Box::new(origin))
        } else {
            None
        };

        match &edition {
            Some(strategy) => info!("Brave edition detected: {}", strategy.browser_name()),
            None => debug!("No Brave edition detected"),
        }

        Self { edition }
    }
}

impl BrowserStrategy for BraveStrategy {
    fn fetch_cookies(&self, domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError> {
        match &self.edition {
            Some(strategy) => strategy.fetch_cookies(domains),
            None => {
                warn!("Brave cookie fetch attempted with no Brave edition installed");
                Err(BrowserError::BrowserNotAvailable {
                    browser: "brave".to_string(),
                })
            }
        }
    }

    fn is_available(&self) -> bool {
        self.edition.is_some()
    }

    fn browser_name(&self) -> &'static str {
        match &self.edition {
            Some(strategy) => strategy.browser_name(),
            None => "brave",
        }
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
        
        let strategy = browser_type.strategy();

        // Check if the selected browser is available
        if !strategy.is_available() {
            warn!("Selected browser {} is not available", browser_type);
            return Err(BrowserError::BrowserNotAvailable {
                browser: browser_type.as_str().to_string()},
            );
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

    /// Detect all available browsers in priority order (see `BrowserType::detection_priority`)
    pub fn detect_available_browsers() -> Vec<BrowserType> {
        debug!("Starting browser detection process");

        let mut available_browsers = Vec::new();

        for browser_type in BrowserType::detection_priority() {
            debug!("Checking availability of {}", browser_type);

            if browser_type.strategy().is_available() {
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
                Err(BrowserError::BrowserNotAvailable { browser: _ }) => {
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

    /// Match a browser name reported by a strategy against the type it came from.
    ///
    /// `brave` resolves to whichever edition is installed, so it answers to the
    /// names of both editions as well as its own. The tests need this because
    /// they cannot assume which browsers a given machine has installed.
    trait ReportsAs {
        fn reports_as(&self, browser_name: &str) -> bool;
    }

    impl ReportsAs for BrowserType {
        fn reports_as(&self, browser_name: &str) -> bool {
            match self {
                BrowserType::Brave => [
                    BrowserType::Brave.as_str(),
                    BrowserType::BraveStandard.as_str(),
                    BrowserType::BraveOrigin.as_str(),
                ]
                .contains(&browser_name),
                other => other.as_str() == browser_name,
            }
        }
    }

    #[test]
    fn test_browser_type_from_str_valid() {
        assert_eq!(
            "chrome".parse::<BrowserType>().unwrap(),
            BrowserType::Chrome
        );
        assert_eq!(
            "chromium".parse::<BrowserType>().unwrap(),
            BrowserType::Chromium
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
        assert_eq!("brave".parse::<BrowserType>().unwrap(), BrowserType::Brave);
        assert_eq!(
            "brave-standard".parse::<BrowserType>().unwrap(),
            BrowserType::BraveStandard
        );
        assert_eq!(
            "brave-origin".parse::<BrowserType>().unwrap(),
            BrowserType::BraveOrigin
        );
    }

    #[test]
    fn test_browser_type_from_str_brave_aliases() {
        for alias in ["brave_standard", "brave-browser", "brave_browser"] {
            assert_eq!(
                alias.parse::<BrowserType>().unwrap(),
                BrowserType::BraveStandard,
                "alias {} should parse as the standard Brave release",
                alias
            );
        }

        assert_eq!(
            "brave_origin".parse::<BrowserType>().unwrap(),
            BrowserType::BraveOrigin
        );
    }

    #[test]
    fn test_brave_reports_as_either_edition() {
        // 'brave' resolves to whichever edition is installed
        assert!(BrowserType::Brave.reports_as("brave"));
        assert!(BrowserType::Brave.reports_as("brave-standard"));
        assert!(BrowserType::Brave.reports_as("brave-origin"));
        assert!(!BrowserType::Brave.reports_as("chrome"));

        // The specific editions only answer to their own name
        assert!(BrowserType::BraveStandard.reports_as("brave-standard"));
        assert!(!BrowserType::BraveStandard.reports_as("brave-origin"));
        assert!(!BrowserType::BraveStandard.reports_as("brave"));
        assert!(BrowserType::BraveOrigin.reports_as("brave-origin"));
        assert!(!BrowserType::BraveOrigin.reports_as("brave-standard"));
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
        assert_eq!("BRAVE".parse::<BrowserType>().unwrap(), BrowserType::Brave);
        assert_eq!(
            "Brave-Origin".parse::<BrowserType>().unwrap(),
            BrowserType::BraveOrigin
        );
        assert_eq!(
            "Brave-Standard".parse::<BrowserType>().unwrap(),
            BrowserType::BraveStandard
        );
    }

    #[test]
    fn test_browser_type_from_str_invalid() {
        let result = "invalid".parse::<BrowserType>();
        assert!(result.is_err());
        match result.unwrap_err() {
            BrowserError::UnsupportedBrowser { browser } => {
                assert_eq!(browser, "invalid");
            }
            _ => panic!("Expected UnsupportedBrowser error"),
        }
    }

    #[test]
    fn test_browser_type_display() {
        assert_eq!(BrowserType::Chrome.to_string(), "chrome");
        assert_eq!(BrowserType::Chromium.to_string(), "chromium");
        assert_eq!(BrowserType::Firefox.to_string(), "firefox");
        assert_eq!(BrowserType::Safari.to_string(), "safari");
        assert_eq!(BrowserType::Edge.to_string(), "edge");
        assert_eq!(BrowserType::Brave.to_string(), "brave");
        assert_eq!(BrowserType::BraveStandard.to_string(), "brave-standard");
        assert_eq!(BrowserType::BraveOrigin.to_string(), "brave-origin");
    }

    #[test]
    fn test_browser_type_as_str() {
        assert_eq!(BrowserType::Chrome.as_str(), "chrome");
        assert_eq!(BrowserType::Chromium.as_str(), "chromium");
        assert_eq!(BrowserType::Firefox.as_str(), "firefox");
        assert_eq!(BrowserType::Safari.as_str(), "safari");
        assert_eq!(BrowserType::Edge.as_str(), "edge");
        assert_eq!(BrowserType::Brave.as_str(), "brave");
        assert_eq!(BrowserType::BraveStandard.as_str(), "brave-standard");
        assert_eq!(BrowserType::BraveOrigin.as_str(), "brave-origin");
    }

    #[test]
    fn test_browser_type_all() {
        let all_browsers = BrowserType::all();
        assert_eq!(all_browsers.len(), 8);
        assert!(all_browsers.contains(&BrowserType::Chrome));
        assert!(all_browsers.contains(&BrowserType::Chromium));
        assert!(all_browsers.contains(&BrowserType::Firefox));
        assert!(all_browsers.contains(&BrowserType::Safari));
        assert!(all_browsers.contains(&BrowserType::Edge));
        assert!(all_browsers.contains(&BrowserType::Brave));
        assert!(all_browsers.contains(&BrowserType::BraveStandard));
        assert!(all_browsers.contains(&BrowserType::BraveOrigin));
    }

    #[test]
    fn test_browser_type_detection_priority_excludes_brave_alias() {
        let priority = BrowserType::detection_priority();

        // Detection reports the edition it found, so the alias never appears
        assert!(!priority.contains(&BrowserType::Brave));
        assert!(priority.contains(&BrowserType::BraveStandard));
        assert!(priority.contains(&BrowserType::BraveOrigin));

        // Everything it does list must be a supported browser
        for browser_type in &priority {
            assert!(BrowserType::all().contains(browser_type));
        }
    }

    #[test]
    fn test_browser_error_unsupported_browser_message() {
        let error = BrowserError::UnsupportedBrowser { browser: "invalid".to_string() };
        let message = error.to_string();
        assert!(message.contains("invalid"));
        assert!(message.contains("chrome"));
        assert!(message.contains("chromium"));
        assert!(message.contains("firefox"));
        assert!(message.contains("safari"));
        assert!(message.contains("edge"));
        assert!(message.contains("brave-origin"));
    }

    #[test]
    fn test_browser_error_no_browsers_available_message() {
        let error = BrowserError::NoBrowsersAvailable;
        let message = error.to_string();
        assert!(message.contains("No supported browsers found"));
        assert!(message.contains("chrome"));
        assert!(message.contains("chromium"));
        assert!(message.contains("firefox"));
        assert!(message.contains("safari"));
        assert!(message.contains("edge"));
        assert!(message.contains("brave-origin"));
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
        assert!(message.contains(
            "Available browsers: chrome, chromium, firefox, safari, edge, brave, brave-standard, brave-origin"
        ));
    }

    #[test]
    fn test_format_browser_not_available_message_chrome() {
        let message = BrowserError::format_browser_not_available_message("chrome");
        assert!(message.contains("⛔ Browser 'chrome' is not available"));
    }

    #[test]
    fn test_format_browser_not_available_message_chromium() {
        let message = BrowserError::format_browser_not_available_message("chromium");
        assert!(message.contains("⛔ Browser 'chromium' is not available"));
    }

    #[test]
    fn test_format_browser_not_available_message_firefox() {
        let message = BrowserError::format_browser_not_available_message("firefox");
        assert!(message.contains("⛔ Browser 'firefox' is not available"));
    }

    #[test]
    fn test_format_browser_not_available_message_safari() {
        let message = BrowserError::format_browser_not_available_message("safari");
        assert!(message.contains("⛔ Browser 'safari' is not available"));
    }

    #[test]
    fn test_format_browser_not_available_message_edge() {
        let message = BrowserError::format_browser_not_available_message("edge");
        assert!(message.contains("⛔ Browser 'edge' is not available"));
    }

    #[test]
    fn test_format_browser_not_available_message_brave() {
        for browser in ["brave", "brave-standard", "brave-origin"] {
            let message = BrowserError::format_browser_not_available_message(browser);
            assert!(message.contains(&format!("⛔ Browser '{}' is not available", browser)));
        }
    }

    #[test]
    fn test_format_no_browsers_available_message() {
        let message = BrowserError::format_no_browsers_available_message();
        assert!(message.contains("No supported browsers found"));
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
    fn test_brief_message() {
        let unsupported = BrowserError::UnsupportedBrowser { browser: "invalid".to_string()};
        assert_eq!(unsupported.brief_message(), "Unsupported browser: invalid");

        let not_available = BrowserError::BrowserNotAvailable { browser: "chrome".to_string() };
        assert_eq!(not_available.brief_message(), "Browser not available: chrome");

        let no_browsers = BrowserError::NoBrowsersAvailable;
        assert_eq!(no_browsers.brief_message(), "No browsers available");

        let fetch_error = BrowserError::cookie_fetch_error("firefox", "Database error");
        assert_eq!(fetch_error.brief_message(), "Cookie fetch failed for firefox: Database error");
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
        // Should not exceed the number of browsers we check for
        assert!(available_browsers.len() <= BrowserType::detection_priority().len());
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
                BrowserError::BrowserNotAvailable { browser: msg} => {
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

    // Brave Strategy Tests
    #[test]
    fn test_brave_standard_strategy_browser_name() {
        let strategy = BraveStandardStrategy::new();
        assert_eq!(strategy.browser_name(), "brave-standard");
    }

    #[test]
    fn test_brave_origin_strategy_browser_name() {
        let strategy = BraveOriginStrategy::new();
        assert_eq!(strategy.browser_name(), "brave-origin");
    }

    #[test]
    fn test_brave_strategy_availability() {
        let strategy = BraveStrategy::new();
        // Depends on the system, but the method should complete
        let _is_available = strategy.is_available();
    }

    #[test]
    fn test_brave_strategy_reports_the_edition_it_found() {
        let strategy = BraveStrategy::new();

        if strategy.is_available() {
            // When an edition is installed, the name identifies which one
            assert!(["brave-standard", "brave-origin"].contains(&strategy.browser_name()));
        } else {
            // With nothing installed there is no edition to name
            assert_eq!(strategy.browser_name(), "brave");
        }
    }

    #[test]
    fn test_brave_strategy_prefers_standard_edition() {
        let strategy = BraveStrategy::new();

        if BraveStandardStrategy::new().is_available() {
            assert_eq!(strategy.browser_name(), "brave-standard");
        }
    }

    #[test]
    fn test_brave_strategy_fetch_without_any_edition_errors() {
        let strategy = BraveStrategy::new();

        if !strategy.is_available() {
            let result = strategy.fetch_cookies(vec!["example.com".to_string()]);
            match result.unwrap_err() {
                BrowserError::BrowserNotAvailable { browser } => {
                    assert_eq!(browser, "brave");
                }
                e => panic!("Expected BrowserNotAvailable, got {:?}", e),
            }
        }
    }

    /// A throwaway directory to build fake Brave profile trees in
    struct ScratchDir {
        path: PathBuf,
    }

    impl ScratchDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("rustdl-{}-{}", name, std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("could not create scratch directory");
            Self { path }
        }

        /// Create an empty file, and every directory leading to it
        fn touch(&self, relative: &str) -> PathBuf {
            let path = self.path.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).expect("could not create directory");
            std::fs::write(&path, b"").expect("could not create file");
            path
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_find_brave_profile_in_finds_default_profile() {
        let scratch = ScratchDir::new("brave-default");
        let cookies = scratch.touch("Default/Cookies");
        scratch.touch("Local State");

        let profile = find_brave_profile_in(&scratch.path).expect("profile should be found");
        assert_eq!(profile.cookies, cookies);
        assert_eq!(profile.local_state, scratch.path.join("Local State"));
    }

    #[test]
    fn test_find_brave_profile_in_prefers_network_subdirectory() {
        // Newer Chromium releases keep the live database under Network/ and
        // may leave a stale one behind at the old location
        let scratch = ScratchDir::new("brave-network");
        scratch.touch("Default/Cookies");
        let network_cookies = scratch.touch("Default/Network/Cookies");

        let profile = find_brave_profile_in(&scratch.path).expect("profile should be found");
        assert_eq!(profile.cookies, network_cookies);
    }

    #[test]
    fn test_find_brave_profile_in_falls_back_to_numbered_profiles() {
        // A user whose only profile is 'Profile 1' still has cookies to read
        let scratch = ScratchDir::new("brave-numbered");
        std::fs::create_dir_all(scratch.path.join("Default")).unwrap();
        let cookies = scratch.touch("Profile 1/Cookies");

        let profile = find_brave_profile_in(&scratch.path).expect("profile should be found");
        assert_eq!(profile.cookies, cookies);
    }

    #[test]
    fn test_find_brave_profile_in_prefers_default_over_numbered_profiles() {
        let scratch = ScratchDir::new("brave-default-wins");
        let default_cookies = scratch.touch("Default/Cookies");
        scratch.touch("Profile 1/Cookies");

        let profile = find_brave_profile_in(&scratch.path).expect("profile should be found");
        assert_eq!(profile.cookies, default_cookies);
    }

    #[test]
    fn test_find_brave_profile_in_ignores_a_root_without_cookies() {
        let scratch = ScratchDir::new("brave-empty");
        scratch.touch("Default/Preferences");
        scratch.touch("NativeMessagingHosts/example.json");

        assert!(find_brave_profile_in(&scratch.path).is_none());
    }

    #[test]
    fn test_find_brave_profile_in_ignores_a_cookie_directory() {
        // Only a file is a database; a directory of that name is not
        let scratch = ScratchDir::new("brave-cookie-dir");
        std::fs::create_dir_all(scratch.path.join("Default").join("Cookies")).unwrap();

        assert!(find_brave_profile_in(&scratch.path).is_none());
    }

    #[test]
    fn test_brave_profile_roots_cover_every_channel() {
        let roots = brave_profile_roots(&BRAVE_STANDARD);
        assert!(!roots.is_empty());

        let joined = roots
            .iter()
            .map(|root| root.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("\n");

        // Every channel Brave installs alongside the stable release
        for channel in ["-Beta", "-Development", "-Nightly"] {
            assert!(
                joined.contains(&format!("Brave-Browser{}", channel)),
                "no root for the {} channel in:\n{}",
                channel,
                joined
            );
        }

        // Stable is checked first, so an everyday install always wins
        assert!(
            roots[0].ends_with("Brave-Browser") || roots[0].ends_with("Brave-Browser/User Data"),
            "stable should be checked first, got {}",
            roots[0].display()
        );
    }

    #[test]
    fn test_brave_editions_use_separate_directories() {
        let standard = brave_profile_roots(&BRAVE_STANDARD);
        let origin = brave_profile_roots(&BRAVE_ORIGIN);

        assert!(!standard.is_empty());
        assert!(!origin.is_empty());

        for root in &standard {
            assert!(!origin.contains(root));
        }
    }

    #[test]
    fn test_brave_origin_has_no_flatpak_or_snap_roots() {
        // Brave Origin ships only as a native package, so those paths would
        // never match anything
        let roots = brave_profile_roots(&BRAVE_ORIGIN);

        for root in &roots {
            let root = root.to_string_lossy();
            assert!(!root.contains(".var/app"), "unexpected flatpak root: {}", root);
            assert!(!root.contains("/snap/"), "unexpected snap root: {}", root);
        }
    }

    // Test that all strategies implement BrowserStrategy trait
    #[test]
    fn test_all_strategies_implement_browser_strategy() {
        let firefox: Box<dyn BrowserStrategy> = Box::new(FirefoxStrategy::new());
        let chrome: Box<dyn BrowserStrategy> = Box::new(ChromeStrategy::new());
        let safari: Box<dyn BrowserStrategy> = Box::new(SafariStrategy::new());
        let edge: Box<dyn BrowserStrategy> = Box::new(EdgeStrategy::new());
        let brave_standard: Box<dyn BrowserStrategy> = Box::new(BraveStandardStrategy::new());
        let brave_origin: Box<dyn BrowserStrategy> = Box::new(BraveOriginStrategy::new());

        assert_eq!(firefox.browser_name(), "firefox");
        assert_eq!(chrome.browser_name(), "chrome");
        assert_eq!(safari.browser_name(), "safari");
        assert_eq!(edge.browser_name(), "edge");
        assert_eq!(brave_standard.browser_name(), "brave-standard");
        assert_eq!(brave_origin.browser_name(), "brave-origin");
    }

    #[test]
    fn test_browser_type_strategy_names_match() {
        // Every browser type must build the strategy it names
        for browser_type in BrowserType::all() {
            let strategy = browser_type.strategy();
            assert!(
                browser_type.reports_as(strategy.browser_name()),
                "{} built a strategy named {}",
                browser_type,
                strategy.browser_name()
            );
        }
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
                    assert!(browser_type.reports_as(manager.browser_name()));
                }
                Err(BrowserError::BrowserNotAvailable { browser }) => {
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
                // Should be one of the browsers auto-detection checks for
                let browser_name = manager.browser_name();
                assert!(
                    BrowserType::detection_priority()
                        .iter()
                        .any(|browser_type| browser_type.reports_as(browser_name)),
                    "auto-detection returned unexpected browser: {}",
                    browser_name
                );
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
                assert!(browser_type.reports_as(manager.browser_name()));
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
        
        // Should be in the documented priority order
        let mut expected_order = Vec::new();
        for browser_type in BrowserType::detection_priority() {
            if browser_type.strategy().is_available() {
                expected_order.push(browser_type);
            }
        }

        assert_eq!(available_browsers, expected_order);
    }

    #[test]
    fn test_cookie_manager_with_fallback_preferred_available() {
        // Test fallback when preferred browser is available
        for browser_type in BrowserType::all() {
            if browser_type.strategy().is_available() {
                let result = CookieManager::with_fallback(Some(browser_type.clone()));
                match result {
                    Ok(manager) => {
                        assert!(browser_type.reports_as(manager.browser_name()));
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
        
        // Find a browser that's not available. 'brave' is an alias for whichever
        // edition is installed, so ask the strategy rather than the detection list.
        let unavailable_browser = all_browsers.iter().find(|&browser| !browser.strategy().is_available());

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
                        let browser_name = manager.browser_name();
                        // Should not be the unavailable browser
                        assert!(!unavailable_browser.reports_as(browser_name));
                        // Should be one of the available browsers
                        assert!(
                            available_browsers
                                .iter()
                                .any(|browser_type| browser_type.reports_as(browser_name)),
                            "fallback returned unexpected browser: {}",
                            browser_name
                        );
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

