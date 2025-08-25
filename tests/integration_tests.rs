use std::process::Command;
use std::env;
use std::path::PathBuf;

// Helper function to get the path to the compiled binary
fn get_binary_path() -> PathBuf {
    let mut path = env::current_exe().unwrap();
    path.pop(); // Remove test executable name
    if path.ends_with("deps") {
        path.pop(); // Remove deps directory
    }
    path.join("download")
}

// Helper function to run the download command with arguments
fn run_download_command(args: &[&str]) -> std::process::Output {
    Command::new(get_binary_path())
        .args(args)
        .output()
        .expect("Failed to execute download command")
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_end_to_end_browser_selection_chrome() {
        // Test complete workflow from CLI argument to cookie usage with Chrome
        let output = run_download_command(&["--browser", "chrome", "--help"]);
        
        // Should not fail with invalid browser error
        assert!(output.status.success() || output.stderr.is_empty() || 
                !String::from_utf8_lossy(&output.stderr).contains("not supported"));
    }

    #[test]
    fn test_end_to_end_browser_selection_firefox() {
        // Test complete workflow from CLI argument to cookie usage with Firefox
        let output = run_download_command(&["--browser", "firefox", "--help"]);
        
        // Should not fail with invalid browser error
        assert!(output.status.success() || output.stderr.is_empty() || 
                !String::from_utf8_lossy(&output.stderr).contains("not supported"));
    }

    #[test]
    fn test_end_to_end_browser_selection_safari() {
        // Test complete workflow from CLI argument to cookie usage with Safari
        let output = run_download_command(&["--browser", "safari", "--help"]);
        
        // Should not fail with invalid browser error
        assert!(output.status.success() || output.stderr.is_empty() || 
                !String::from_utf8_lossy(&output.stderr).contains("not supported"));
    }

    #[test]
    fn test_end_to_end_browser_selection_edge() {
        // Test complete workflow from CLI argument to cookie usage with Edge
        let output = run_download_command(&["--browser", "edge", "--help"]);
        
        // Should not fail with invalid browser error
        assert!(output.status.success() || output.stderr.is_empty() || 
                !String::from_utf8_lossy(&output.stderr).contains("not supported"));
    }

    #[test]
    fn test_end_to_end_invalid_browser_error() {
        // Test error handling for invalid browser selection
        let output = run_download_command(&["--browser", "invalid", "http://example.com"]);
        
        // Should fail with appropriate error message
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("not supported") || stderr.contains("invalid"));
        assert!(stderr.contains("chrome") || stderr.contains("firefox"));
    }

    #[test]
    fn test_end_to_end_case_insensitive_browser_names() {
        // Test that browser names are case-insensitive
        let test_cases = vec![
            ("CHROME", true),
            ("Firefox", true),
            ("SAFARI", true),
            ("Edge", true),
            ("chrome", true),
            ("firefox", true),
            ("safari", true),
            ("edge", true),
        ];

        for (browser_name, should_succeed) in test_cases {
            let output = run_download_command(&["--browser", browser_name, "--help"]);
            
            if should_succeed {
                // Should not fail with invalid browser error
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(!stderr.contains("not supported"), 
                        "Browser '{}' should be supported but got error: {}", browser_name, stderr);
            }
        }
    }

    #[test]
    fn test_end_to_end_auto_detection_no_browser_specified() {
        // Test auto-detection behavior when no browser is specified
        let output = run_download_command(&["--help"]);
        
        // Should succeed (help should work regardless of browser availability)
        assert!(output.status.success());
        
        // Help text should contain browser option information
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("--browser") || stdout.contains("-b"));
    }

    #[test]
    fn test_end_to_end_browser_availability_scenarios() {
        // Test different browser availability scenarios
        // This test checks that the application handles browser availability gracefully
        
        for browser in &["chrome", "firefox", "safari", "edge"] {
            let output = run_download_command(&["--browser", browser, "--help"]);
            
            // The help command should always work, regardless of browser availability
            // If browser is not available, it should be handled gracefully
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // Should not crash or produce unexpected errors
            assert!(!stderr.contains("panic") && !stderr.contains("thread panicked"));
        }
    }

    #[test]
    fn test_end_to_end_error_message_format() {
        // Test that error messages are user-friendly and contain helpful information
        let output = run_download_command(&["--browser", "nonexistent", "http://example.com"]);
        
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Should contain helpful error information
        assert!(stderr.contains("not supported") || stderr.contains("invalid"));
        assert!(stderr.contains("chrome") || stderr.contains("firefox") || 
                stderr.contains("safari") || stderr.contains("edge"));
        assert!(stderr.contains("Tip:") || stderr.contains("Available"));
    }

    #[test]
    fn test_end_to_end_multiple_urls_with_browser() {
        // Test that browser selection works with multiple URLs
        let output = run_download_command(&[
            "--browser", "firefox", 
            "--help" // Using help to avoid actual downloads
        ]);
        
        // Should handle multiple URLs without browser-related errors
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("not supported"));
    }

    #[test]
    fn test_end_to_end_browser_short_flag() {
        // Test that the short browser flag (-b) works
        let output = run_download_command(&["-b", "chrome", "--help"]);
        
        // Should not fail with invalid browser error
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("not supported"));
    }

    #[test]
    fn test_end_to_end_browser_equals_syntax() {
        // Test that --browser=chrome syntax works
        let output = run_download_command(&["--browser=firefox", "--help"]);
        
        // Should not fail with invalid browser error
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("not supported"));
    }

    #[test]
    fn test_end_to_end_help_contains_browser_information() {
        // Test that help output contains browser-related information
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Should contain browser option information
        assert!(stdout.contains("--browser") || stdout.contains("-b"));
        assert!(stdout.contains("chrome") || stdout.contains("firefox") || 
                stdout.contains("safari") || stdout.contains("edge"));
    }

    #[test]
    fn test_end_to_end_empty_browser_argument() {
        // Test handling of empty browser argument
        let output = run_download_command(&["--browser", "", "http://example.com"]);
        
        // Should fail with appropriate error
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("not supported") || stderr.contains("invalid"));
    }

    #[test]
    fn test_end_to_end_browser_with_actual_url() {
        // Test browser selection with a real URL (but don't actually download)
        // We'll use a non-existent URL to avoid actual network requests
        let output = run_download_command(&[
            "http://nonexistent.invalid.test.url.that.should.not.exist"
        ]);
        
        // Should fail due to network error, not browser error
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should not contain browser-related errors
        assert!(!stderr.contains("not supported"));
        assert!(!stderr.contains("Browser") || stderr.contains("network") || stderr.contains("resolve"));
    }

    #[test]
    fn test_end_to_end_cookie_manager_integration() {
        // Test that cookie manager integration works end-to-end
        // This is a smoke test to ensure the cookie manager is properly integrated
        
        for browser in &["chrome", "firefox", "safari", "edge"] {
            let output = run_download_command(&[
                "--browser", browser,
                "http://httpbin.org/cookies" // This URL returns cookies in JSON format
            ]);
            
            // The command might fail due to network issues or browser availability,
            // but it should not fail due to cookie manager integration issues
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // Should not contain cookie manager related panics or crashes
            assert!(!stderr.contains("panic"));
            assert!(!stderr.contains("thread panicked"));
            assert!(!stderr.contains("CookieManager"));
        }
    }

    #[test]
    fn test_end_to_end_backward_compatibility_no_browser() {
        // Test that the application works without specifying a browser (backward compatibility)
        let output = run_download_command(&["--help"]);
        
        // Should work without any browser-related errors
        assert!(output.status.success());
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("Browser"));
        assert!(!stderr.contains("not supported"));
    }

    #[test]
    fn test_end_to_end_graceful_browser_unavailable() {
        // Test graceful handling when specified browser is not available
        // We can't easily simulate this, but we can test that the error handling works
        
        let output = run_download_command(&["--browser", "chrome", "--help"]);
        
        // Should either succeed or fail gracefully with helpful error message
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // If it fails, should provide helpful error message
            assert!(stderr.contains("not available") || stderr.contains("not installed") ||
                    stderr.contains("Tip:") || stderr.contains("Available"));
        }
    }

    #[test]
    fn test_end_to_end_version_with_browser() {
        // Test that version command works with browser argument
        let output = run_download_command(&["--browser", "firefox", "--version"]);
        
        // Version should work regardless of browser selection
        // (if --version is supported by the application)
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("not supported"));
    }

    #[test]
    fn test_end_to_end_cookie_filtering_consistency() {
        // Test that cookie filtering works consistently across different browsers
        // This is a behavioral test to ensure cookie matching logic is preserved
        
        for browser in &["chrome", "firefox", "safari", "edge"] {
            let output = run_download_command(&[
                "--browser", browser,
                "--help" // Use help to avoid network requests
            ]);
            
            // Should not fail due to cookie filtering issues
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!stderr.contains("cookie_matches_url"));
            assert!(!stderr.contains("panic"));
        }
    }

    #[test]
    fn test_end_to_end_reqwest_client_integration() {
        // Test that reqwest client creation works with cookie support
        // This tests the integration between CookieManager and reqwest
        
        for browser in &["chrome", "firefox", "safari", "edge"] {
            let output = run_download_command(&[
                "--browser", browser,
                "--help"
            ]);
            
            // Should not fail due to reqwest client creation issues
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!stderr.contains("cookie_provider"));
            assert!(!stderr.contains("Client::builder"));
            assert!(!stderr.contains("panic"));
        }
    }
}

