# Requirements Document

## Introduction

This feature will refactor the existing Firefox-only cookie support to allow users to fetch cookies from multiple browsers including Chrome, Safari, Edge, and others. The current implementation is hardcoded to use Firefox cookies through the `rookie` crate, limiting users who primarily use other browsers. This enhancement will provide flexibility for users to specify their preferred browser or automatically detect available browsers.

## Requirements

### Requirement 1

**User Story:** As a user who uses Chrome as my primary browser, I want the application to fetch cookies from Chrome instead of Firefox, so that I can download files that require authentication from my Chrome sessions.

#### Acceptance Criteria

1. WHEN the user specifies Chrome as their browser THEN the system SHALL fetch cookies from Chrome's cookie store
2. WHEN Chrome cookies are available for a domain THEN the system SHALL use those cookies for HTTP requests to that domain
3. WHEN Chrome is not installed or accessible THEN the system SHALL provide a clear error message indicating Chrome cookies cannot be accessed

### Requirement 2

**User Story:** As a user with multiple browsers installed, I want to specify which browser to use for cookie fetching, so that I can control which browser's authentication state is used.

#### Acceptance Criteria

1. WHEN the user provides a browser selection option THEN the system SHALL accept browser names including "firefox", "chrome", "safari", "edge"
2. WHEN an invalid browser name is provided THEN the system SHALL display available browser options and exit with an error
3. WHEN no browser is specified THEN the system SHALL use a default browser preference order

### Requirement 3

**User Story:** As a user who wants convenience, I want the application to automatically detect and use available browsers, so that I don't have to manually specify which browser to use every time.

#### Acceptance Criteria

1. WHEN no browser is explicitly specified THEN the system SHALL attempt to detect available browsers in a predefined order
2. WHEN multiple browsers are available THEN the system SHALL use the first available browser from the preference order: Chrome, Firefox, Safari, Edge
3. WHEN no supported browsers are found THEN the system SHALL display an error message listing supported browsers

### Requirement 4

**User Story:** As a developer maintaining this application, I want the cookie fetching logic to be modular and extensible, so that adding support for new browsers in the future is straightforward.

#### Acceptance Criteria

1. WHEN adding support for a new browser THEN the system SHALL require minimal code changes outside of the browser-specific implementation
2. WHEN a browser-specific implementation fails THEN the system SHALL handle the error gracefully without affecting other browser support
3. WHEN the cookie fetching interface is used THEN it SHALL abstract away browser-specific details from the calling code

### Requirement 5

**User Story:** As a user downloading files, I want the same cookie matching and filtering logic to work regardless of which browser's cookies are used, so that authentication works consistently across browsers.

#### Acceptance Criteria

1. WHEN cookies are fetched from any supported browser THEN the system SHALL apply the same domain and path matching logic
2. WHEN cookies are filtered for a URL THEN the system SHALL use the existing `cookie_matches_url` function regardless of cookie source
3. WHEN cookies are formatted for HTTP headers THEN the system SHALL maintain the same format regardless of browser source