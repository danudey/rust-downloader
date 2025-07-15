# Design Document

## Overview

This design refactors the cookie fetching system from a Firefox-only implementation to a multi-browser architecture. The solution introduces a browser abstraction layer that can fetch cookies from Chrome, Firefox, Safari, and Edge using the `rookie` crate's browser-specific functions. The design maintains backward compatibility while adding flexibility for browser selection through command-line arguments or automatic detection.

## Architecture

The refactored architecture follows a strategy pattern where different browser implementations are encapsulated behind a common interface:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Application   │───▶│  CookieManager   │───▶│ BrowserStrategy │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         │
                        ┌────────────────────────────────┼────────────────────────────────┐
                        │                                │                                │
                        ▼                                ▼                                ▼
                ┌──────────────┐                ┌──────────────┐                ┌──────────────┐
                │   Firefox    │                │    Chrome    │                │   Safari     │
                │   Strategy   │                │   Strategy   │                │   Strategy   │
                └──────────────┘                └──────────────┘                └──────────────┘
```

## Components and Interfaces

### BrowserStrategy Trait

A trait that defines the interface for browser-specific cookie fetching:

```rust
trait BrowserStrategy {
    fn fetch_cookies(&self, domains: Vec<String>) -> Result<Vec<Cookie>, BrowserError>;
    fn is_available(&self) -> bool;
    fn browser_name(&self) -> &'static str;
}
```

### Browser Implementations

Concrete implementations for each supported browser:

- `FirefoxStrategy`: Uses `rookie::firefox()`
- `ChromeStrategy`: Uses `rookie::chrome()`  
- `SafariStrategy`: Uses `rookie::safari()`
- `EdgeStrategy`: Uses `rookie::edge()`

### CookieManager

A manager class that handles browser selection and cookie fetching:

```rust
struct CookieManager {
    strategy: Box<dyn BrowserStrategy>,
}

impl CookieManager {
    fn new(browser_preference: Option<BrowserType>) -> Result<Self, BrowserError>
    fn with_auto_detection() -> Result<Self, BrowserError>
    fn fetch_cookies_for_domain(&self, domain: String) -> Result<Vec<Cookie>, BrowserError>
}
```

### Browser Selection Logic

The browser selection follows this priority:

1. **Explicit Selection**: If user specifies `--browser chrome`, use Chrome
2. **Auto Detection**: Try browsers in order: Chrome → Firefox → Safari → Edge
3. **Fallback**: Error if no browsers are available

### Command Line Interface

Extend the existing CLI with browser selection:

```rust
#[derive(Parser)]
struct Cli {
    /// The URL to download from
    urls: Vec<String>,
    
    /// Browser to use for cookies (chrome, firefox, safari, edge)
    #[arg(long, short)]
    browser: Option<String>,
}
```

## Data Models

### BrowserType Enum

```rust
#[derive(Debug, Clone, PartialEq)]
enum BrowserType {
    Chrome,
    Firefox,
    Safari,
    Edge,
}

impl FromStr for BrowserType {
    type Err = BrowserError;
    // Implementation for parsing from string
}
```

### BrowserError Enum

```rust
#[derive(Debug, thiserror::Error)]
enum BrowserError {
    #[error("Browser {0} is not supported")]
    UnsupportedBrowser(String),
    
    #[error("Browser {0} is not available or installed")]
    BrowserNotAvailable(String),
    
    #[error("No supported browsers found")]
    NoBrowsersAvailable,
    
    #[error("Failed to fetch cookies: {0}")]
    CookieFetchError(String),
}
```

## Error Handling

The design implements comprehensive error handling:

1. **Browser Detection Errors**: When a specified browser isn't available
2. **Cookie Fetch Errors**: When cookie retrieval fails for any reason
3. **Graceful Degradation**: Try next browser in auto-detection mode
4. **User-Friendly Messages**: Clear error messages with available options

Error handling strategy:
- Log detailed errors for debugging
- Display user-friendly messages for common scenarios
- Provide actionable suggestions (e.g., "Available browsers: chrome, firefox")

## Testing Strategy

### Unit Tests

1. **Browser Strategy Tests**: Test each browser implementation independently
2. **Cookie Manager Tests**: Test browser selection logic and fallback behavior
3. **Error Handling Tests**: Verify proper error propagation and messages
4. **CLI Parsing Tests**: Test command-line argument parsing for browser selection

### Integration Tests

1. **End-to-End Cookie Fetching**: Test complete flow from CLI to cookie retrieval
2. **Browser Availability Tests**: Test behavior when browsers are/aren't installed
3. **Cookie Matching Tests**: Ensure existing cookie matching logic works with all browsers

### Test Doubles

- Mock browser strategies for testing manager logic
- Fake cookie data for testing cookie matching
- Simulated browser availability states

## Migration Strategy

The refactoring maintains backward compatibility:

1. **Default Behavior**: If no browser is specified, maintain current Firefox preference
2. **Gradual Rollout**: Existing functionality continues to work unchanged
3. **Optional Features**: New browser support is additive, not replacing

## Performance Considerations

- **Lazy Loading**: Browser strategies are only initialized when needed
- **Caching**: Browser availability detection results can be cached
- **Minimal Overhead**: Browser abstraction adds minimal performance cost
- **Memory Usage**: Only one browser strategy is active at a time