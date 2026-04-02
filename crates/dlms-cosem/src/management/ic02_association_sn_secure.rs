//!
//! IC 02 — Association SN (Secure)
//!
//! Short-name association with security context. Extends IC 12 (Association SN)
//! with authenticated/encrypted access support.
//!
//! Reference: Blue Book Part 2 §4.2.2
//!
//! Attributes:
//!   1. logical_name        (octet-string, static)
//!   2. object_list         (array, static)
//!   3. association_status  (enum, dynamic)
//!   4. security_suite      (bit-string, static)
//!   5. user_list           (array, static)
//!   6. current_user        (unsigned, dynamic)
//!   7. associated_partners (structure, static)
//!
//! Methods:
//!   1. change_password_secret(signing_key)
//!   2. add_user(user_name, password, role)
//!   3. remove_user(user_name)
//!
//! Security levels:
//!   - Level 0: No security (same as IC 12)
//!   - Level 1: Low Level Security (password)
//!   - Level 2: High Level Security (GMAC)
//!   - Level 3: High Level Security (SHA-256)
//!   - Level 4: High Level Security with AES-128-GCM encryption
//!   - Level 5: HLS with SHA-256 and AES-128-GCM
//!   - Level 6: HLS with ECDSA and AES-128-GCM
//!   - Level 7: HLS with ECDSA and AES-128-GCM (all signed)
//!   - Level 8: HLS with ECDSA, AES-128-GCM, and digital signature

use dlms_core::{errors::CosemError, obis::ObisCode, traits::CosemClass, types::DlmsType};

/// Maximum number of authorized users
const MAX_USERS: usize = 4;

/// User role / access level
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum UserRole {
    /// Read-only access
    ReadOnly = 0,
    /// Read and write access
    ReadWrite = 1,
    /// Full administration
    Admin = 2,
}

/// User record
#[derive(Clone, Debug)]
pub struct UserRecord {
    /// User ID (1-based)
    pub id: u8,
    /// User name (up to 16 bytes)
    pub name: alloc::vec::Vec<u8>,
    /// Password hash (LLS: 8 bytes; HLS: 16 bytes)
    pub password_hash: alloc::vec::Vec<u8>,
    /// User role
    pub role: UserRole,
}

/// Association status
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AssociationStatus {
    /// No association established
    NoAssociation = 0,
    /// Association pending (authentication in progress)
    Pending = 1,
    /// Association established (authenticated)
    Associated = 2,
    /// Association in degraded mode
    Degraded = 3,
    /// Association terminated
    Terminated = 4,
}

/// IC 02 — Association SN (Secure)
pub struct AssociationSnSecure {
    /// Logical name (OBIS code)
    logical_name: ObisCode,
    /// Current association status
    status: AssociationStatus,
    /// Supported security suites (bit-string: bit 0 = Suite 0, bit 1 = Suite 1, etc.)
    security_suites: u8,
    /// Authorized user list
    users: alloc::vec::Vec<UserRecord>,
    /// Currently active user (0 = none)
    current_user: u8,
    /// Security level of current association
    security_level: u8,
}

impl AssociationSnSecure {
    /// Create a new IC 02 instance
    pub fn new(logical_name: ObisCode) -> Self {
        Self {
            logical_name,
            status: AssociationStatus::NoAssociation,
            security_suites: 0x0F, // Suites 0-3 supported by default
            users: alloc::vec::Vec::new(),
            current_user: 0,
            security_level: 0,
        }
    }

    /// Add a user
    pub fn add_user(
        &mut self,
        id: u8,
        name: alloc::vec::Vec<u8>,
        password_hash: alloc::vec::Vec<u8>,
        role: UserRole,
    ) -> Result<(), CosemError> {
        if self.users.len() >= MAX_USERS {
            return Err(CosemError::HardwareError);
        }
        if self.users.iter().any(|u| u.id == id) {
            return Err(CosemError::HardwareError);
        }
        self.users.push(UserRecord {
            id,
            name,
            password_hash,
            role,
        });
        Ok(())
    }

    /// Remove a user by ID
    pub fn remove_user(&mut self, user_id: u8) -> Result<(), CosemError> {
        let idx = self
            .users
            .iter()
            .position(|u| u.id == user_id)
            .ok_or(CosemError::ObjectNotFound)?;
        self.users.remove(idx);
        if self.current_user == user_id {
            self.current_user = 0;
            self.status = AssociationStatus::NoAssociation;
        }
        Ok(())
    }

