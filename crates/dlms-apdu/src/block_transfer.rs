//! General Block Transfer
//!
//! Reference: IEC 62056-53 §8.4.7

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use crate::codec::{ApduDecoder, ApduEncoder};
use crate::types::{ApduError, InvokeId, TAG_GENERAL_BLOCK_TRANSFER};
use crate::types::{
    BTF_ACTION_REQUEST, BTF_ACTION_RESPONSE, BTF_GET_REQUEST, BTF_GET_RESPONSE, BTF_LAST_BLOCK,
    BTF_SET_REQUEST, BTF_SET_RESPONSE,
};
#[allow(unused_imports)]
use alloc::vec;
use alloc::vec::Vec;

/// General Block Transfer PDU
///
/// Used for transferring large data in blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralBlockTransfer {
    pub invoke_id: InvokeId,
    pub block_number: u32,
    pub last_block: bool,
    pub command: BlockTransferCommand,
}

impl GeneralBlockTransfer {
    pub fn new(
        invoke_id: InvokeId,
        block_number: u32,
        last_block: bool,
        command: BlockTransferCommand,
    ) -> Self {
        Self {
            invoke_id,
            block_number,
            last_block,
            command,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut enc = ApduEncoder::new();
        enc.write_tag(TAG_GENERAL_BLOCK_TRANSFER, 0x00);
        enc.write_invoke_id(self.invoke_id);

        // Block control: bit 7 = last block flag, bits 0-4 = command
        let block_control = if self.last_block { 0x80 } else { 0x00 } | self.command.to_byte();
        enc.write_byte(block_control);

        // Block number
        enc.write_u32(self.block_number);

        // Encode command-specific data
        self.command.encode(&mut enc);

        enc.into_bytes()
    }

    pub fn decode(data: &[u8]) -> Result<Self, ApduError> {
        let mut dec = ApduDecoder::new(data);

        let (tag_type, _subtype) = dec.read_tag()?;
        if tag_type != TAG_GENERAL_BLOCK_TRANSFER {
            return Err(ApduError::InvalidTag(tag_type));
        }

        let invoke_id = dec.read_invoke_id()?;
        let block_control = dec.read_byte()?;
        let last_block = (block_control & 0x80) != 0;
        let command_byte = block_control & 0x7F;
        let block_number = dec.read_u32()?;

        let command = BlockTransferCommand::decode(command_byte, &data[dec.position()..])?;

        Ok(Self {
            invoke_id,
            block_number,
            last_block,
            command,
        })
    }
}

/// Block transfer command types
#[derive(Debug, Clone, PartialEq)]
pub enum BlockTransferCommand {
    /// Last block acknowledgment
    LastBlockAcknowledged,
    /// Get-Request block
    GetRequestBlock { data: Vec<u8> },
    /// Get-Response block
    GetResponseBlock { data: Vec<u8> },
    /// Set-Request block
    SetRequestBlock { data: Vec<u8> },
    /// Set-Response block
    SetResponseBlock { data: Vec<u8> },
    /// Action-Request block
    ActionRequestBlock { data: Vec<u8> },
    /// Action-Response block
    ActionResponseBlock { data: Vec<u8> },
}

impl BlockTransferCommand {
    pub fn to_byte(&self) -> u8 {
        match self {
            Self::LastBlockAcknowledged => BTF_LAST_BLOCK,
            Self::GetRequestBlock { .. } => BTF_GET_REQUEST,
            Self::GetResponseBlock { .. } => BTF_GET_RESPONSE,
            Self::SetRequestBlock { .. } => BTF_SET_REQUEST,
            Self::SetResponseBlock { .. } => BTF_SET_RESPONSE,
            Self::ActionRequestBlock { .. } => BTF_ACTION_REQUEST,
            Self::ActionResponseBlock { .. } => BTF_ACTION_RESPONSE,
        }
    }

    pub fn encode(&self, enc: &mut ApduEncoder) {
        match self {
            Self::LastBlockAcknowledged => {
                // No additional data
            }
            Self::GetRequestBlock { data }
            | Self::GetResponseBlock { data }
            | Self::SetRequestBlock { data }
            | Self::SetResponseBlock { data }
            | Self::ActionRequestBlock { data }
            | Self::ActionResponseBlock { data } => {
                // Write data length (u32) followed by data
                enc.write_u32(data.len() as u32);
                enc.write_bytes(data);
            }
        }
    }

