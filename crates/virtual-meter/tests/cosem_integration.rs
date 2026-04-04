//! Integration test: Virtual meter COSEM protocol interaction via HDLC
//!
//! Tests the complete DLMS/COSEM stack:
//! 1. HDLC frame encoding/decoding
//! 2. APDU GetRequest/GetResponse for standard meter objects
//! 3. COSEM server object initialization and attribute access
//! 4. Meter data synchronization to COSEM objects
//! 5. Profile generic, demand register, and clock interactions

use dlms_apdu::{
    get::{GetRequest, GetRequestNormal},
    types::AttributeDescriptor,
    InvokeId,
};
use dlms_core::{CosemClass, DlmsType, ObisCode};
use dlms_cosem::data_register::ic3_register::Register;
use dlms_meter_app::CosemServer;
use dlms_meter_app::MeterApp;

// ── Helper ──

fn make_get_request_normal(class_id: u16, obis: [u8; 6], attr_id: u8) -> GetRequest {
    let desc = AttributeDescriptor::new(
        class_id,
        ObisCode::new(obis[0], obis[1], obis[2], obis[3], obis[4], obis[5]),
        attr_id,
    );
    GetRequest::Normal(GetRequestNormal::new(InvokeId::new(1), desc))
}

// ── Test Suite ──

#[test]
fn test_cosem_server_init_and_lookup() {
    let mut server = CosemServer::new();
    server.init_standard_objects();

    let list = server.object_list();
    assert!(list.len() >= 10);

    // Check specific objects exist
    assert!(server
        .get_object(8, &ObisCode::new(0, 0, 1, 0, 0, 255))
        .is_some());
    assert!(server
        .get_object(3, &ObisCode::new(1, 0, 32, 7, 0, 255))
        .is_some());
    assert!(server
        .get_object(3, &ObisCode::new(1, 0, 1, 8, 0, 255))
        .is_some());
    assert!(server
        .get_object(5, &ObisCode::new(1, 0, 1, 6, 0, 255))
        .is_some());
    assert!(server
        .get_object(7, &ObisCode::new(1, 0, 99, 1, 0, 255))
        .is_some());
}

#[test]
fn test_cosem_register_read_write() {
    let mut server = CosemServer::new();
    server.init_standard_objects();

    // Read logical_name (attr 1)
    let name = server
        .get_attribute(3, &ObisCode::new(1, 0, 1, 8, 0, 255), 1)
        .unwrap();
    assert!(matches!(name, DlmsType::OctetString(_)));

    // Read scaler_unit (attr 3) — returns Structure(scaler, unit)
    let scaler = server
        .get_attribute(3, &ObisCode::new(1, 0, 1, 8, 0, 255), 3)
        .unwrap();
    assert!(matches!(scaler, DlmsType::Structure(_)));

    // Write and read energy value
    server
        .set_attribute(
            3,
            &ObisCode::new(1, 0, 1, 8, 0, 255),
            2,
            DlmsType::Int64(123456),
        )
        .unwrap();
    let val = server
        .get_attribute(3, &ObisCode::new(1, 0, 1, 8, 0, 255), 2)
        .unwrap();
    assert_eq!(val, DlmsType::Int64(123456));
}

#[test]
fn test_cosem_clock_operations() {
    let mut server = CosemServer::new();
    server.init_standard_objects();

    // Read time zone (attr 3) — Int16
    let tz = server
        .get_attribute(8, &ObisCode::new(0, 0, 1, 0, 0, 255), 3)
        .unwrap();
    assert!(matches!(tz, DlmsType::Int16(_)));

    // Read status (attr 4) — UInt8
    let status = server
        .get_attribute(8, &ObisCode::new(0, 0, 1, 0, 0, 255), 4)
        .unwrap();
    assert!(matches!(status, DlmsType::UInt8(_)));
}

#[test]
fn test_cosem_demand_register() {
    let mut server = CosemServer::new();
    server.init_standard_objects();

    let obis = ObisCode::new(1, 0, 1, 6, 0, 255);

    // Set current demand value
    server
        .set_attribute(5, &obis, 2, DlmsType::Int32(2500))
        .unwrap();
    let val = server.get_attribute(5, &obis, 2).unwrap();
    assert_eq!(val, DlmsType::Int32(2500));

    // Read scaler_unit (attr 4) — Structure
    let scaler = server.get_attribute(5, &obis, 4).unwrap();
    assert!(matches!(scaler, DlmsType::Structure(_)));

    // Reset demand (method 1)
    let result = server.execute_method(5, &obis, 1, DlmsType::Null);
    assert!(result.is_ok());
}

