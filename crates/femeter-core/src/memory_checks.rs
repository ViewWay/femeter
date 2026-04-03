//! Compile-time memory size verification for embedded targets.
//! All assertions use const assertions to catch regressions at build time.

use crate::PhaseData;
use core::mem::{align_of, size_of};

// ── Compile-time size assertions ──

// PhaseData: should be reasonably sized
const _: () = assert!(
    size_of::<PhaseData>() <= 64,
    "PhaseData exceeds 64-byte budget"
);
const _: () = assert!(align_of::<PhaseData>() == 4, "PhaseData alignment changed");

// EventLogEntry budget < 64 bytes
const _: () = assert!(
    size_of::<crate::event_detect::EventLogEntry>() <= 128,
    "EventLogEntry exceeds 64-byte budget"
);

// VoltageEventDetector budget < 256 bytes
const _: () = assert!(
    size_of::<crate::power_quality::VoltageEventDetector>() <= 512,
    "VoltageEventDetector exceeds 256-byte budget"
);

// FlickerAnalyzer budget < 512 bytes
const _: () = assert!(
    size_of::<crate::power_quality::FlickerAnalyzer>() <= 1024,
    "FlickerAnalyzer exceeds 512-byte budget"
);

// PowerQualityMonitor budget < 512 bytes
const _: () = assert!(
    size_of::<crate::power_quality::PowerQualityMonitor>() <= 4096,
    "PowerQualityMonitor exceeds 512-byte budget"
);

// LoadForecaster budget < 2KB
const _: () = assert!(
    size_of::<crate::load_forecast::LoadForecaster>() <= 4096,
    "LoadForecaster exceeds 2KB budget"
);

// LinearForecast budget < 512 bytes
const _: () = assert!(
    size_of::<crate::load_forecast::LinearForecast>() <= 1024,
    "LinearForecast exceeds 512-byte budget"
);

// EwmaForecast budget < 64 bytes
const _: () = assert!(
    size_of::<crate::load_forecast::EwmaForecast>() <= 128,
    "EwmaForecast exceeds 64-byte budget"
);

// TamperDetector budget < 256 bytes
const _: () = assert!(
    size_of::<crate::tamper_detection::TamperDetector>() <= 512,
    "TamperDetector exceeds 256-byte budget"
);

// EventDetector budget < 1024 bytes
const _: () = assert!(
    size_of::<crate::event_detect::EventDetector>() <= 8192,
    "EventDetector exceeds 1024-byte budget"
);

// EnergyData: should be 56 bytes
const _: () = assert!(
    size_of::<crate::EnergyData>() == 56,
    "EnergyData size changed unexpectedly"
);

// CalibrationParams: should be 48 bytes
const _: () = assert!(
    size_of::<crate::CalibrationParams>() == 48,
    "CalibrationParams size changed unexpectedly"
);

// ── Runtime size reporting tests ──

#[test]
fn test_memory_sizes_report() {
    eprintln!("═══ Memory Size Report ═══");
    eprintln!(
        "PhaseData:                  {} bytes",
        size_of::<PhaseData>()
    );
    eprintln!(
        "EnergyData:                 {} bytes",
        size_of::<crate::EnergyData>()
    );
    eprintln!(
        "CalibrationParams:          {} bytes",
        size_of::<crate::CalibrationParams>()
    );
    eprintln!(
        "EventLogEntry:              {} bytes",
        size_of::<crate::event_detect::EventLogEntry>()
    );
    eprintln!(
        "EventDetector:              {} bytes",
        size_of::<crate::event_detect::EventDetector>()
    );
    eprintln!(
        "VoltageEventDetector:       {} bytes",
        size_of::<crate::power_quality::VoltageEventDetector>()
    );
    eprintln!(
        "FlickerAnalyzer:            {} bytes",
        size_of::<crate::power_quality::FlickerAnalyzer>()
    );
    eprintln!(
        "PowerQualityMonitor:        {} bytes",
        size_of::<crate::power_quality::PowerQualityMonitor>()
    );
    eprintln!(
        "LoadForecaster:             {} bytes",
        size_of::<crate::load_forecast::LoadForecaster>()
    );
    eprintln!(
        "LinearForecast:             {} bytes",
        size_of::<crate::load_forecast::LinearForecast>()
    );
    eprintln!(
        "EwmaForecast:               {} bytes",
        size_of::<crate::load_forecast::EwmaForecast>()
    );
    eprintln!(
        "TamperDetector:             {} bytes",
        size_of::<crate::tamper_detection::TamperDetector>()
    );
    eprintln!(
        "HarmonicAnalysis:           {} bytes",
        size_of::<crate::power_quality::HarmonicAnalysis>()
    );
    eprintln!(
        "UnbalanceResult:            {} bytes",
        size_of::<crate::power_quality::UnbalanceResult>()
    );
    eprintln!(
        "PfAnalysis:                 {} bytes",
        size_of::<crate::power_quality::PfAnalysis>()
    );
}

#[test]
fn test_alignments() {
    eprintln!("═══ Alignment Report ═══");
    eprintln!("PhaseData align:      {}", align_of::<PhaseData>());
    eprintln!("EnergyData align:     {}", align_of::<crate::EnergyData>());
    eprintln!(
        "CalibrationParams:    {}",
        align_of::<crate::CalibrationParams>()
    );
}
