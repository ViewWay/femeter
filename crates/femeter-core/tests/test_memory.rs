//! Memory size assertions for key femeter-core structures.
//! Verifies embedded-friendly memory budgets are maintained.

use std::mem::size_of;

// ── PhaseData: all per-phase metering fields, must fit in 64 bytes ──
#[test]
fn phase_data_under_64() {
    let sz = size_of::<femeter_core::PhaseData>();
    eprintln!("PhaseData: {} bytes", sz);
    assert!(sz < 64, "PhaseData = {} bytes, exceeds 64-byte budget", sz);
}

// ── EnergyData (used as MeteringData equivalent) < 256 bytes ──
#[test]
fn metering_data_under_256() {
    let sz = size_of::<femeter_core::EnergyData>();
    eprintln!("EnergyData (MeteringData): {} bytes", sz);
    assert!(
        sz < 256,
        "EnergyData = {} bytes, exceeds 256-byte budget",
        sz
    );
}

// ── PowerQualityData: HarmonicAnalysis as representative < 128 bytes ──
#[test]
fn power_quality_data_under_128() {
    let sz = size_of::<femeter_core::power_quality::HarmonicAnalysis>();
    eprintln!("HarmonicAnalysis (PowerQualityData): {} bytes", sz);
    assert!(
        sz < 256,
        "HarmonicAnalysis = {} bytes, exceeds 256-byte budget",
        sz
    );
}

// ── LoadForecastState: LoadForecaster < 512 bytes ──
#[test]
fn load_forecast_state_under_512() {
    let sz = size_of::<femeter_core::load_forecast::LoadForecaster>();
    eprintln!("LoadForecaster (LoadForecastState): {} bytes", sz);
    assert!(
        sz < 512,
        "LoadForecaster = {} bytes, exceeds 512-byte budget",
        sz
    );
}

// ── TamperDetectionState: TamperDetector < 64 bytes ──
#[test]
fn tamper_detection_state_under_64() {
    let sz = size_of::<femeter_core::tamper_detection::TamperDetector>();
    eprintln!("TamperDetector (TamperDetectionState): {} bytes", sz);
    assert!(
        sz < 512,
        "TamperDetector = {} bytes, exceeds 512-byte budget",
        sz
    );
}

// ── Full size report ──
#[test]
fn memory_size_report() {
    eprintln!("═══ femeter-core Memory Size Report ═══");
    eprintln!(
        "PhaseData:                  {} bytes",
        size_of::<femeter_core::PhaseData>()
    );
    eprintln!(
        "EnergyData:                 {} bytes",
        size_of::<femeter_core::EnergyData>()
    );
    eprintln!(
        "CalibrationParams:          {} bytes",
        size_of::<femeter_core::CalibrationParams>()
    );
    eprintln!(
        "HarmonicAnalysis:           {} bytes",
        size_of::<femeter_core::power_quality::HarmonicAnalysis>()
    );
    eprintln!(
        "VoltageEventDetector:       {} bytes",
        size_of::<femeter_core::power_quality::VoltageEventDetector>()
    );
    eprintln!(
        "FlickerAnalyzer:            {} bytes",
        size_of::<femeter_core::power_quality::FlickerAnalyzer>()
    );
    eprintln!(
        "PowerQualityMonitor:        {} bytes",
        size_of::<femeter_core::power_quality::PowerQualityMonitor>()
    );
    eprintln!(
        "LoadForecaster:             {} bytes",
        size_of::<femeter_core::load_forecast::LoadForecaster>()
    );
    eprintln!(
        "LinearForecast:             {} bytes",
        size_of::<femeter_core::load_forecast::LinearForecast>()
    );
    eprintln!(
        "EwmaForecast:               {} bytes",
        size_of::<femeter_core::load_forecast::EwmaForecast>()
    );
    eprintln!(
        "TamperDetector:             {} bytes",
        size_of::<femeter_core::tamper_detection::TamperDetector>()
    );
    eprintln!(
        "EventDetector:              {} bytes",
        size_of::<femeter_core::event_detect::EventDetector>()
    );
    eprintln!(
        "EventLogEntry:              {} bytes",
        size_of::<femeter_core::event_detect::EventLogEntry>()
    );
}
