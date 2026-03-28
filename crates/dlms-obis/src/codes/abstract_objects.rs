//! Abstract/physical data items (A=0)
//! Reference: Blue Book Part 1

use dlms_core::ObisCode;

// --- Data (IC 1) ---
pub const DATA_VALUE: ObisCode = ObisCode::new(0, 0, 0, 2, 0, 255);
pub const DATA_VALUE_RESET: ObisCode = ObisCode::new(0, 0, 0, 2, 0, 255);

// --- Register (IC 3) ---
pub const REGISTER_VALUE: ObisCode = ObisCode::new(0, 0, 1, 2, 0, 255);

// --- Extended Register (IC 4) ---
pub const EXTENDED_REGISTER_VALUE: ObisCode = ObisCode::new(0, 0, 2, 2, 0, 255);
pub const EXTENDED_REGISTER_STATUS: ObisCode = ObisCode::new(0, 0, 2, 2, 1, 255);
pub const EXTENDED_REGISTER_CAPTURE_TIME: ObisCode = ObisCode::new(0, 0, 2, 2, 2, 255);

// --- Clock (IC 8) ---
pub const CLOCK: ObisCode = ObisCode::new(0, 0, 1, 0, 0, 255);
pub const CLOCK_TIME: ObisCode = ObisCode::new(0, 0, 2, 0, 0, 255);
pub const CLOCK_TIMEZONE: ObisCode = ObisCode::new(0, 0, 3, 0, 0, 255);
pub const CLOCK_STATUS: ObisCode = ObisCode::new(0, 0, 4, 0, 0, 255);

// --- Utility tables (IC 26) ---
pub const UTILITY_TABLES: ObisCode = ObisCode::new(0, 0, 6, 0, 0, 255);

// --- SAP Assignment (IC 17) ---
pub const SAP_ASSIGNMENT: ObisCode = ObisCode::new(0, 0, 3, 0, 0, 255);

// --- Association LN (IC 15) ---
pub const ASSOCIATION_LN: ObisCode = ObisCode::new(0, 0, 40, 0, 1, 255);

// --- Image transfer (IC 18) ---
pub const IMAGE_TRANSFER: ObisCode = ObisCode::new(0, 0, 44, 0, 0, 255);
pub const IMAGE_TRANSFER_STATUS: ObisCode = ObisCode::new(0, 0, 44, 0, 1, 255);
pub const IMAGE_TRANSFER_BLOCKS: ObisCode = ObisCode::new(0, 0, 44, 0, 2, 255);

// --- Activity Calendar (IC 20) ---
pub const ACTIVITY_CALENDAR: ObisCode = ObisCode::new(0, 0, 1, 0, 0, 255);

// --- Script Table (IC 9) ---
pub const SCRIPT_TABLE: ObisCode = ObisCode::new(0, 0, 12, 0, 0, 255);

// --- Security Setup (IC 64) ---
pub const SECURITY_SETUP: ObisCode = ObisCode::new(0, 0, 43, 0, 1, 255);

// --- Push Setup (IC 40) ---
pub const PUSH_SETUP: ObisCode = ObisCode::new(0, 0, 22, 0, 0, 255);

// --- TCP/UDP Setup (IC 41) ---
pub const TCP_UDP_SETUP: ObisCode = ObisCode::new(0, 0, 25, 0, 0, 255);
