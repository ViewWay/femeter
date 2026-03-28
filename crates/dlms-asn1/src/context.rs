//! Application context name for DLMS/COSEM

use alloc::vec::Vec;
use crate::ber::{BerEncoder, BerDecoder, BerTag, BerError};

/// DLMS/COSEM application context name
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationContextName {
    /// LN referencing with no ciphering
    LogicalNameNoCiphering,
    /// LN referencing with ciphering
    LogicalNameWithCiphering,
    /// SN referencing with no ciphering
    ShortNameNoCiphering,
    /// SN referencing with ciphering
    ShortNameWithCiphering,
    /// Custom OID
    Custom(Vec<u64>),
}

impl ApplicationContextName {
    /// Standard DLMS LN no-ciphering OID: 2.16.776.1.1
    pub const OID_LN_NO_CIPHER: [u64; 5] = [2, 16, 776, 1, 1];
    /// Standard DLMS LN with ciphering OID: 2.16.776.1.2
    pub const OID_LN_WITH_CIPHER: [u64; 5] = [2, 16, 776, 1, 2];
    /// Standard DLMS SN no-ciphering OID: 2.16.776.2.1
    pub const OID_SN_NO_CIPHER: [u64; 5] = [2, 16, 776, 2, 1];
    /// Standard DLMS SN with ciphering OID: 2.16.776.2.2
    pub const OID_SN_WITH_CIPHER: [u64; 5] = [2, 16, 776, 2, 2];

    pub fn oid(&self) -> &[u64] {
        match self {
            Self::LogicalNameNoCiphering => &Self::OID_LN_NO_CIPHER,
            Self::LogicalNameWithCiphering => &Self::OID_LN_WITH_CIPHER,
            Self::ShortNameNoCiphering => &Self::OID_SN_NO_CIPHER,
            Self::ShortNameWithCiphering => &Self::OID_SN_WITH_CIPHER,
            Self::Custom(oid) => oid,
        }
    }

    pub fn encode(&self, enc: &mut BerEncoder) {
        enc.write_oid(self.oid());
    }

    pub fn decode(dec: &mut BerDecoder) -> Result<Self, BerError> {
        let (tag, value) = dec.read_tlv()?;
        if tag != BerTag::universal(0x06) {
            return Err(BerError::InvalidTag);
        }
        // Decode OID value
        if value.len() < 2 {
            return Err(BerError::InvalidData);
        }
        let mut components = Vec::new();
        components.push((value[0] / 40) as u64);
        components.push((value[0] % 40) as u64);

        let mut i = 1;
        while i < value.len() {
            let mut n: u64 = 0;
            while i < value.len() {
                let b = value[i];
                n = (n << 7) | ((b & 0x7F) as u64);
                i += 1;
                if (b & 0x80) == 0 { break; }
            }
            components.push(n);
        }

        if components[..] == Self::OID_LN_NO_CIPHER { Ok(Self::LogicalNameNoCiphering) }
        else if components[..] == Self::OID_LN_WITH_CIPHER { Ok(Self::LogicalNameWithCiphering) }
        else if components[..] == Self::OID_SN_NO_CIPHER { Ok(Self::ShortNameNoCiphering) }
        else if components[..] == Self::OID_SN_WITH_CIPHER { Ok(Self::ShortNameWithCiphering) }
        else { Ok(Self::Custom(components)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_name_roundtrip() {
        let names = [
            ApplicationContextName::LogicalNameNoCiphering,
            ApplicationContextName::LogicalNameWithCiphering,
        ];
        for name in &names {
            let mut enc = BerEncoder::new();
            name.encode(&mut enc);
            let mut dec = BerDecoder::new(enc.to_bytes());
            let decoded = ApplicationContextName::decode(&mut dec).unwrap();
            assert_eq!(name.oid(), decoded.oid());
        }
    }
}