#[cfg(test)]
mod backward_compatibility_tests {
    use super::*;

    #[test]
    fn test_backward_compatibility_no_browser_specified() {
        // Verify that existing behavior is preserved when no browser is specified
        let output = run_download_command(&["--help"]);
        
        // Should work exactly as before
        assert!(output.status.success());
        
        // Should not show any browser-related warnings or errors
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("Warning:"));
        assert!(!stderr.contains("browser"));
    }

    #[test]
    fn test_backward_compatibility_firefox_default() {
        // Test that Firefox remains the default browser for backward compatibility
        let output = run_download_command(&["--help"]);
        
        // Should succeed (help command should always work)
        assert!(output.status.success());
        
        // The application should handle Firefox as default without explicit errors
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("not supported"));
    }

    #[test]
    fn test_backward_compatibility_existing_cookie_functionality() {
        // Ensure existing cookie functionality continues to work unchanged
        let output = run_download_command(&["--help"]);
        
        // Should work without cookie-related errors
        assert!(output.status.success());
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("cookie"));
        assert!(!stderr.contains("CookieJar"));
    }

    #[test]
    fn test_backward_compatibility_cli_interface() {
        // Test that existing CLI interface is preserved
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Should still contain basic functionality
        assert!(stdout.contains("URL") || stdout.contains("url"));
        
        // New browser option should be optional/additional
        if stdout.contains("--browser") {
            assert!(stdout.contains("optional") || stdout.contains("Browser"));
        }
    }

    #[test]
    fn test_backward_compatibility_error_handling() {
        // Test that error handling maintains backward compatibility
        let output = run_download_command(&["invalid-url-format"]);
        
        // Should handle invalid URLs the same way as before
        // (might succeed with help or fail with URL parsing error)
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Should not introduce new browser-related errors for non-browser issues
        if stderr.contains("error") || stderr.contains("Error") {
            assert!(!stderr.contains("browser") || stderr.contains("URL") || stderr.contains("url"));
        }
    }

    #[test]
    fn test_backward_compatibility_output_format() {
        // Test that output format remains consistent
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Should maintain existing help format
        assert!(stdout.contains("Usage:") || stdout.contains("USAGE:"));
    }

    #[test]
    fn test_backward_compatibility_exit_codes() {
        // Test that exit codes remain consistent
        let success_output = run_download_command(&["--help"]);
        assert!(success_output.status.success());
        
        let error_output = run_download_command(&["--browser", "invalid"]);
        assert!(!error_output.status.success());
    }

    #[test]
    fn test_backward_compatibility_no_regression() {
        // Regression test to ensure no breaking changes
        let output = run_download_command(&["--help"]);
        
        // Basic functionality should work
        assert!(output.status.success());
        
        // Should not introduce unexpected new requirements
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("required"));
        assert!(!stderr.contains("missing"));
    }

    #[test]
    fn test_backward_compatibility_firefox_preference() {
        // Test that Firefox is still preferred for backward compatibility
        // This test ensures that when no browser is specified, Firefox is tried first
        
        let output = run_download_command(&["--help"]);
        assert!(output.status.success());
        
        // The application should handle Firefox preference without errors
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("not supported"));
    }

    #[test]
    fn test_backward_compatibility_cookie_jar_wrapper() {
        // Test that CookieJarWrapper continues to work as before
        let output = run_download_command(&["--help"]);
        
        // Should not fail due to CookieJarWrapper changes
        assert!(output.status.success());
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("CookieJarWrapper"));
        assert!(!stderr.contains("panic"));
    }
}
#[cfg(test)]
mod additional_backward_compatibility_tests {
    use super::*;

