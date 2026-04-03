//! Fuzz tests — randomized boundary testing for protocol parsers.
//! Each target runs 1000+ random inputs; no panic allowed on malformed data.

#![allow(dead_code, unused_imports)]

use std::time::Instant;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next_u8(&mut self) -> u8 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u8
    }
    fn next_u16(&mut self) -> u16 {
        (self.next_u8() as u16) << 8 | self.next_u8() as u16
    }
    fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

fn rand_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut rng = Rng(seed);
    let mut buf = vec![0u8; len];
    rng.fill(&mut buf);
    buf
}

fn rand_bytes_range(seed: u64, min: usize, max: usize) -> Vec<u8> {
    rand_bytes(seed, min + (seed as usize % (max - min + 1)))
}

// ═══ HDLC ═══

#[test]
fn fuzz_hdlc_frame_decode() {
    let start = Instant::now();
    for i in 0..1500u64 {
        let data = rand_bytes_range(i, 0, 256);
        let _ = dlms_hdlc::frame::HdlcFrame::decode(&data);
    }
    eprintln!("hdlc frame decode: 1500 cases in {:?}", start.elapsed());
}

#[test]
fn fuzz_hdlc_crc() {
    let start = Instant::now();
    for i in 0..2000u64 {
        let data = rand_bytes_range(i, 0, 512);
        let _ = dlms_hdlc::crc::crc16(&data);
        let _ = dlms_hdlc::crc::verify_crc(&data);
    }
    eprintln!("hdlc crc: 2000 cases in {:?}", start.elapsed());
}

#[test]
fn fuzz_hdlc_address() {
    let start = Instant::now();
    for i in 0..1500u64 {
        let data = rand_bytes_range(i, 1, 16);
        let _ = dlms_hdlc::address::decode_address(&data);
    }
    eprintln!("hdlc address: 1500 cases in {:?}", start.elapsed());
}

#[test]
fn fuzz_hdlc_control() {
    for i in 0..=255u8 {
        let _ = dlms_hdlc::control::ControlField::decode(i);
    }
}

#[test]
fn fuzz_hdlc_llc() {
    for i in 0..1000u64 {
        let data = rand_bytes_range(i, 0, 300);
        let _ = dlms_hdlc::llc::strip_llc_header(&data);
    }
}

#[test]
fn fuzz_hdlc_segment() {
    for i in 0..1000u64 {
        let data = rand_bytes_range(i, 0, 1024);
        let addr = dlms_hdlc::address::HdlcAddress::new(1, 16, 1);
        let _ = dlms_hdlc::segment::segment_payload(&addr, &data, 50, 0, 0);
    }
}

#[test]
fn fuzz_hdlc_config() {
    for i in 0..1000u64 {
        let data = rand_bytes_range(i, 0, 64);
        let _ = dlms_hdlc::config::HdlcConfig::parse_ua_payload(&data);
    }
}

// ═══ AXDR ═══

#[test]
fn fuzz_axdr_decode() {
    let start = Instant::now();
    for i in 0..2000u64 {
        let data = rand_bytes_range(i, 0, 512);
        let mut dec = dlms_axdr::AxdrDecoder::new(&data);
        while dec.remaining() > 0 {
            if dec.decode().is_err() {
                break;
            }
        }
    }
    eprintln!("axdr decode: 2000 cases in {:?}", start.elapsed());
}

#[test]
fn fuzz_axdr_datetime() {
    for i in 0..1000u64 {
        let mut buf = [0u8; 12];
        Rng(i).fill(&mut buf);
        let _ = dlms_axdr::datetime_codec::decode_date(&buf[..5]);
        let _ = dlms_axdr::datetime_codec::decode_time(&buf[..4]);
        let _ = dlms_axdr::datetime_codec::decode_datetime(&buf);
    }
}

#[test]
fn fuzz_axdr_compact() {
    for i in 0..1000u64 {
        let _ = dlms_axdr::CompactArrayCodec::element_size((i % 256) as u8);
    }
}

// ═══ ASN1 ═══

#[test]
fn fuzz_asn1_aarq_decode() {
    let start = Instant::now();
    for i in 0..1500u64 {
        let data = rand_bytes_range(i, 0, 256);
        let _ = dlms_asn1::decode_aarq(&data);
        let _ = dlms_asn1::decode_aare(&data);
    }
    eprintln!("asn1 aarq/aare: 1500 cases in {:?}", start.elapsed());
}

