//! Initiate Request and Response APDUs
//!
//! Reference: IEC 62056-53 §8.5.2
//!
//! These are APDU-level initiate messages (different from the AARQ/AARE
//! association messages which are at the ACSE level).

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use crate::codec::{ApduDecoder, ApduEncoder};
use crate::types::{ApduError, InvokeId};
use alloc::vec::Vec;

/// Initiate Request PDU
///
/// Used by the client to propose protocol parameters at the start of a session.
/// This is sent after the AARQ/AARE association is established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitiateRequest {
    pub invoke_id: InvokeId,
    pub dedicated_key: u8,
    pub negotiated_conformance: u32,
    pub client_max_receive_pdu_size: u16,
    pub server_max_receive_pdu_size: u16,
}

impl InitiateRequest {
    pub fn new(
        invoke_id: InvokeId,
        negotiated_conformance: u32,
        client_max_receive_pdu_size: u16,
        server_max_receive_pdu_size: u16,
    ) -> Self {
        Self {
            invoke_id,
            dedicated_key: 0x00, // Default: no dedicated key
            negotiated_conformance,
            client_max_receive_pdu_size,
            server_max_receive_pdu_size,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(0xFF, 0x01); // Initiate-Request tag
        enc.write_invoke_id(self.invoke_id);
        enc.write_byte(self.dedicated_key);

        // Conformance block (4 bytes: 3 bytes conformance + 1 reserved)
        enc.write_byte((self.negotiated_conformance >> 16) as u8);
        enc.write_byte((self.negotiated_conformance >> 8) as u8);
        enc.write_byte(self.negotiated_conformance as u8);
        enc.write_byte(0x00); // Reserved

        // Client max receive PDU size (2 bytes)
        enc.write_u16(self.client_max_receive_pdu_size);

        // Server max receive PDU size (2 bytes)
        enc.write_u16(self.server_max_receive_pdu_size);

        enc.into_bytes()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != 0xFF || subtype != 0x01 {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let dedicated_key = dec.read_byte()?;

        // Conformance block
        let conf1 = dec.read_byte()? as u32;
        let conf2 = dec.read_byte()? as u32;
        let conf3 = dec.read_byte()? as u32;
        let _reserved = dec.read_byte()?;
        let negotiated_conformance = (conf1 << 16) | (conf2 << 8) | conf3;

        let client_max_receive_pdu_size = dec.read_u16()?;
        let server_max_receive_pdu_size = dec.read_u16()?;

        Ok(Self {
            invoke_id,
            dedicated_key,
            negotiated_conformance,
            client_max_receive_pdu_size,
            server_max_receive_pdu_size,
        })
    }
}

/// Initiate Response PDU
///
/// Sent by the server in response to Initiate Request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitiateResponse {
    pub invoke_id: InvokeId,
    pub negotiated_conformance: u32,
    pub server_max_receive_pdu_size: u16,
    pub client_max_receive_pdu_size: u16,
}

impl InitiateResponse {
    pub fn new(
        invoke_id: InvokeId,
        negotiated_conformance: u32,
        server_max_receive_pdu_size: u16,
        client_max_receive_pdu_size: u16,
    ) -> Self {
        Self {
            invoke_id,
            negotiated_conformance,
            server_max_receive_pdu_size,
            client_max_receive_pdu_size,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(0xFF, 0x02); // Initiate-Response tag
        enc.write_invoke_id(self.invoke_id);

        // Conformance block (4 bytes: 3 bytes conformance + 1 reserved)
        enc.write_byte((self.negotiated_conformance >> 16) as u8);
        enc.write_byte((self.negotiated_conformance >> 8) as u8);
        enc.write_byte(self.negotiated_conformance as u8);
        enc.write_byte(0x00); // Reserved

        // Server max receive PDU size (2 bytes)
        enc.write_u16(self.server_max_receive_pdu_size);

        // Client max receive PDU size (2 bytes)
        enc.write_u16(self.client_max_receive_pdu_size);

        enc.into_bytes()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, subtype) = dec.read_tag()?;
        if tag_type != 0xFF || subtype != 0x02 {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;

        // Conformance block
        let conf1 = dec.read_byte()? as u32;
        let conf2 = dec.read_byte()? as u32;
        let conf3 = dec.read_byte()? as u32;
        let _reserved = dec.read_byte()?;
        let negotiated_conformance = (conf1 << 16) | (conf2 << 8) | conf3;

        let server_max_receive_pdu_size = dec.read_u16()?;
        let client_max_receive_pdu_size = dec.read_u16()?;

        Ok(Self {
            invoke_id,
            negotiated_conformance,
            server_max_receive_pdu_size,
            client_max_receive_pdu_size,
        })
    }
}

/// Conformance bit definitions
///
/// These bits indicate which DLMS services are supported.
pub mod conformance {
    /// General protection (bit 0)
    pub const GENERAL_PROTECTION: u32 = 0x000001;
    /// General block transfer (bit 1)
    pub const GENERAL_BLOCK_TRANSFER: u32 = 0x000002;
    /// Get (bit 2)
    pub const GET: u32 = 0x000004;
    /// Set (bit 3)
    pub const SET: u32 = 0x000008;
    /// Selective access (bit 4)
    pub const SELECTIVE_ACCESS: u32 = 0x000010;
    /// Action (bit 5)
    pub const ACTION: u32 = 0x000020;
    /// Parameterized access (bit 6)
    pub const PARAMETERIZED_ACCESS: u32 = 0x000040;
    /// Get with list (bit 7)
    pub const GET_WITH_LIST: u32 = 0x000080;
    /// Set with list (bit 8)
    pub const SET_WITH_LIST: u32 = 0x000100;
    /// Event notification (bit 9)
    pub const EVENT_NOTIFICATION: u32 = 0x000200;
    /// Atomic read (bit 10)
    pub const ATOMIC_READ: u32 = 0x000400;
    /// Atomic write (bit 11)
    pub const ATOMIC_WRITE: u32 = 0x000800;
    /// Parameterized get (bit 12)
    pub const PARAMETERIZED_GET: u32 = 0x001000;
    /// Parameterized set (bit 13)
    pub const PARAMETERIZED_SET: u32 = 0x002000;
    /// Snapshot (bit 14)
    pub const SNAPSHOT: u32 = 0x004000;
    /// Block transfer with get/set (bit 15)
    pub const BLOCK_TRANSFER_WITH_GET_OR_SET: u32 = 0x008000;
    /// Block transfer with action (bit 16)
    pub const BLOCK_TRANSFER_WITH_ACTION: u32 = 0x010000;
    /// Data notification (bit 17)
    pub const DATA_NOTIFICATION: u32 = 0x020000;
    /// Access (bit 18)
    pub const ACCESS: u32 = 0x040000;
    /// Reserved (bit 19)
    pub const RESERVED_BIT_19: u32 = 0x080000;
    /// Reserved (bit 20)
    pub const RESERVED_BIT_20: u32 = 0x100000;
    /// Reserved (bit 21)
    pub const RESERVED_BIT_21: u32 = 0x200000;
    /// Reserved (bit 22)
    pub const RESERVED_BIT_22: u32 = 0x400000;
    /// Reserved (bit 23)
    pub const RESERVED_BIT_23: u32 = 0x800000;