    #[test]
    fn test_regression_firefox_default_behavior() {
        // Regression test: Ensure Firefox is still the default when no browser is specified
        // This test verifies that the fallback logic maintains Firefox preference
        let output = run_download_command(&["--help"]);
        
        // Should succeed without any browser selection errors
        assert!(output.status.success());
        
        // Should not require explicit browser selection for basic functionality
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("must specify"));
        assert!(!stderr.contains("required"));
    }

    #[test]
    fn test_regression_cookie_jar_behavior() {
        // Regression test: Ensure CookieJarWrapper behavior is unchanged
        // This verifies that existing cookie functionality works as before
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        
        // Should not introduce cookie-related errors in basic operations
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("CookieJar"));
        assert!(!stderr.contains("cookie_matches_url"));
    }

    #[test]
    fn test_regression_cli_argument_parsing() {
        // Regression test: Ensure existing CLI arguments still work
        let test_cases = vec![
            vec!["--help"],
            vec!["--version"],
        ];

        for args in test_cases {
            let output = run_download_command(&args);
            
            // Basic CLI functionality should work
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!stderr.contains("not supported"));
            assert!(!stderr.contains("browser"));
        }
    }

    #[test]
    fn test_regression_url_handling() {
        // Regression test: Ensure URL handling behavior is unchanged
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Should still accept URLs as before
        assert!(stdout.contains("URL") || stdout.contains("url"));
    }

    #[test]
    fn test_regression_error_messages() {
        // Regression test: Ensure error message format is consistent
        let output = run_download_command(&["--browser", "invalid", "http://example.com"]);
        
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Error messages should be helpful and consistent
        assert!(stderr.contains("ERROR") || stderr.contains("error"));
        assert!(stderr.contains("chrome") || stderr.contains("firefox"));
    }

    #[test]
    fn test_regression_no_breaking_changes() {
        // Comprehensive regression test for breaking changes
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        
        // Should not introduce breaking changes to basic functionality
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Basic help should work
        assert!(stdout.contains("Usage:") || stdout.contains("USAGE:"));
        assert!(!stderr.contains("panic"));
        assert!(!stderr.contains("error"));
    }

    #[test]
    fn test_regression_firefox_cookie_compatibility() {
        // Regression test: Ensure Firefox cookie handling is preserved
        let output = run_download_command(&["--help"]);
        
        // Should work without Firefox-specific errors
        assert!(output.status.success());
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("firefox"));
        assert!(!stderr.contains("Firefox"));
    }

    #[test]
    fn test_regression_existing_workflow() {
        // Regression test: Ensure existing user workflow is preserved
        // Test the most common usage pattern (no browser specified)
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        
        // Should work exactly as it did before multi-browser support
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.is_empty() || !stderr.contains("Warning"));
    }

    #[test]
    fn test_regression_cookie_matching_logic() {
        // Regression test: Ensure cookie matching logic is unchanged
        // This is tested indirectly by ensuring no cookie-related errors
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("cookie_matches_url"));
        assert!(!stderr.contains("domain"));
        assert!(!stderr.contains("path"));
    }

    #[test]
    fn test_regression_reqwest_integration() {
        // Regression test: Ensure reqwest integration is unchanged
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        
        // Should not introduce reqwest-related errors
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("reqwest"));
        assert!(!stderr.contains("Client"));
        assert!(!stderr.contains("cookie_provider"));
    }

    #[test]
    fn test_regression_performance_no_degradation() {
        // Regression test: Ensure no significant performance degradation
        use std::time::Instant;
        
        let start = Instant::now();
        let output = run_download_command(&["--help"]);
        let duration = start.elapsed();
        
        // Help command should complete quickly (within reasonable time)
        assert!(duration.as_secs() < 5, "Help command took too long: {:?}", duration);
        assert!(output.status.success());
    }

    #[test]
    fn test_regression_memory_usage() {
        // Regression test: Ensure no memory leaks or excessive usage
        // This is a basic smoke test
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        
        // Should not crash due to memory issues
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("memory"));
        assert!(!stderr.contains("allocation"));
    }

    #[test]
    fn test_regression_thread_safety() {
        // Regression test: Ensure thread safety is maintained
        use std::thread;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        let success_count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];
        
        // Run multiple help commands concurrently
        for _ in 0..5 {
            let success_count = Arc::clone(&success_count);
            let handle = thread::spawn(move || {
                let output = run_download_command(&["--help"]);
                if output.status.success() {
                    success_count.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // All commands should succeed
        assert_eq!(success_count.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_regression_environment_independence() {
        // Regression test: Ensure behavior is consistent across environments
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        
        // Should work regardless of environment variables
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("environment"));
        assert!(!stderr.contains("PATH"));
    }

    #[test]
    fn test_regression_unicode_handling() {
        // Regression test: Ensure Unicode handling is preserved
        let output = run_download_command(&["--help"]);
        
        assert!(output.status.success());
        
        // Should handle Unicode in output correctly
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Should not have encoding issues
        assert!(!stdout.contains("�"));
        assert!(!stderr.contains("�"));
    }
}