#[test]
fn fuzz_asn1_rlrq_rlre() {
    for i in 0..1000u64 {
        let data = rand_bytes_range(i, 0, 64);
        let _ = dlms_asn1::decode_rlre(&data);
    }
}

// ═══ OBIS ═══

#[test]
fn fuzz_obis_parse() {
    let start = Instant::now();
    for i in 0..1000u64 {
        let mut rng = Rng(i + 600000);
        let parts: Vec<String> = (0..6)
            .map(|_| {
                let b = rng.next_u8();
                match b % 4 {
                    0 => (rng.next_u16() % 300).to_string(),
                    1 => String::from_utf8(vec![b'a' + (rng.next_u8() % 26)]).unwrap_or_default(),
                    2 => String::new(),
                    _ => ".".to_string(),
                }
            })
            .collect();
        let s = parts.join(".");
        let _ = dlms_obis::parser::parse_obis(&s);
    }
    eprintln!("obis parse: 1000 cases in {:?}", start.elapsed());
}

// ═══ Femeter Core ═══

#[test]
fn fuzz_event_detect() {
    let start = Instant::now();
    for i in 0..2000u64 {
        let mut rng = Rng(i);
        let data = femeter_core::PhaseData {
            voltage_a: rng.next_u16(),
            voltage_b: rng.next_u16(),
            voltage_c: rng.next_u16(),
            current_a: rng.next_u16(),
            current_b: rng.next_u16(),
            current_c: rng.next_u16(),
            active_power_total: rng.next_u16() as i32,
            reactive_power_total: rng.next_u16() as i32,
            apparent_power_total: rng.next_u16() as i32,
            frequency: rng.next_u16(),
            power_factor_total: rng.next_u16(),
            ..Default::default()
        };
        let mut det = femeter_core::event_detect::EventDetector::new();
        let _ = det.check(&data);
    }
    eprintln!("event detect: 2000 cases in {:?}", start.elapsed());
}

#[test]
fn fuzz_tamper_detection() {
    for i in 0..2000u64 {
        let mut rng = Rng(i + 50000);
        let data = femeter_core::PhaseData {
            voltage_a: rng.next_u16(),
            voltage_b: rng.next_u16(),
            voltage_c: rng.next_u16(),
            current_a: rng.next_u16(),
            current_b: rng.next_u16(),
            current_c: rng.next_u16(),
            frequency: rng.next_u16(),
            power_factor_total: rng.next_u16(),
            ..Default::default()
        };
        let mut td = femeter_core::tamper_detection::TamperDetector::new(22000, 100);
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
    }
}

#[test]
fn fuzz_power_quality() {
    for i in 0..1500u64 {
        let mut rng = Rng(i + 100000);
        let mut harmonics = [0.0f32; 50];
        for h in harmonics.iter_mut() {
            *h = (rng.next_u16() as f32) / 65535.0;
        }
        let _ = femeter_core::power_quality::calculate_thd(&harmonics, 1, 50);
        let _ = femeter_core::power_quality::analyze_harmonics(&harmonics);
    }
}

#[test]
fn fuzz_load_forecast() {
    for i in 0..1500u64 {
        let mut rng = Rng(i + 200000);
        let mut fc = femeter_core::load_forecast::LoadForecaster::new();
        for _ in 0..48 {
            let load = (rng.next_u16() as f32) / 10.0;
            let ctx = femeter_core::load_forecast::TimeContext::new(
                rng.next_u8() % 24,
                rng.next_u8() % 7,
                (rng.next_u8() % 12) + 1,
            );
            fc.update_with_context(load, &ctx);
        }
        let _ = fc.update(50.0);
    }
}

#[test]
fn fuzz_voltage_event_detector() {
    for i in 0..1000u64 {
        let mut rng = Rng(i + 300000);
        let mut det = femeter_core::power_quality::VoltageEventDetector::new(22000);
        let _ = det.check([rng.next_u16(), rng.next_u16(), rng.next_u16()], i as u32);
    }
}

