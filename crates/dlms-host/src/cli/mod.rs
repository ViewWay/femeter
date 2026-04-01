//! CLI argument parsing for dlms-host tools
//!
//! This module provides command-line interface parsing using clap.

pub use clap::{Parser, Subcommand};

/// dlms-host command-line interface
#[derive(Debug, Clone, Parser)]
#[command(name = "dlms-host")]
#[command(about = "DLMS/COSEM host tools - simulator, sniffer, test runner", long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Serial port device (for simulator/sniffer)
    #[arg(short = 'p', long)]
    pub port: Option<String>,

    /// Baud rate (default: 9600)
    #[arg(short = 'b', long, default_value = "9600")]
    pub baud: u32,

    /// Client address (HDLC)
    #[arg(short = 'c', long, default_value = "1")]
    pub client_address: u16,

    /// Server address (HDLC)
    #[arg(short = 's', long, default_value = "1")]
    pub server_address: u16,

    /// Logical device ID
    #[arg(short = 'l', long, default_value = "1")]
    pub logical_device: u16,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands
#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    /// Simulate a DLMS smart meter
    Simulate {
        /// OBIS code to expose (can be specified multiple times)
        #[arg(short = 'o', long)]
        objects: Vec<String>,

        /// Listen for incoming connections
        #[arg(short = 'l', long)]
        listen: bool,

        /// Port number for TCP listening
        #[arg(short = 'p', long, default_value = "4059")]
        port: u16,
    },

    /// Sniff DLMS protocol frames
    Sniff {
        /// Output file for captured frames
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Decode APDU contents
        #[arg(short = 'd', long)]
        decode: bool,

        /// Filter by client address
        #[arg(short = 'c', long)]
        client_filter: Option<u16>,

        /// Capture duration in seconds (0 = infinite)
        #[arg(short = 't', long, default_value = "0")]
        duration: u32,
    },

    /// Run integration tests
    Test {
        /// Test pattern (e.g., "read", "write", "all")
        #[arg(short = 'p', long, default_value = "all")]
        pattern: String,

        /// Continue on failure
        #[arg(short = 'k', long)]
        keep_going: bool,

        /// Output test results to file
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Specific test(s) to run
        #[arg(name = "TESTS")]
        tests: Vec<String>,
    },
}

impl Cli {
    /// Parse command-line arguments from environment
    #[cfg(feature = "std")]
    pub fn from_env() -> Self {
        Self::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulate_command() {
        let cli = Cli::parse_from(&["dlms-host", "simulate", "--listen"]);
        assert!(matches!(cli.command, Commands::Simulate { .. }));
    }

    #[test]
    fn test_sniff_command() {
        let cli = Cli::parse_from(&["dlms-host", "sniff", "--decode"]);
        assert!(matches!(cli.command, Commands::Sniff { .. }));
    }

    #[test]
    fn test_test_command() {
        let cli = Cli::parse_from(&["dlms-host", "test", "--pattern", "read"]);
        assert!(matches!(cli.command, Commands::Test { .. }));
    }

    #[test]
    fn test_verbose_flag() {
        let cli = Cli::parse_from(&["dlms-host", "-v", "test"]);
        assert!(cli.verbose);
    }
}
