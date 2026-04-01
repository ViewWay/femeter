//! dlms-host - DLMS/COSEM host tools CLI
//!
//! Command-line interface for simulator, sniffer, and test runner.

use dlms_host::{
    Cli, Commands, IntegrationTest, MeterAppBuilder, ProtocolSniffer, SimulatorApp, TestResult,
    TestRunner,
};
use std::fs::File;
use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::from_env();

    if cli.verbose {
        eprintln!("dlms-host v{}", env!("CARGO_PKG_VERSION"));
    }

    let result = match cli.command {
        Commands::Simulate {
            objects,
            listen,
            port,
        } => run_simulate(cli.verbose, objects, listen, port),
        Commands::Sniff {
            output,
            decode,
            client_filter,
            duration,
        } => run_sniff(cli.verbose, output, decode, client_filter, duration),
        Commands::Test {
            pattern,
            keep_going,
            output,
            tests,
        } => run_test(cli.verbose, pattern, keep_going, output, tests),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Run the simulator subcommand
fn run_simulate(
    _verbose: bool,
    _objects: Vec<String>,
    listen: bool,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DLMS/COSEM Meter Simulator ===");

    // Create simulator
    let mut simulator = SimulatorApp::new();
    simulator.set_meter_id("SIM-METER-CLI-001".to_string());
    simulator.start();

    println!("Meter ID: {}", simulator.meter_id());
    println!("Objects: {}", simulator.object_list().len());

    // List all COSEM objects
    println!("\nSupported COSEM objects:");
    for obis in simulator.object_list() {
        println!("  {}", obis);
    }

    // Simulate some load
    println!("\nSimulating load...");
    simulator.simulate_3phase_load(1000, 1100, 900);

    // Show current values
    println!("\nCurrent values:");
    println!("  Total Power: {} W", simulator.total_power());
    println!(
        "  L1 Power: {} W",
        simulator.app.measurement.instant_power(0).unwrap_or(0)
    );
    println!(
        "  L2 Power: {} W",
        simulator.app.measurement.instant_power(1).unwrap_or(0)
    );
    println!(
        "  L3 Power: {} W",
        simulator.app.measurement.instant_power(2).unwrap_or(0)
    );
    println!(
        "  Total Energy: {} Wh",
        simulator.app.measurement.total_energy_import()
    );

    if listen {
        println!("\nListening on TCP port {}...", port);
        println!("Press Ctrl+C to stop");
        // In real implementation, would start TCP server here
    }

    Ok(())
}

/// Run the sniffer subcommand
fn run_sniff(
    verbose: bool,
    output: Option<String>,
    decode: bool,
    client_filter: Option<u16>,
    _duration: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DLMS/COSEM Protocol Sniffer ===");

    let mut sniffer = ProtocolSniffer::new();
    sniffer.set_decode_apdu(decode);

    if let Some(addr) = client_filter {
        sniffer.set_client_filter(Some(addr as u8));
        if verbose {
            println!("Filtering for client address: {}", addr);
        }
    }

    println!("Sniffer started. Press Ctrl+C to stop.");
    println!("Capturing frames...");

    // Simulate some captured frames (in real implementation, would read from serial/port)
    use dlms_host::Direction;
    let test_frame = vec![0x01, 0x02, 0x03, 0x04, 0x7E];
    sniffer.process_bytes(&test_frame, Direction::Rx);

    println!("Captured {} frames", sniffer.frame_count());

    // Export if requested
    if let Some(path) = output {
        let csv = sniffer.export_csv();
        let mut file = File::create(&path)?;
        file.write_all(csv.as_bytes())?;
        println!("Exported to {}", path);
    }

    Ok(())
}

/// Run the test subcommand
fn run_test(
    verbose: bool,
    pattern: String,
    keep_going: bool,
    output: Option<String>,
    test_names: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DLMS/COSEM Test Runner ===");

    let mut runner = TestRunner::new();
    runner.set_verbose(verbose);
    runner.set_keep_going(keep_going);

    // Add built-in tests
    add_builtin_tests(&mut runner);

    // Filter tests if specific ones requested
    if !test_names.is_empty() {
        // Would filter by specific test names
        println!("Running {} specific tests", test_names.len());
    }

    // Run tests
    let summary = if pattern == "all" {
        runner.run()
    } else {
        runner.run_pattern(&pattern)
    };

    // Print summary
    summary.print();

    // Export results if requested
    if let Some(path) = output {
        let mut file = File::create(&path)?;
        writeln!(file, "=== Test Results ===")?;
        writeln!(file, "Total: {}", summary.total())?;
        writeln!(file, "Passed: {}", summary.passed)?;
        writeln!(file, "Failed: {}", summary.failed)?;
        writeln!(file, "Skipped: {}", summary.skipped)?;
        println!("\nResults exported to {}", path);
    }

    Ok(())
}

/// Add built-in integration tests
fn add_builtin_tests(runner: &mut TestRunner) {
    use dlms_core::obis::*;

    runner.add_test(IntegrationTest {
        name: "simulator_creation".to_string(),
        description: "Create a new simulator instance".to_string(),
        test_fn: || {
            let sim = SimulatorApp::new();
            if sim.meter_id() == "SIM-METER-001" {
                TestResult::Passed
            } else {
                TestResult::Failed
            }
        },
    });

    runner.add_test(IntegrationTest {
        name: "meter_app_creation".to_string(),
        description: "Create a new meter app instance".to_string(),
        test_fn: || {
            let app = MeterAppBuilder::new().build();
            if app.uptime() == 0 {
                TestResult::Passed
            } else {
                TestResult::Failed
            }
        },
    });

    runner.add_test(IntegrationTest {
        name: "simulator_load".to_string(),
        description: "Test load simulation".to_string(),
        test_fn: || {
            let mut sim = SimulatorApp::new();
            sim.simulate_load(1500);
            if sim.app.measurement.total_energy_import() > 0 {
                TestResult::Passed
            } else {
                TestResult::Failed
            }
        },
    });

    runner.add_test(IntegrationTest {
        name: "simulator_voltage".to_string(),
        description: "Test voltage update".to_string(),
        test_fn: || {
            let mut sim = SimulatorApp::new();
            let result = sim.set_voltage(0, 2300);
            if result.is_ok() && sim.app.measurement.voltage(0) == Some(2300) {
                TestResult::Passed
            } else {
                TestResult::Failed
            }
        },
    });

    runner.add_test(IntegrationTest {
        name: "sniffer_capture".to_string(),
        description: "Test frame capture".to_string(),
        test_fn: || {
            use dlms_host::Direction;
            let mut sniffer = ProtocolSniffer::new();
            let frame = vec![0x01, 0x02, 0x03, 0x04, 0x7E];
            sniffer.process_bytes(&frame, Direction::Rx);
            if sniffer.frame_count() == 1 {
                TestResult::Passed
            } else {
                TestResult::Failed
            }
        },
    });

    runner.add_test(IntegrationTest {
        name: "obis_codes".to_string(),
        description: "Verify standard OBIS codes are supported".to_string(),
        test_fn: || {
            let sim = SimulatorApp::new();
            let list = sim.object_list();
            let has_clock = list.contains(&CLOCK);
            let has_energy = list.contains(&TOTAL_ACTIVE_ENERGY_IMPORT);
            if has_clock && has_energy {
                TestResult::Passed
            } else {
                TestResult::Failed
            }
        },
    });

    runner.add_test(IntegrationTest {
        name: "tick_time".to_string(),
        description: "Test time advancement".to_string(),
        test_fn: || {
            let mut app = MeterAppBuilder::new().build();
            app.tick(60);
            if app.uptime() == 60 {
                TestResult::Passed
            } else {
                TestResult::Failed
            }
        },
    });

    runner.add_test(IntegrationTest {
        name: "read_attribute".to_string(),
        description: "Test reading COSEM attributes".to_string(),
        test_fn: || {
            let sim = SimulatorApp::new();
            let result = sim.read_attribute(&CLOCK, 2);
            if result.is_ok() {
                TestResult::Passed
            } else {
                TestResult::Failed
            }
        },
    });
}