#[test]
fn fuzz_flicker_analyzer() {
    for i in 0..1000u64 {
        let mut rng = Rng(i + 400000);
        let mut fa = femeter_core::power_quality::FlickerAnalyzer::new();
        for _ in 0..100 {
            fa.feed_half_cycle_rms((rng.next_u16() as f32) / 65535.0);
        }
        let _ = fa.instantaneous_flicker();
    }
}

#[test]
fn fuzz_security_control() {
    for i in 0..=255u8 {
        let _ = dlms_security::SecurityControl::from_byte(i);
        let _ = dlms_security::SecuritySuite::from_bits(i);
    }
}

#[test]
fn fuzz_key_operations() {
    for i in 0..1000u64 {
        let _ = dlms_security::generate_key(i as u32);
    }
}

#[test]
fn fuzz_unbalance() {
    for i in 0..1000u64 {
        let mut rng = Rng(i + 500000);
        let vals = [
            rng.next_u16() as f32,
            rng.next_u16() as f32,
            rng.next_u16() as f32,
        ];
        let _ = femeter_core::power_quality::calculate_unbalance(vals);
    }
}

// ═══ Edge cases ═══

#[test]
fn fuzz_edge_all_zeros() {
    let zeros = vec![0u8; 512];
    let _ = dlms_hdlc::frame::HdlcFrame::decode(&zeros);
    let _ = dlms_hdlc::crc::crc16(&zeros);
    let mut dec = dlms_axdr::AxdrDecoder::new(&zeros);
    while dec.remaining() > 0 {
        if dec.decode().is_err() {
            break;
        }
    }
    let _ = dlms_asn1::decode_aarq(&zeros);
}

#[test]
fn fuzz_edge_all_ones() {
    let ones = vec![0xFFu8; 512];
    let _ = dlms_hdlc::frame::HdlcFrame::decode(&ones);
    let _ = dlms_hdlc::crc::crc16(&ones);
    let mut dec = dlms_axdr::AxdrDecoder::new(&ones);
    while dec.remaining() > 0 {
        if dec.decode().is_err() {
            break;
        }
    }
    let _ = dlms_asn1::decode_aarq(&ones);
}

#[test]
fn fuzz_edge_empty() {
    let empty: Vec<u8> = vec![];
    let _ = dlms_hdlc::frame::HdlcFrame::decode(&empty);
    let _ = dlms_hdlc::crc::crc16(&empty);
    let mut dec = dlms_axdr::AxdrDecoder::new(&empty);
    let _ = dec.decode();
    let _ = dlms_asn1::decode_aarq(&empty);
}

#[test]
fn fuzz_edge_single_byte() {
    for b in 0..=255u8 {
        let data = vec![b];
        let _ = dlms_hdlc::frame::HdlcFrame::decode(&data);
        let mut dec = dlms_axdr::AxdrDecoder::new(&data);
        let _ = dec.decode();
        let _ = dlms_asn1::decode_aarq(&data);
    }
}

#[test]
fn fuzz_edge_flag_flood() {
    let flags = vec![0x7Eu8; 512];
    let _ = dlms_hdlc::frame::HdlcFrame::decode(&flags);
}

#[test]
fn fuzz_edge_escape_flood() {
    let escapes = vec![0x7Du8; 512];
    let _ = dlms_hdlc::frame::HdlcFrame::decode(&escapes);
}

#[test]
fn fuzz_edge_zero_harmonics() {
    let zeros = [0.0f32; 50];
    let _ = femeter_core::power_quality::calculate_thd(&zeros, 1, 50);
    let _ = femeter_core::power_quality::analyze_harmonics(&zeros);
}

#[test]
fn fuzz_edge_extreme_phase_data() {
    let data = femeter_core::PhaseData {
        voltage_a: 0xFFFF,
        voltage_b: 0x0000,
        voltage_c: 0x8000,
        current_a: 0xFFFF,
        current_b: 0x0000,
        current_c: 0x8000,
        active_power_total: i32::MIN,
        reactive_power_total: i32::MAX,
        frequency: 0xFFFF,
        power_factor_total: 0x8000,
        ..Default::default()
    };
    let mut det = femeter_core::event_detect::EventDetector::new();
    let _ = det.check(&data);
    let mut td = femeter_core::tamper_detection::TamperDetector::new(22000, 100);
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
}