    /// Standard meter conformance (common features)
    pub const fn standard_meter() -> u32 {
        GET | SET | ACTION | EVENT_NOTIFICATION | GENERAL_PROTECTION
    }

    /// Full conformance (all standard features)
    pub const fn full() -> u32 {
        GENERAL_PROTECTION
            | GENERAL_BLOCK_TRANSFER
            | GET
            | SET
            | SELECTIVE_ACCESS
            | ACTION
            | GET_WITH_LIST
            | SET_WITH_LIST
            | EVENT_NOTIFICATION
            | ATOMIC_READ
            | ATOMIC_WRITE
            | PARAMETERIZED_GET
            | PARAMETERIZED_SET
            | SNAPSHOT
            | BLOCK_TRANSFER_WITH_GET_OR_SET
            | BLOCK_TRANSFER_WITH_ACTION
            | DATA_NOTIFICATION
    }

    /// Check if a feature is supported
    pub const fn has_feature(conformance: u32, feature: u32) -> bool {
        (conformance & feature) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initiate_request_encode() {
        let req = InitiateRequest::new(InvokeId::new(1), conformance::standard_meter(), 2048, 2048);
        let encoded = req.encode();

        assert_eq!(encoded[0], 0xFF); // Initiate tag
        assert_eq!(encoded[1], 0x01); // Request subtype
        assert_eq!(encoded[2], 1); // invoke_id
    }

    #[test]
    fn test_initiate_request_roundtrip() {
        let req = InitiateRequest::new(
            InvokeId::new(42),
            conformance::GET | conformance::SET | conformance::ACTION,
            1024,
            2048,
        );
        let encoded = req.encode();
        let decoded = InitiateRequest::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, req.invoke_id);
        assert_eq!(decoded.negotiated_conformance, req.negotiated_conformance);
        assert_eq!(
            decoded.client_max_receive_pdu_size,
            req.client_max_receive_pdu_size
        );
        assert_eq!(
            decoded.server_max_receive_pdu_size,
            req.server_max_receive_pdu_size
        );
    }

