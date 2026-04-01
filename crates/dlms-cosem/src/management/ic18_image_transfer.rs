//!
//! Interface Class 18: Image Transfer
//!
//! Reference: Blue Book Part 2 §6.8
//!
//! Image Transfer manages firmware image transfer for over-the-air updates.

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// Image transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ImageTransferStatus {
    ImageNotAvailable = 0,
    ImageInitiated = 1,
    ImageVerified = 2,
    ImageVerificationFailed = 3,
    ImageWaitingForActivation = 4,
    ImageActivated = 5,
    Other = 6,
}

/// COSEM IC 18: Image Transfer
///
/// | Attribute | ID | Type | Access |
/// |-----------|----|----|----|
/// | logical_name | 1 | octet-string | static |
/// | image_size | 2 | double-long-unsigned | static |
/// | image_transferred_blocks | 3 | double-long-unsigned | dynamic |
/// | image_transfer_status | 4 | enum | dynamic |
/// | image_first_not_transferred_block | 5 | double-long-unsigned | dynamic |
/// | image_to_activate_info | 6 | octet-string | static |
/// | image_identification | 7 | octet-string | static |
///
/// | Method | ID | Description |
/// |--------|----|----|
/// | image_transfer_initiate | 1 | Start image transfer |
/// | image_block_transfer | 2 | Transfer a block |
/// | image_verify | 3 | Verify the image |
/// | image_activate | 4 | Activate the image |
#[derive(Debug, Clone)]
pub struct ImageTransfer {
    logical_name: ObisCode,
    image_size: u32,
    image_transferred_blocks: u32,
    image_transfer_status: ImageTransferStatus,
    image_first_not_transferred_block: u32,
    image_to_activate_info: DlmsType,
    image_identification: DlmsType,
}

impl ImageTransfer {
    /// Create a new Image Transfer object
    pub fn new(logical_name: ObisCode, image_size: u32, image_identification: DlmsType) -> Self {
        Self {
            logical_name,
            image_size,
            image_transferred_blocks: 0,
            image_transfer_status: ImageTransferStatus::ImageNotAvailable,
            image_first_not_transferred_block: 0,
            image_to_activate_info: DlmsType::OctetString(alloc::vec![]),
            image_identification,
        }
    }

    pub const fn get_status(&self) -> ImageTransferStatus {
        self.image_transfer_status
    }
}

impl CosemClass for ImageTransfer {
    const CLASS_ID: u16 = 18;
    const VERSION: u8 = 0;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        7
    }

    fn method_count() -> u8 {
        4
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => Ok(DlmsType::UInt32(self.image_size)),
            3 => Ok(DlmsType::UInt32(self.image_transferred_blocks)),
            4 => Ok(DlmsType::UInt8(self.image_transfer_status as u8)),
            5 => Ok(DlmsType::UInt32(self.image_first_not_transferred_block)),
            6 => Ok(self.image_to_activate_info.clone()),
            7 => Ok(self.image_identification.clone()),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 2 => Err(CosemError::ReadOnly),
            3 => Err(CosemError::ReadOnly),
            4 => Err(CosemError::ReadOnly),
            5 => Err(CosemError::ReadOnly),
            6 => {
                self.image_to_activate_info = value;
                Ok(())
            }
            7 => {
                self.image_identification = value;
                Ok(())
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, _params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => {
                // image_transfer_initiate
                self.image_transfer_status = ImageTransferStatus::ImageInitiated;
                self.image_transferred_blocks = 0;
                Ok(DlmsType::Null)
            }
            2 => {
                // image_block_transfer
                self.image_transferred_blocks += 1;
                Ok(DlmsType::Null)
            }
            3 => {
                // image_verify
                self.image_transfer_status = ImageTransferStatus::ImageVerified;
                Ok(DlmsType::Null)
            }
            4 => {
                // image_activate
                self.image_transfer_status = ImageTransferStatus::ImageActivated;
                Ok(DlmsType::Null)
            }
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_transfer_class_id() {
        let _it = ImageTransfer::new(
            ObisCode::new(0, 0, 44, 0, 0, 255),
            100000,
            DlmsType::OctetString(alloc::vec![]),
        );
        assert_eq!(ImageTransfer::CLASS_ID, 18);
        assert_eq!(ImageTransfer::method_count(), 4);
    }
}
