//! Performance benchmarks — using std::time::Instant.
//! All benchmarks assert timing targets.

#![allow(dead_code, unused_imports)]

use std::time::Instant;

const ITERATIONS: usize = 10_000;

macro_rules! bench {
    ($name:ident, $target_ms:expr, $body:expr) => {
        #[test]
        fn $name() {
            let start = Instant::now();
            for _ in 0..ITERATIONS {
                $body;
            }
            let elapsed = start.elapsed();
            let per_iter = elapsed / ITERATIONS as u32;
            eprintln!(
                "{}: total={:?}, per_iter={:?} (target<{:?})",
                stringify!($name),
                elapsed,
                per_iter,
                std::time::Duration::from_millis($target_ms)
            );
            assert!(
                per_iter.as_millis() <= $target_ms as u128,
                "{} exceeded target: {:?} > {:?}ms",
                stringify!($name),
                per_iter,
                $target_ms
            );
        }
    };
}

// ═══ HDLC frame encode/decode ═══

bench!(bench_hdlc_frame_roundtrip, 1, {
    let addr = dlms_hdlc::address::HdlcAddress::new(1, 16, 1);
    let ctrl = dlms_hdlc::control::ControlField::information(0, 0, false);
    let mut frame = dlms_hdlc::frame::HdlcFrame::new(addr, ctrl, vec![1, 2, 3, 4]);
    let bytes = frame.encode();
    let _ = dlms_hdlc::frame::HdlcFrame::decode(&bytes);
});

bench!(bench_hdlc_crc16_256, 1, {
    let data = [42u8; 256];
    let _ = dlms_hdlc::crc::crc16(&data);
});

bench!(bench_hdlc_address_encode_decode, 1, {
    let addr = dlms_hdlc::address::HdlcAddress::new(1, 16, 1);
    let bytes = dlms_hdlc::address::encode_address(&addr);
    let _ = dlms_hdlc::address::decode_address(&bytes);
});

bench!(bench_hdlc_llc_roundtrip, 1, {
    let payload = vec![
        0xE0, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0xFF, 0x02, 1, 2, 3,
    ];
    let wrapped = dlms_hdlc::llc::add_llc_header(true, &payload);
    let _ = dlms_hdlc::llc::strip_llc_header(&wrapped);
});

// ═══ AXDR encode/decode ═══

bench!(bench_axdr_encode_u8, 1, {
    let mut enc = dlms_axdr::AxdrEncoder::new();
    let val = dlms_axdr::DlmsType::Unsigned8(42);
    let _ = enc.encode(&val);
});

bench!(bench_axdr_encode_complex, 1, {
    let mut enc = dlms_axdr::AxdrEncoder::new();
    let _ = enc.encode(&dlms_axdr::DlmsType::Unsigned16(1000));
    let _ = enc.encode(&dlms_axdr::DlmsType::Unsigned32(100000));
    let _ = enc.encode(&dlms_axdr::DlmsType::OctetString(vec![1, 2, 3, 4, 5]));
    let _ = enc.encode(&dlms_axdr::DlmsType::Array(vec![
        dlms_axdr::DlmsType::Unsigned8(1),
        dlms_axdr::DlmsType::Unsigned8(2),
        dlms_axdr::DlmsType::Unsigned8(3),
    ]));
    let bytes = enc.to_bytes().to_vec();
    let mut dec = dlms_axdr::AxdrDecoder::new(&bytes);
    while dec.remaining() > 0 {
        if dec.decode().is_err() {
            break;
        }
    }
});

bench!(bench_axdr_datetime, 1, {
    let dt = dlms_axdr::datetime_codec::CosemDateTime {
        year: 2024,
        month: 6,
        day: 15,
        hour: 14,
        minute: 30,
        second: 0,
        hundredths: 0,
        deviation: 0,
        clock_status: 0,
    };
    let encoded = dlms_axdr::datetime_codec::encode_datetime(&dt);
    let _ = dlms_axdr::datetime_codec::decode_datetime(&encoded);
});

// ═══ ASN1 encode/decode ═══

bench!(bench_asn1_aarq_roundtrip, 1, {
    let aarq = dlms_asn1::aarq::Aarq::new_ln_no_cipher(dlms_asn1::aarq::InitiateRequest {
        proposed_conformance: dlms_asn1::conformance::ConformanceBlock::general(),
        proposed_quality_of_service: 0,
    });
    let bytes = aarq.encode();
    let _ = dlms_asn1::aarq::decode_aarq(&bytes);
});