#[test]
fn test_cosem_profile_generic() {
    let mut server = CosemServer::new();
    server.init_standard_objects();

    let obis = ObisCode::new(1, 0, 99, 1, 0, 255);

    // Read capture_objects (attr 3) — Array
    let capture_objects = server.get_attribute(7, &obis, 3).unwrap();
    assert!(matches!(capture_objects, DlmsType::Array(_)));

    // Read capture_period (attr 4) — UInt32
    let period = server.get_attribute(7, &obis, 4).unwrap();
    assert!(matches!(period, DlmsType::UInt32(_)));

    // Read entries_in_use (attr 8) — UInt32
    let entries = server.get_attribute(7, &obis, 8).unwrap();
    assert!(matches!(entries, DlmsType::UInt32(_)));
}

#[test]
fn test_meter_app_sync_to_cosem() {
    let mut app = MeterApp::new();

    for _ in 0..100 {
        app.process_power(1500, 0, 60);
    }
    app.tick(60);

    let mut server = CosemServer::new();
    server.init_standard_objects();
    server.sync_from_meter_app(&app);

    // Verify energy was synced
    let energy = server
        .get_attribute(3, &ObisCode::new(1, 0, 1, 8, 0, 255), 2)
        .unwrap();
    assert!(matches!(energy, DlmsType::Int64(v) if v > 0));

    // Verify power was synced
    let power = server
        .get_attribute(3, &ObisCode::new(1, 0, 1, 7, 0, 255), 2)
        .unwrap();
    assert!(matches!(power, DlmsType::Int32(v) if v > 0));
}

#[test]
fn test_apdu_get_request_encode_decode() {
    let req = make_get_request_normal(3, [1, 0, 1, 8, 0, 255], 2);
    let encoded = match &req {
        GetRequest::Normal(n) => n.encode().unwrap(),
        _ => panic!("expected normal"),
    };
    assert!(!encoded.is_empty());
    assert_eq!(encoded[0], 0xC0);
    assert_eq!(encoded[1], 0x01);
}

#[test]
fn test_cosem_server_unknown_attribute() {
    let mut server = CosemServer::new();
    server.init_standard_objects();

    let result = server.get_attribute(3, &ObisCode::new(1, 0, 1, 8, 0, 255), 99);
    assert!(result.is_err());
}

#[test]
fn test_cosem_server_multiple_syncs() {
    let mut app = MeterApp::new();
    let mut server = CosemServer::new();
    server.init_standard_objects();

    for _ in 0..10 {
        app.process_power(1000, 0, 60);
        app.tick(60);
        server.sync_from_meter_app(&app);
    }

    let energy = server
        .get_attribute(3, &ObisCode::new(1, 0, 1, 8, 0, 255), 2)
        .unwrap();
    assert!(matches!(energy, DlmsType::Int64(_)));
}

#[test]
fn test_cosem_voltage_registers() {
    let mut app = MeterApp::new();

    app.update_voltage(0, 22050).unwrap();
    app.update_voltage(1, 22100).unwrap();
    app.update_voltage(2, 21980).unwrap();

    let mut server = CosemServer::new();
    server.init_standard_objects();
    server.sync_from_meter_app(&app);

    let v_l1 = server
        .get_attribute(3, &ObisCode::new(1, 0, 32, 7, 0, 255), 2)
        .unwrap();
    assert!(matches!(v_l1, DlmsType::UInt32(v) if v > 0));
}

#[test]
fn test_cosem_register_creation() {
    let reg = dlms_cosem::data_register::ic3_register::Register::new(
        ObisCode::new(1, 0, 1, 8, 0, 255),
        -3,
        dlms_core::units::Unit::WattHour,
    );

    assert_eq!(reg.logical_name().to_bytes(), [1, 0, 1, 8, 0, 255]);
    assert_eq!(Register::CLASS_ID, 3);

    let name = reg.get_attribute(1).unwrap();
    assert!(matches!(name, DlmsType::OctetString(_)));

    let su = reg.get_attribute(3).unwrap();
    assert!(matches!(su, DlmsType::Structure(_)));
}

#[test]
fn test_cosem_clock_creation() {
    let clock =
        dlms_cosem::time_control::ic8_clock::Clock::new(ObisCode::new(0, 0, 1, 0, 0, 255), 480);

    assert_eq!(dlms_cosem::time_control::ic8_clock::Clock::CLASS_ID, 8);

    let name = clock.get_attribute(1).unwrap();
    assert!(matches!(name, DlmsType::OctetString(_)));

    let tz = clock.get_attribute(3).unwrap();
    assert!(matches!(tz, DlmsType::Int16(v) if v == 480));
}
