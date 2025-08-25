  # Implementation Plan

- [x] 1. Create browser abstraction layer and error types
  - Define the `BrowserStrategy` trait with methods for cookie fetching and availability checking
  - Implement `BrowserType` enum with string parsing capabilities
  - Create `BrowserError` enum with comprehensive error variants and user-friendly messages
  - Add unit tests for error type conversions and browser type parsing
  - _Requirements: 4.1, 4.2_

- [x] 2. Implement browser-specific strategies
- [x] 2.1 Create Firefox strategy implementation
  - Implement `FirefoxStrategy` struct that wraps existing `rookie::firefox()` functionality
  - Add availability checking by testing if Firefox profile directory exists
  - Write unit tests for Firefox cookie fetching and availability detection
  - _Requirements: 1.1, 3.1, 4.2_

- [x] 2.2 Create Chrome strategy implementation
  - Implement `ChromeStrategy` struct using `rookie::chrome()` function
  - Add Chrome-specific availability checking for cookie database access
  - Write unit tests for Chrome cookie fetching and error handling
  - _Requirements: 1.1, 1.2, 4.2_

- [x] 2.3 Create Safari strategy implementation
  - Implement `SafariStrategy` struct using `rookie::safari()` function
  - Add Safari-specific availability checking for macOS cookie store access
  - Write unit tests for Safari cookie fetching with platform-specific considerations
  - _Requirements: 2.1, 4.2_

- [x] 2.4 Create Edge strategy implementation
  - Implement `EdgeStrategy` struct using `rookie::edge()` function
  - Add Edge-specific availability checking for cookie database access
  - Write unit tests for Edge cookie fetching and availability detection
  - _Requirements: 2.1, 4.2_

- [x] 3. Implement cookie manager with browser selection logic
- [x] 3.1 Create CookieManager struct with strategy pattern
  - Implement `CookieManager` that holds a browser strategy instance
  - Add constructor methods for explicit browser selection and auto-detection
  - Implement cookie fetching method that delegates to the selected strategy
  - Write unit tests for manager initialization and strategy delegation
  - _Requirements: 2.2, 3.2, 4.1_

- [x] 3.2 Implement browser auto-detection logic
  - Create method to detect available browsers in priority order (Chrome, Firefox, Safari, Edge)
  - Implement fallback logic when preferred browsers are not available
  - Add comprehensive error handling for when no browsers are found
  - Write unit tests for auto-detection with various browser availability scenarios
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 4. Update CLI interface for browser selection
- [x] 4.1 Extend command-line argument parsing
  - Add `--browser` option to CLI struct with validation
  - Implement browser name validation and error messages for invalid options
  - Add help text showing available browser options
  - Write unit tests for CLI parsing with valid and invalid browser arguments
  - _Requirements: 2.1, 2.2_

- [x] 4.2 Update main application to use browser selection
  - Modify main function to parse browser argument and create appropriate CookieManager
  - Update error handling to display user-friendly browser-related error messages
  - Ensure backward compatibility when no browser is specified
  - Write integration tests for complete CLI-to-cookie-fetching flow
  - _Requirements: 2.3, 3.1_

- [x] 5. Refactor CookieJarWrapper to use new browser system
- [x] 5.1 Update CookieJarWrapper to accept CookieManager
  - Modify `CookieJarWrapper` constructor to accept a `CookieManager` instance
  - Replace hardcoded Firefox cookie fetching with manager-based fetching
  - Ensure existing cookie matching logic (`cookie_matches_url`) works with all browser sources
  - Write unit tests to verify cookie jar behavior with different browser strategies
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 5.2 Update cookie fetching in reqwest integration
  - Modify the `cookies` method to use the injected CookieManager
  - Ensure proper error handling when cookie fetching fails
  - Maintain the same cookie filtering and header formatting logic
  - Write integration tests for HTTP requests with cookies from different browsers
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 6. Add comprehensive error handling and user feedback
- [x] 6.1 Implement user-friendly error messages
  - Create helper functions to format browser-related error messages
  - Add suggestions for common error scenarios (browser not installed, no browsers found)
  - Ensure error messages include available browser options when relevant
  - Write unit tests for error message formatting and content
  - _Requirements: 1.3, 2.2, 3.3_

- [x] 6.2 Add logging and debugging support
  - Add debug logging for browser detection and selection process
  - Log cookie fetching attempts and results for troubleshooting
  - Ensure sensitive cookie data is not logged in production
  - Write tests to verify appropriate logging behavior
  - _Requirements: 4.2_

- [x] 7. Create integration tests for multi-browser support
- [x] 7.1 Write end-to-end tests for browser selection
  - Create tests that verify complete workflow from CLI argument to cookie usage
  - Test auto-detection behavior with different browser availability scenarios
  - Verify that existing cookie matching logic works consistently across browsers
  - Test error handling for various failure scenarios
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 7.2 Add backward compatibility tests
  - Verify that existing behavior is preserved when no browser is specified
  - Test that Firefox remains the default browser for backward compatibility
  - Ensure existing cookie functionality continues to work unchanged
  - Write regression tests to prevent breaking changes
  - _Requirements: 2.3_