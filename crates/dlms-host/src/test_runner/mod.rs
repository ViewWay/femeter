//! Integration test runner for DLMS/COSEM smart meter testing
//!
//! This module provides a framework for running integration tests
//! against a real or simulated meter.

use std::time::Duration;
use std::io;
use std::net::TcpListener;
use std::thread;
use std::path::Path;

/// Test result
#[derive(Debug, Clone, PartialEq)]
pub enum TestResult {
    /// Test passed
    Passed,
    /// Test failed
    Failed,
    /// Test skipped
    Skipped,
    /// Test error
    Error(String),
}

/// Integration test case
#[derive(Debug, Clone)]
pub struct IntegrationTest {
    /// Test name
    pub name: String,
    /// Test description
    pub description: String,
    /// Test function
    pub test_fn: fn() -> TestResult,
}

/// Integration test runner
#[derive(Debug)]
pub struct TestRunner {
    /// Tests to run
    tests: Vec<IntegrationTest>,
    /// Continue on failure
    keep_going: bool,
    /// Verbose output
    verbose: bool,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new() -> Self {
        Self {
            tests: Vec::new(),
            keep_going: false,
            verbose: false,
        }
    }

    /// Add a test case
    pub fn add_test(&mut self, test: IntegrationTest) {
        self.tests.push(test);
    }

    /// Set continue-on-failure flag
    pub fn set_keep_going(&mut self, keep_going: bool) {
        self.keep_going = keep_going;
    }

    /// Set verbose flag
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    /// Run all tests
    pub fn run(&self) -> TestSummary {
        let mut summary = TestSummary::new();

        println!("Running {} tests...", self.tests.len());

        for test in &self.tests {
            if self.verbose {
                println!("Running: {}...", test.name);
            }

            let result = (test.test_fn)();
            let passed = result == TestResult::Passed;

            summary.add_result(test.name.clone(), result);

            if !passed {
                println!("  FAILED: {}", test.name);
                if !self.keep_going {
                    break;
                }
            } else if self.verbose {
                println!("  PASSED: {}", test.name);
            }
        }

        summary
    }

    /// Run tests matching a pattern
    pub fn run_pattern(&self, pattern: &str) -> TestSummary {
        let mut summary = TestSummary::new();
        let matched: Vec<_> = self.tests
            .iter()
            .filter(|t| t.name.contains(pattern))
            .collect();

        println!("Running {} tests matching '{}'...", matched.len(), pattern);

        for test in matched {
            let result = (test.test_fn)();
            summary.add_result(test.name.clone(), result);
        }

        summary
    }
}

/// Test execution summary
#[derive(Debug, Default)]
pub struct TestSummary {
    /// Number of passed tests
    pub passed: usize,
    /// Number of failed tests
    pub failed: usize,
    /// Number of skipped tests
    pub skipped: usize,
    /// Individual results
    pub results: Vec<(String, TestResult)>,
}

impl TestSummary {
    /// Create a new summary
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a test result
    pub fn add_result(&mut self, name: String, result: TestResult) {
        match result {
            TestResult::Passed => self.passed += 1,
            TestResult::Failed => self.failed += 1,
            TestResult::Skipped => self.skipped += 1,
            TestResult::Error(_) => self.failed += 1,
        }
        self.results.push((name, result));
    }

    /// Get total test count
    pub fn total(&self) -> usize {
        self.passed + self.failed + self.skipped
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Print summary to console
    pub fn print(&self) {
        println!("\n=== Test Summary ===");
        println!("Total: {}", self.total());
        println!("Passed: {}", self.passed);
        println!("Failed: {}", self.failed);
        println!("Skipped: {}", self.skipped);

        if self.all_passed() {
            println!("\n✓ All tests passed!");
        } else {
            println!("\n✗ Some tests failed!");
            for (name, result) in &self.results {
                if result != &TestResult::Passed {
                    println!("  - {}: {:?}", name, result);
                }
            }
        }
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Find available serial ports
#[cfg(unix)]
pub fn list_serial_ports() -> Vec<String> {
    let mut ports = Vec::new();

    // Common serial port locations on Unix-like systems
    for i in 0..10 {
        let path = format!("/dev/ttyUSB{}", i);
        if Path::new(&path).exists() {
            ports.push(path);
        }
        let path_cu = format!("/dev/cu.usbserial-{}", i);
        if Path::new(&path_cu).exists() {
            ports.push(path_cu);
        }
    }

    // Also check for /dev/tty.usbserial
    if let Ok(entries) = std::fs::read_dir("/dev") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("tty.usbserial") || name_str.starts_with("cu.usbserial") {
                ports.push(format!("/dev/{}", name_str));
            }
        }
    }

    ports.sort();
    ports.dedup();
    ports
}

/// Find an available TCP port
pub fn find_available_port() -> Option<u16> {
    for port in 4059u16..4100u16 {
        if TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok() {
            return Some(port);
        }
    }
    None
}

/// Sleep for specified milliseconds
pub fn sleep_ms(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_test() -> TestResult {
        TestResult::Passed
    }

    fn failing_test() -> TestResult {
        TestResult::Failed
    }

    #[test]
    fn test_runner_new() {
        let runner = TestRunner::new();
        assert_eq!(runner.tests.len(), 0);
    }

    #[test]
    fn test_runner_add_test() {
        let mut runner = TestRunner::new();
        let test = IntegrationTest {
            name: "dummy".to_string(),
            description: "dummy test".to_string(),
            test_fn: dummy_test,
        };
        runner.add_test(test);
        assert_eq!(runner.tests.len(), 1);
    }

    #[test]
    fn test_run() {
        let mut runner = TestRunner::new();
        runner.add_test(IntegrationTest {
            name: "dummy".to_string(),
            description: "dummy test".to_string(),
            test_fn: dummy_test,
        });

        let summary = runner.run();
        assert_eq!(summary.passed, 1);
        assert!(summary.all_passed());
    }

    #[test]
    fn test_summary() {
        let mut summary = TestSummary::new();
        summary.add_result("test1".to_string(), TestResult::Passed);
        summary.add_result("test2".to_string(), TestResult::Failed);

        assert_eq!(summary.total(), 2);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert!(!summary.all_passed());
    }

    #[test]
    fn test_find_available_port() {
        let port = find_available_port();
        assert!(port.is_some());
        assert!(port.unwrap() >= 4059);
    }
}