    pub fn decode(command: u8, data: &[u8]) -> Result<Self, ApduError> {
        match command {
            BTF_LAST_BLOCK => Ok(Self::LastBlockAcknowledged),
            BTF_GET_REQUEST | BTF_GET_RESPONSE | BTF_SET_REQUEST | BTF_SET_RESPONSE
            | BTF_ACTION_REQUEST | BTF_ACTION_RESPONSE => {
                if data.len() < 4 {
                    return Err(ApduError::TooShort);
                }
                let data_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
                if data.len() < 4 + data_len {
                    return Err(ApduError::TooShort);
                }
                let block_data = data[4..4 + data_len].to_vec();

                Ok(match command {
                    BTF_GET_REQUEST => Self::GetRequestBlock { data: block_data },
                    BTF_GET_RESPONSE => Self::GetResponseBlock { data: block_data },
                    BTF_SET_REQUEST => Self::SetRequestBlock { data: block_data },
                    BTF_SET_RESPONSE => Self::SetResponseBlock { data: block_data },
                    BTF_ACTION_REQUEST => Self::ActionRequestBlock { data: block_data },
                    BTF_ACTION_RESPONSE => Self::ActionResponseBlock { data: block_data },
                    _ => unreachable!(),
                })
            }
            _ => Err(ApduError::InvalidData),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_general_block_transfer_last_block() {
        let gbt = GeneralBlockTransfer::new(
            InvokeId::new(1),
            5,
            true,
            BlockTransferCommand::LastBlockAcknowledged,
        );
        let encoded = gbt.encode();

        assert_eq!(encoded[0], TAG_GENERAL_BLOCK_TRANSFER);
        assert_eq!(encoded[2], 1); // invoke_id
                                   // Block control with last_block flag and command
        assert_eq!(encoded[3], 0x80 | BTF_LAST_BLOCK);
    }

    #[test]
    fn test_general_block_transfer_with_data() {
        let data = vec![1, 2, 3, 4, 5];
        let gbt = GeneralBlockTransfer::new(
            InvokeId::new(1),
            0,
            false,
            BlockTransferCommand::GetRequestBlock { data },
        );
        let encoded = gbt.encode();

        assert_eq!(encoded[0], TAG_GENERAL_BLOCK_TRANSFER);
        assert_eq!(encoded[2], 1); // invoke_id
        assert_eq!(encoded[3], BTF_GET_REQUEST); // command without last_block flag
                                                 // Block number
        assert_eq!(&encoded[4..8], [0, 0, 0, 0]);
        // Data length
        assert_eq!(&encoded[8..12], [0, 0, 0, 5]);
        // Data
        assert_eq!(&encoded[12..17], [1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_general_block_transfer_roundtrip() {
        let data = vec![0xAA, 0xBB, 0xCC];
        let gbt = GeneralBlockTransfer::new(
            InvokeId::new(42),
            10,
            true,
            BlockTransferCommand::GetResponseBlock { data },
        );
        let encoded = gbt.encode();
        let decoded = GeneralBlockTransfer::decode(&encoded).unwrap();

        assert_eq!(decoded.invoke_id, gbt.invoke_id);
        assert_eq!(decoded.block_number, gbt.block_number);
        assert_eq!(decoded.last_block, gbt.last_block);
        match decoded.command {
            BlockTransferCommand::GetResponseBlock { data: d } => {
                assert_eq!(d, vec![0xAA, 0xBB, 0xCC]);
            }
            _ => panic!("Unexpected command type"),
        }
    }

    #[test]
    fn test_block_transfer_command_to_byte() {
        assert_eq!(
            BlockTransferCommand::LastBlockAcknowledged.to_byte(),
            BTF_LAST_BLOCK
        );
        assert_eq!(
            BlockTransferCommand::GetRequestBlock { data: vec![] }.to_byte(),
            BTF_GET_REQUEST
        );
    }
}