    /// Verify user password (LLS mode)
    pub fn verify_lls_password(&self, user_id: u8, password: &[u8]) -> bool {
        self.users
            .iter()
            .find(|u| u.id == user_id)
            .map(|u| {
                if u.password_hash.len() == 8 && password.len() == 8 {
                    u.password_hash == password
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    /// Verify user password (HLS mode) — compare raw challenge hash
    pub fn verify_hls_auth(&self, user_id: u8, auth_value: &[u8]) -> bool {
        self.users
            .iter()
            .find(|u| u.id == user_id)
            .map(|u| {
                if u.password_hash.len() == auth_value.len() {
                    u.password_hash == auth_value
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    /// Establish association for a user
    pub fn associate(&mut self, user_id: u8, security_level: u8) -> Result<(), CosemError> {
        if !self.users.iter().any(|u| u.id == user_id) {
            return Err(CosemError::AccessDenied);
        }
        self.current_user = user_id;
        self.security_level = security_level;
        self.status = AssociationStatus::Associated;
        Ok(())
    }

    /// Terminate current association
    pub fn dissociate(&mut self) {
        self.current_user = 0;
        self.security_level = 0;
        self.status = AssociationStatus::Terminated;
    }

    /// Get current association status
    pub fn status(&self) -> AssociationStatus {
        self.status
    }

    /// Get current user's role
    pub fn current_role(&self) -> Option<UserRole> {
        self.users
            .iter()
            .find(|u| u.id == self.current_user)
            .map(|u| u.role)
    }

    /// Set supported security suites
    pub fn set_security_suites(&mut self, suites: u8) {
        self.security_suites = suites;
    }
}

impl CosemClass for AssociationSnSecure {
    const CLASS_ID: u16 = 2;
    const VERSION: u8 = 1;

    fn logical_name(&self) -> &ObisCode {
        &self.logical_name
    }

    fn attribute_count() -> u8 {
        7
    }

    fn method_count() -> u8 {
        3
    }

    fn get_attribute(&self, id: u8) -> Result<DlmsType, CosemError> {
        match id {
            1 => Ok(DlmsType::OctetString(self.logical_name.to_bytes().to_vec())),
            2 => {
                // Object list — simplified: return empty array
                // Full implementation would list all accessible COSEM objects
                Ok(DlmsType::Array(alloc::vec![]))
            }
            3 => Ok(DlmsType::Enum(self.status as u8)),
            4 => {
                // Security suites as bit-string
                let mut bits = alloc::vec![0u8; 2];
                bits[0] = self.security_suites;
                bits[1] = 0;
                Ok(DlmsType::BitString(bits))
            }
            5 => {
                // User list — return array of user structures
                let list: alloc::vec::Vec<DlmsType> = self
                    .users
                    .iter()
                    .map(|u| {
                        DlmsType::Structure(alloc::vec![
                            DlmsType::UInt8(u.id),
                            DlmsType::OctetString(u.name.clone()),
                            DlmsType::Enum(u.role as u8),
                        ])
                    })
                    .collect();
                Ok(DlmsType::Array(list))
            }
            6 => Ok(DlmsType::UInt8(self.current_user)),
            7 => {
                // Associated partners — simplified
                Ok(DlmsType::Structure(alloc::vec![
                    DlmsType::OctetString(alloc::vec![]), // client system title
                    DlmsType::OctetString(alloc::vec![]), // server system title
                ]))
            }
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn set_attribute(&mut self, id: u8, value: DlmsType) -> Result<(), CosemError> {
        match id {
            1 | 2 | 5 => Err(CosemError::ReadOnly),
            3 => {
                if let DlmsType::Enum(s) = value {
                    self.status = match s {
                        0 => AssociationStatus::NoAssociation,
                        1 => AssociationStatus::Pending,
                        4 => AssociationStatus::Terminated,
                        _ => return Err(CosemError::InvalidParameter),
                    };
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 0x55,
                        got: value.tag(),
                    })
                }
            }
            4 => {
                if let DlmsType::BitString(ref bits) = value {
                    self.security_suites = bits.first().copied().unwrap_or(0);
                    Ok(())
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 0x04,
                        got: value.tag(),
                    })
                }
            }
            6 => Err(CosemError::ReadOnly), // current_user is read-only
            7 => Err(CosemError::ReadOnly),
            _ => Err(CosemError::NoSuchAttribute(id)),
        }
    }

    fn execute_method(&mut self, id: u8, params: DlmsType) -> Result<DlmsType, CosemError> {
        match id {
            1 => {
                // change_password_secret(new_password)
                if self.current_user == 0 {
                    return Err(CosemError::AccessDenied);
                }
                if let DlmsType::OctetString(hash) = params {
                    if let Some(user) = self.users.iter_mut().find(|u| u.id == self.current_user) {
                        user.password_hash = hash;
                        Ok(DlmsType::Null)
                    } else {
                        Err(CosemError::AccessDenied)
                    }
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 9,
                        got: params.tag(),
                    })
                }
            }
            2 => {
                // add_user(name, password_hash, role)
                if self.current_role() != Some(UserRole::Admin) {
                    return Err(CosemError::AccessDenied);
                }
                if let DlmsType::Structure(items) = params {
                    if items.len() >= 3 {
                        let name = match &items[0] {
                            DlmsType::OctetString(n) => n.clone(),
                            _ => {
                                return Err(CosemError::TypeMismatch {
                                    expected: 9,
                                    got: items[0].tag(),
                                })
                            }
                        };
                        let hash = match &items[1] {
                            DlmsType::OctetString(h) => h.clone(),
                            _ => {
                                return Err(CosemError::TypeMismatch {
                                    expected: 9,
                                    got: items[1].tag(),
                                })
                            }
                        };
                        let role = match &items[2] {
                            DlmsType::Enum(r) => match *r {
                                0 => UserRole::ReadOnly,
                                1 => UserRole::ReadWrite,
                                2 => UserRole::Admin,
                                _ => return Err(CosemError::InvalidParameter),
                            },
                            _ => {
                                return Err(CosemError::TypeMismatch {
                                    expected: 0x55,
                                    got: items[2].tag(),
                                })
                            }
                        };
                        let new_id = self.users.iter().map(|u| u.id).max().unwrap_or(0) + 1;
                        self.add_user(new_id, name, hash, role)?;
                        Ok(DlmsType::Null)
                    } else {
                        Err(CosemError::InvalidParameter)
                    }
                } else {
                    Err(CosemError::TypeMismatch {
                        expected: 2,
                        got: params.tag(),
                    })
                }
            }
            3 => {
                // remove_user(user_id)
                if self.current_role() != Some(UserRole::Admin) {
                    return Err(CosemError::AccessDenied);
                }
                let user_id = params.as_u8().ok_or(CosemError::TypeMismatch {
                    expected: 0x12,
                    got: params.tag(),
                })?;
                self.remove_user(user_id)?;
                Ok(DlmsType::Null)
            }
            _ => Err(CosemError::NoSuchMethod(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dlms_core::traits::CosemClass;

    #[test]
    fn test_creation() {
        let assoc = AssociationSnSecure::new(ObisCode::new(0, 0, 40, 0, 0, 255));
        assert_eq!(assoc.status(), AssociationStatus::NoAssociation);
        assert_eq!(<AssociationSnSecure as CosemClass>::CLASS_ID, 2);
        assert_eq!(AssociationSnSecure::attribute_count(), 7);
        assert_eq!(AssociationSnSecure::method_count(), 3);
    }

    #[test]
    fn test_add_remove_user() {
        let mut assoc = AssociationSnSecure::new(ObisCode::new(0, 0, 40, 0, 0, 255));
        assoc
            .add_user(1, b"admin".to_vec(), alloc::vec![0xAA; 8], UserRole::Admin)
            .unwrap();
        assoc
            .add_user(
                2,
                b"user".to_vec(),
                alloc::vec![0xBB; 8],
                UserRole::ReadOnly,
            )
            .unwrap();

        // Duplicate ID should fail
        assert!(assoc
            .add_user(1, b"x".to_vec(), alloc::vec![0; 8], UserRole::ReadOnly)
            .is_err());

        // Max users
        assert!(assoc
            .add_user(3, b"a".to_vec(), alloc::vec![0; 8], UserRole::ReadOnly)
            .is_ok());
        assert!(assoc
            .add_user(4, b"b".to_vec(), alloc::vec![0; 8], UserRole::ReadOnly)
            .is_ok());
        assert!(assoc
            .add_user(5, b"c".to_vec(), alloc::vec![0; 8], UserRole::ReadOnly)
            .is_err());

        // Remove user
        assoc.remove_user(2).unwrap();
        assert!(assoc.remove_user(99).is_err());
    }

    #[test]
    fn test_association_lifecycle() {
        let mut assoc = AssociationSnSecure::new(ObisCode::new(0, 0, 40, 0, 0, 255));
        assoc
            .add_user(1, b"admin".to_vec(), alloc::vec![0xAA; 8], UserRole::Admin)
            .unwrap();

        // Not associated
        assert_eq!(assoc.current_role(), None);

        // Associate
        assoc.associate(1, 2).unwrap();
        assert_eq!(assoc.status(), AssociationStatus::Associated);
        assert_eq!(assoc.current_role(), Some(UserRole::Admin));

        // Dissociate
        assoc.dissociate();
        assert_eq!(assoc.status(), AssociationStatus::Terminated);
        assert_eq!(assoc.current_role(), None);
    }

    #[test]
    fn test_lls_password_verify() {
        let mut assoc = AssociationSnSecure::new(ObisCode::new(0, 0, 40, 0, 0, 255));
        let password: alloc::vec::Vec<u8> =
            alloc::vec![0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38];
        assoc
            .add_user(1, b"admin".to_vec(), password.clone(), UserRole::Admin)
            .unwrap();

        assert!(assoc.verify_lls_password(1, &password));
        assert!(!assoc.verify_lls_password(1, &[0xFF; 8]));
        assert!(!assoc.verify_lls_password(99, &password)); // non-existent user
    }

    #[test]
    fn test_get_attribute() {
        let assoc = AssociationSnSecure::new(ObisCode::new(0, 0, 40, 0, 0, 255));
        // Attr 1: logical name
        let ln = assoc.get_attribute(1).unwrap();
        assert!(matches!(ln, DlmsType::OctetString(_)));
        // Attr 3: status
        let status = assoc.get_attribute(3).unwrap();
        assert_eq!(status, DlmsType::Enum(0)); // NoAssociation
                                               // Invalid attr
        assert!(assoc.get_attribute(99).is_err());
    }

    #[test]
    fn test_set_attribute_read_only() {
        let mut assoc = AssociationSnSecure::new(ObisCode::new(0, 0, 40, 0, 0, 255));
        assert!(assoc.set_attribute(1, DlmsType::Null).is_err()); // read-only
        assert!(assoc.set_attribute(99, DlmsType::Null).is_err()); // no such attr
    }

    #[test]
    fn test_execute_change_password() {
        let mut assoc = AssociationSnSecure::new(ObisCode::new(0, 0, 40, 0, 0, 255));
        assoc
            .add_user(1, b"admin".to_vec(), alloc::vec![0xAA; 8], UserRole::Admin)
            .unwrap();
        assoc.associate(1, 1).unwrap();

        // Change password
        let new_hash = DlmsType::OctetString(alloc::vec![0xBB; 8]);
        assert!(assoc.execute_method(1, new_hash).is_ok());
    }

    #[test]
    fn test_execute_change_password_no_auth() {
        let mut assoc = AssociationSnSecure::new(ObisCode::new(0, 0, 40, 0, 0, 255));
        let new_hash = DlmsType::OctetString(alloc::vec![0xBB; 8]);
        assert!(assoc.execute_method(1, new_hash).is_err()); // not associated
    }

    #[test]
    fn test_execute_remove_user() {
        let mut assoc = AssociationSnSecure::new(ObisCode::new(0, 0, 40, 0, 0, 255));
        assoc
            .add_user(1, b"admin".to_vec(), alloc::vec![0; 8], UserRole::Admin)
            .unwrap();
        assoc
            .add_user(2, b"user".to_vec(), alloc::vec![0; 8], UserRole::ReadOnly)
            .unwrap();
        assoc.associate(1, 1).unwrap();

        // Admin can remove
        assert!(assoc.execute_method(3, DlmsType::UInt8(2)).is_ok());
        // Remove non-existent
        assert!(assoc.execute_method(3, DlmsType::UInt8(99)).is_err());
    }

    #[test]
    fn test_execute_add_user_method() {
        let mut assoc = AssociationSnSecure::new(ObisCode::new(0, 0, 40, 0, 0, 255));
        assoc
            .add_user(1, b"admin".to_vec(), alloc::vec![0; 8], UserRole::Admin)
            .unwrap();
        assoc.associate(1, 1).unwrap();

        let params = DlmsType::Structure(alloc::vec![
            DlmsType::OctetString(b"newuser".to_vec()),
            DlmsType::OctetString(alloc::vec![0xCC; 8]),
            DlmsType::Enum(1), // ReadWrite
        ]);
        assert!(assoc.execute_method(2, params).is_ok());
    }
}