    #[test]
    fn test_initiate_response_encode() {
        let resp =
            InitiateResponse::new(InvokeId::new(1), conformance::standard_meter(), 2048, 1024);
        let encoded = resp.encode();

        assert_eq!(encoded[0], 0xFF); // Initiate tag
        assert_eq!(encoded[1], 0x02); // Response subtype
        assert_eq!(encoded[2], 1); // invoke_id
    }

    #[test]
    fn test_initiate_response_roundtrip() {
        let resp = InitiateResponse::new(
            InvokeId::new(42),
            conformance::GET | conformance::SET,
            2048,
            1024,
        );
        let encoded = resp.encode();
        let decoded = InitiateResponse::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, resp.invoke_id);
        assert_eq!(decoded.negotiated_conformance, resp.negotiated_conformance);
        assert_eq!(
            decoded.server_max_receive_pdu_size,
            resp.server_max_receive_pdu_size
        );
        assert_eq!(
            decoded.client_max_receive_pdu_size,
            resp.client_max_receive_pdu_size
        );
    }

    #[test]
    fn test_conformance_has_feature() {
        let conf = conformance::GET | conformance::SET;
        assert!(conformance::has_feature(conf, conformance::GET));
        assert!(conformance::has_feature(conf, conformance::SET));
        assert!(!conformance::has_feature(conf, conformance::ACTION));
    }

    #[test]
    fn test_conformance_standard_meter() {
        let conf = conformance::standard_meter();
        assert!(conformance::has_feature(conf, conformance::GET));
        assert!(conformance::has_feature(conf, conformance::SET));
        assert!(conformance::has_feature(conf, conformance::ACTION));
        assert!(conformance::has_feature(
            conf,
            conformance::EVENT_NOTIFICATION
        ));
        assert!(conformance::has_feature(
            conf,
            conformance::GENERAL_PROTECTION
        ));
    }

    #[test]
    fn test_conformance_full() {
        let conf = conformance::full();
        assert!(conformance::has_feature(conf, conformance::GET));
        assert!(conformance::has_feature(conf, conformance::SET));
        assert!(conformance::has_feature(conf, conformance::ACTION));
        assert!(conformance::has_feature(
            conf,
            conformance::GENERAL_BLOCK_TRANSFER
        ));
        assert!(conformance::has_feature(
            conf,
            conformance::SELECTIVE_ACCESS
        ));
    }
}