// ═══ DLMS Security ═══

bench!(bench_security_key_generation, 1, {
    let _ = dlms_security::key::generate_key(42);
});

bench!(bench_security_constant_time_eq, 1, {
    let a = dlms_security::key::generate_key(1);
    let b = dlms_security::key::generate_key(1);
    let _ = dlms_security::key::constant_time_eq(&a, &b);
});

bench!(bench_security_control_parse, 1, {
    let _ = dlms_security::control::SecurityControl::from_byte(0x55);
});

// ═══ Femeter Core ═══

bench!(bench_metering_data_collection, 1, {
    let data = femeter_core::PhaseData {
        voltage_a: 22000,
        voltage_b: 22050,
        voltage_c: 21950,
        current_a: 5000,
        current_b: 4800,
        current_c: 5200,
        active_power_total: 3300,
        reactive_power_total: 200,
        apparent_power_total: 3306,
        frequency: 5000,
        power_factor_total: 998,
        ..Default::default()
    };
    let mut det = femeter_core::event_detect::EventDetector::new();
    det.check(&data);
});

bench!(bench_event_detection, 1, {
    let mut det = femeter_core::event_detect::EventDetector::new();
    let data = femeter_core::PhaseData {
        voltage_a: 22000,
        voltage_b: 22050,
        voltage_c: 21950,
        current_a: 5000,
        current_b: 4800,
        current_c: 5200,
        active_power_total: 3300,
        reactive_power_total: 200,
        frequency: 5000,
        power_factor_total: 998,
        ..Default::default()
    };
    det.check(&data);
});

bench!(bench_tamper_detection, 1, {
    let mut td = femeter_core::tamper_detection::TamperDetector::new(22000, 100);
    let data = femeter_core::PhaseData {
        voltage_a: 22000,
        voltage_b: 22050,
        voltage_c: 21950,
        current_a: 5000,
        current_b: 4800,
        current_c: 5200,
        frequency: 5000,
        power_factor_total: 998,
        ..Default::default()
    };
    let accel = femeter_core::tamper_detection::AccelerometerData::default();
    td.check(
        [data.voltage_a, data.voltage_b, data.voltage_c],
        [data.current_a, data.current_b, data.current_c],
        [
            data.voltage_angle_a,
            data.voltage_angle_b,
            data.voltage_angle_c,
        ],
        &accel,
        false,
        false,
        0,
    );
});

bench!(bench_power_quality_thd, 1, {
    let harmonics = [
        1.0, 0.05, 0.03, 0.02, 0.01, 0.008, 0.006, 0.005, 0.004, 0.003, 0.002, 0.002, 0.001, 0.001,
        0.001, 0.001, 0.001, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0,
    ];
    let _ = femeter_core::power_quality::calculate_thd(&harmonics, 1, 50);
});

bench!(bench_power_quality_harmonics_analysis, 1, {
    let harmonics = [
        1.0, 0.05, 0.03, 0.02, 0.01, 0.008, 0.006, 0.005, 0.004, 0.003, 0.002, 0.002, 0.001, 0.001,
        0.001, 0.001, 0.001, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0,
    ];
    let _ = femeter_core::power_quality::analyze_harmonics(&harmonics);
});

bench!(bench_load_forecast_update, 1, {
    let mut fc = femeter_core::load_forecast::LoadForecaster::new();
    let ctx = femeter_core::load_forecast::TimeContext::new(14, 2, 6);
    fc.update_with_context(50.0, &ctx);
});

bench!(bench_load_forecast_linear, 1, {
    let mut lf = femeter_core::load_forecast::LinearForecast::new();
    lf.push(50.0);
    lf.fit();
    let _ = lf.predict_next();
});

bench!(bench_load_forecast_ewma, 1, {
    let mut ewma = femeter_core::load_forecast::EwmaForecast::new(0.3);
    ewma.update(50.0);
    let _ = ewma.predict();
});

bench!(bench_voltage_unbalance, 1, {
    let _ = femeter_core::power_quality::calculate_unbalance([220.0, 221.0, 219.0]);
});

bench!(bench_power_factor_analysis, 1, {
    let _ = femeter_core::power_quality::analyze_power_factor([980, 975, 985, 978], 3300.0);
});
