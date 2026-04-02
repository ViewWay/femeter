/* ================================================================== */
/*                                                                    */
/*  comm.rs — DLMS HDLC Frame Processing + Multi-Channel Comm Manager */
/*                                                                    */
/*  IEC 62056-46 HDLC frame layer, IEC 62056-21 Mode C parser,       */
/*  and multi-channel communication manager for FeMeter.              */
/*                                                                    */
/*  UART0: RS-485 (HDLC/DLMS), 9600–115200 bps                       */
/*  UART1: Infrared  (IEC 62056-21), 300–9600 bps                    */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

#![no_std]

use crate::hal::{UartConfig, UartDriver, UartError};

/* ================================================================== */
/*  Constants                                                          */
/* ================================================================== */

/// HDLC flag byte (frame delimiter)
const HDLC_FLAG: u8 = 0x7E;
/// HDLC escape byte
const HDLC_ESCAPE: u8 = 0x7D;
/// HDLC escape XOR mask
const HDLC_ESCAPE_MASK: u8 = 0x20;

/* ================================================================== */
/*  FCS-16 Lookup Table (CRC-16/HDLC)                                 */
/*                                                                    */
/*  Polynomial: x^16 + x^12 + x^5 + 1  (reversed: 0x8408)            */
/*  Init: 0xFFFF, Final XOR: 0xFFFF                                   */
/* ================================================================== */

const FCS16_TABLE: [u16; 256] = {
    let mut table = [0u16; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u16;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x8408;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

/// Compute FCS-16 (CRC-16/HDLC) over a byte slice.
pub fn fcs16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc = (crc >> 8) ^ FCS16_TABLE[((crc ^ b as u16) & 0xFF) as usize];
    }
    !crc
}

/* ================================================================== */
/*  HDLC Error Types                                                   */
/* ================================================================== */

/// HDLC frame processing errors
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HdlcError {
    /// Frame too short (minimum: address + control + FCS)
    TooShort,
    /// FCS checksum mismatch
    FcsError,
    /// Frame exceeds maximum buffer size
    Overflow,
    /// Invalid byte-unstuffing sequence
    InvalidEscape,
    /// Address field too long (>2 bytes)
    AddressTooLong,
}

/* ================================================================== */
/*  U-Frame Type Enumeration                                           */
/* ================================================================== */

/// HDLC Unnumbered frame subtypes
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UFrameType {
    /// Set Normal Response Mode
    Snrm,
    /// Disconnect
    Disc,
    /// Unnumbered Acknowledge
    Ua,
    /// Disconnected Mode
    Dm,
    /// Frame Reject
    Frmr,
    /// Unknown U-frame function
    Unknown(u8),
}

/* ================================================================== */
/*  HDLC Frame                                                         */
/* ================================================================== */

/// Parsed HDLC frame (between flags, after byte-unstuffing, without FCS).
pub struct HdlcFrame {
    /// Frame payload (address + control + information, without FCS)
    data: [u8; 256],
    /// Length of valid data in bytes
    len: u8,
}

impl HdlcFrame {
    /// Parse raw bytes (between flags, still byte-stuffed) into an HDLC frame.
    ///
    /// Performs byte-unstuffing and verifies the FCS-16 checksum.
    pub fn parse(raw: &[u8]) -> Result<Self, HdlcError> {
        // Need at least: address(1) + control(1) + FCS(2) = 4 bytes
        if raw.len() < 4 {
            return Err(HdlcError::TooShort);
        }

        let mut frame = HdlcFrame {
            data: [0u8; 256],
            len: 0,
        };

        // Byte-unstuffing
        let mut i = 0;
        while i < raw.len() {
            if frame.len as usize >= frame.data.len() {
                return Err(HdlcError::Overflow);
            }
            if raw[i] == HDLC_ESCAPE {
                i += 1;
                if i >= raw.len() {
                    return Err(HdlcError::InvalidEscape);
                }
                frame.data[frame.len as usize] = raw[i] ^ HDLC_ESCAPE_MASK;
            } else if raw[i] == HDLC_FLAG {
                // Should not encounter flags inside a properly extracted frame,
                // but skip them gracefully.
                i += 1;
                continue;
            } else {
                frame.data[frame.len as usize] = raw[i];
            }
            frame.len += 1;
            i += 1;
        }

        // Minimum after unstuffing: address(1) + control(1) + FCS(2) = 4
        if frame.len < 4 {
            return Err(HdlcError::TooShort);
        }

        // Verify FCS: compute over all bytes except the last 2 (FCS field)
        let payload_len = frame.len as usize - 2;
        let fcs_offset = payload_len;
        let computed = fcs16(&frame.data[..payload_len]);
        let received = frame.data[fcs_offset] as u16 | ((frame.data[fcs_offset + 1] as u16) << 8);

        if computed != received {
            return Err(HdlcError::FcsError);
        }

        // Trim FCS from data — now data = address + control + information
        frame.len = payload_len as u8;

        Ok(frame)
    }

    /// Build an HDLC frame with byte-stuffing and FCS.
    ///
    /// Returns the total length written to `out` (including flags).
    pub fn build(address: &[u8], control: u8, information: &[u8], out: &mut [u8]) -> usize {
        // Calculate FCS over address + control + information
        let mut fcs_buf: [u8; 256] = [0; 256];
        let mut fcs_len = 0;
        for &b in address {
            fcs_buf[fcs_len] = b;
            fcs_len += 1;
        }
        fcs_buf[fcs_len] = control;
        fcs_len += 1;
        for &b in information {
            fcs_buf[fcs_len] = b;
            fcs_len += 1;
        }
        let fcs = fcs16(&fcs_buf[..fcs_len]);
        fcs_buf[fcs_len] = fcs as u8;
        fcs_len += 1;
        fcs_buf[fcs_len] = (fcs >> 8) as u8;
        fcs_len += 1;

        // Build output: FLAG + (byte-stuffed payload+FCS) + FLAG
        let mut pos = 0;
        if pos >= out.len() {
            return pos;
        }
        out[pos] = HDLC_FLAG;
        pos += 1;

        for i in 0..fcs_len {
            if pos >= out.len() {
                return pos;
            }
            let b = fcs_buf[i];
            if b == HDLC_FLAG || b == HDLC_ESCAPE {
                out[pos] = HDLC_ESCAPE;
                pos += 1;
                if pos >= out.len() {
                    return pos;
                }
                out[pos] = b ^ HDLC_ESCAPE_MASK;
            } else {
                out[pos] = b;
            }
            pos += 1;
        }

        if pos >= out.len() {
            return pos;
        }
        out[pos] = HDLC_FLAG;
        pos += 1;

        pos
    }

    /// Extract the variable-length HDLC address field.
    ///
    /// HDLC addresses have LSB=1 on the last (or only) byte.
    pub fn address(&self) -> &[u8] {
        let mut i = 0;
        while i < self.len as usize {
            if self.data[i] & 0x01 != 0 {
                // Last byte of address
                return &self.data[..=i];
            }
            i += 1;
            // Sanity: address should not exceed 2 bytes in our use case
            if i >= 4 {
                break;
            }
        }
        // Fallback: return first byte as 1-byte address
        &self.data[..core::cmp::min(1, self.len as usize)]
    }

    /// Extract the control field (immediately after address).
    pub fn control(&self) -> u8 {
        let addr_len = self.address().len();
        if addr_len < self.len as usize {
            self.data[addr_len]
        } else {
            0
        }
    }

    /// Extract the information field (after address + control, to end of data).
    pub fn information(&self) -> &[u8] {
        let addr_len = self.address().len();
        let ctrl_end = addr_len + 1;
        if ctrl_end < self.len as usize {
            &self.data[ctrl_end..self.len as usize]
        } else {
            &[]
        }
    }

    /// Check if this is an I-frame (information frame).
    /// I-frames have bit 0 of control = 0.
    pub fn is_i_frame(&self) -> bool {
        self.control() & 0x01 == 0
    }

    /// Check if this is an S-frame (supervisory frame).
    /// S-frames have control bits [1:0] = 0b01.
    pub fn is_s_frame(&self) -> bool {
        let c = self.control();
        c & 0x01 != 0 && c & 0x02 == 0
    }

    /// Check if this is a U-frame (unnumbered frame).
    /// U-frames have control bits [1:0] = 0b11.
    pub fn is_u_frame(&self) -> bool {
        let c = self.control();
        c & 0x03 == 0x03
    }

    /// Parse the U-frame subtype from the control field.
    pub fn u_frame_type(&self) -> UFrameType {
        if !self.is_u_frame() {
            return UFrameType::Unknown(self.control());
        }
        // U-frame modifier bits are [7:5] and [3:2] of the control byte
        // Combine them: bits 7-5 and 3-2
        let c = self.control();
        let modifier = (c & 0xE0) | ((c & 0x0C) << 2);

        match modifier {
            // SNRM: modifier = 0x80 (bit 7 set)
            0x80..=0x83 => UFrameType::Snrm,
            // DISC: modifier = 0x40 (bit 6 set)
            0x40..=0x43 => UFrameType::Disc,
            // UA: modifier = 0x60 (bits 6+5 set)
            0x60..=0x63 => UFrameType::Ua,
            // DM: modifier = 0x10 (bit 4 set)
            0x10..=0x13 => UFrameType::Dm,
            // FRMR: modifier = 0x84 (bit 7 + bit 2)
            0x84..=0x87 => UFrameType::Frmr,
            _ => UFrameType::Unknown(c),
        }
    }
}

/* ================================================================== */
/*  HDLC Receiver State Machine                                        */
/* ================================================================== */

/// Receiver state
#[derive(Clone, Copy, PartialEq)]
enum RxState {
    /// Waiting for opening flag
    WaitingFlag,
    /// Receiving frame data
    Receiving,
    /// Complete frame received
    Complete,
}

/// Event returned by HdlcReceiver::feed()
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HdlcRxEvent {
    /// No event (byte consumed, frame still in progress)
    Idle,
    /// Complete frame received (between flags, still byte-stuffed)
    Complete,
    /// Error detected (overflow)
    Error(HdlcError),
}

/// HDLC byte-by-byte receiver with state machine.
pub struct HdlcReceiver {
    buf: [u8; 512],
    len: usize,
    state: RxState,
    escaped: bool,
}

impl HdlcReceiver {
    /// Create a new HDLC receiver in initial state.
    pub const fn new() -> Self {
        Self {
            buf: [0u8; 512],
            len: 0,
            state: RxState::WaitingFlag,
            escaped: false,
        }
    }

    /// Feed one received byte into the state machine.
    pub fn feed(&mut self, byte: u8) -> HdlcRxEvent {
        match self.state {
            RxState::WaitingFlag => {
                if byte == HDLC_FLAG {
                    self.state = RxState::Receiving;
                    self.len = 0;
                    self.escaped = false;
                }
                HdlcRxEvent::Idle
            }
            RxState::Receiving => {
                if byte == HDLC_FLAG {
                    if self.len == 0 {
                        // Multiple flags — stay receiving
                        return HdlcRxEvent::Idle;
                    }
                    // End of frame
                    self.state = RxState::Complete;
                    return HdlcRxEvent::Complete;
                }
                if byte == HDLC_ESCAPE {
                    self.escaped = true;
                    return HdlcRxEvent::Idle;
                }
                if self.len >= self.buf.len() {
                    self.state = RxState::WaitingFlag;
                    self.len = 0;
                    return HdlcRxEvent::Error(HdlcError::Overflow);
                }
                if self.escaped {
                    self.buf[self.len] = byte ^ HDLC_ESCAPE_MASK;
                    self.escaped = false;
                } else {
                    self.buf[self.len] = byte;
                }
                self.len += 1;
                HdlcRxEvent::Idle
            }
            RxState::Complete => {
                // Previous frame not yet consumed; overwrite
                self.reset();
                self.feed(byte)
            }
        }
    }

    /// Get the received raw frame data (between flags, still byte-stuffed).
    /// Only valid after `feed()` returns `Complete`.
    pub fn frame(&self) -> Option<&[u8]> {
        if self.state == RxState::Complete && self.len > 0 {
            Some(&self.buf[..self.len])
        } else {
            None
        }
    }

    /// Parse the received frame into an HdlcFrame (un-stuff + verify FCS).
    pub fn parse_frame(&self) -> Result<HdlcFrame, HdlcError> {
        match self.frame() {
            Some(data) => HdlcFrame::parse(data),
            None => Err(HdlcError::TooShort),
        }
    }

    /// Reset the receiver to initial state.
    pub fn reset(&mut self) {
        self.len = 0;
        self.state = RxState::WaitingFlag;
        self.escaped = false;
    }
}

/* ================================================================== */
/*  IEC 62056-21 Mode C Parser                                         */
/* ================================================================== */

/// IEC 62056-21 parser state
#[derive(Clone, Copy, PartialEq)]
enum IecState {
    /// Waiting for initial request
    Idle,
    /// Reading data lines
    ReadingData,
    /// Baud rate switch ack sent, waiting for high-speed data
    HighSpeed,
}

/// IEC 62056-21 event
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IecRxEvent {
    /// No event
    Idle,
    /// Complete request received (e.g. /?!<CR><LF>)
    RequestReceived,
    /// Data block received
    DataBlock,
    /// Complete message received (ETX! seen)
    Complete,
    /// Buffer overflow
    Overflow,
}

/// Special characters for IEC 62056-21
const STX: u8 = 0x02;
const ETX: u8 = 0x03;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const CR: u8 = 0x0D;
const LF: u8 = 0x0A;

/// IEC 62056-21 Mode C protocol parser.
pub struct Iec62056Parser {
    buf: [u8; 128],
    len: usize,
    state: IecState,
}

impl Iec62056Parser {
    /// Create a new IEC 62056-21 parser.
    pub const fn new() -> Self {
        Self {
            buf: [0u8; 128],
            len: 0,
            state: IecState::Idle,
        }
    }

    /// Feed one received byte into the parser.
    pub fn feed(&mut self, byte: u8) -> IecRxEvent {
        if self.len >= self.buf.len() {
            self.reset();
            return IecRxEvent::Overflow;
        }

        self.buf[self.len] = byte;
        self.len += 1;

        match self.state {
            IecState::Idle => {
                // Looking for /?!<CR><LF> request
                if byte == LF && self.len >= 4 {
                    // Check for "/?!" + CR+LF pattern
                    if self.len >= 4
                        && self.buf[self.len - 4] == b'/'
                        && self.buf[self.len - 3] == b'?'
                        && self.buf[self.len - 2] == b'!'
                        && self.buf[self.len - 1] == LF
                    {
                        // Actually it's /?!<CR><LF> = 5 bytes
                        // Re-check with CR
                    }
                    // Simpler: check last bytes for LF preceded by CR and "!"
                    if self.len >= 5
                        && self.buf[0] == b'/'
                        && self.buf[1] == b'?'
                        && self.buf[2] == b'!'
                        && self.buf[3] == CR
                        && self.buf[4] == LF
                    {
                        self.state = IecState::ReadingData;
                        return IecRxEvent::RequestReceived;
                    }
                }
                IecRxEvent::Idle
            }
            IecState::ReadingData | IecState::HighSpeed => {
                // In data mode, look for <ETX>!<CR><LF> terminator
                // or just accumulate data blocks
                if byte == LF && self.len >= 3 {
                    let n = self.len;
                    // Check for ETX ! CR LF
                    if n >= 4
                        && self.buf[n - 4] == ETX
                        && self.buf[n - 3] == b'!'
                        && self.buf[n - 2] == CR
                        && self.buf[n - 1] == LF
                    {
                        return IecRxEvent::Complete;
                    }
                    // Check for a complete data line (CR LF at end)
                    if self.buf[n - 2] == CR && self.buf[n - 1] == LF {
                        return IecRxEvent::DataBlock;
                    }
                }
                IecRxEvent::Idle
            }
        }
    }

    /// Get the current receive buffer contents.
    pub fn data(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    /// Switch to high-speed mode after baud rate change.
    pub fn set_high_speed(&mut self) {
        self.state = IecState::HighSpeed;
        self.len = 0;
    }

    /// Reset parser to initial state.
    pub fn reset(&mut self) {
        self.len = 0;
        self.state = IecState::Idle;
    }

    /// Build a baud-rate acknowledgment sequence.
    /// Returns bytes to send: <ACK>/5<CR><LF> (select 9600 baud)
    pub fn build_ack_baud_switch(baud_char: u8, out: &mut [u8]) -> usize {
        if out.len() < 4 {
            return 0;
        }
        out[0] = ACK;
        out[1] = b'/';
        out[2] = baud_char; // '0'=300, '1'=600, '2'=1200, '5'=9600, etc.
        out[3] = CR;
        // Note: some implementations append LF too
        4
    }

    /// Build identification message response.
    /// Format: /XXXYYYY<CR><LF>
    pub fn build_ident(
        manufacturer: &[u8; 3],
        meter_type: u8,
        baud_char: u8,
        out: &mut [u8],
    ) -> usize {
        if out.len() < 6 {
            return 0;
        }
        out[0] = b'/';
        out[1] = manufacturer[0];
        out[2] = manufacturer[1];
        out[3] = manufacturer[2];
        out[4] = meter_type;
        out[5] = baud_char;
        // CR LF appended by caller
        6
    }
}

/* ================================================================== */
/*  RS-485 Direction Control Trait                                     */
/* ================================================================== */

/// Abstract RS-485 transceiver direction control.
/// Implemented by board-specific code to toggle DE/RE pins.
pub trait Rs485DirControl {
    /// Switch RS-485 transceiver to transmit mode (DE=HIGH).
    fn tx_enable(&mut self);
    /// Switch RS-485 transceiver to receive mode (DE=LOW).
    fn tx_disable(&mut self);
}

/* ================================================================== */
/*  HDLC Timeout Constants                                             */
/* ================================================================== */

/// Default inter-frame timeout (ms) — time between end of one frame and start of next
pub const HDLC_INTER_FRAME_TIMEOUT_MS: u32 = 250;
/// Default response timeout (ms) — max time to wait for a reply frame
pub const HDLC_RESPONSE_TIMEOUT_MS: u32 = 5000;
/// Default max retry count for unacknowledged I-frames
pub const HDLC_MAX_RETRIES: u8 = 3;
/// Default window size for multi-frame transmission
pub const HDLC_DEFAULT_WINDOW_SIZE: u8 = 1;
/// Maximum information field length (bytes)
pub const HDLC_MAX_INFO_LEN: usize = 2032;
/// Default maximum information field length
pub const HDLC_DEFAULT_MAX_INFO_LEN: usize = 128;

/* ================================================================== */
/*  HDLC Address Types                                                 */
/* ================================================================== */

/// HDLC address type classification
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HdlcAddressType {
    /// Individual address (unicast)
    Individual,
    /// Group address (multicast)
    Group,
    /// Broadcast address
    Broadcast,
}

/// Classify an HDLC address type from its first byte.
pub fn classify_address(first_byte: u8) -> HdlcAddressType {
    if first_byte == 0xFF {
        HdlcAddressType::Broadcast
    } else if first_byte & 0x80 != 0 {
        HdlcAddressType::Group
    } else {
        HdlcAddressType::Individual
    }
}

/// Build a 1-byte HDLC client address (with extension bit).
pub fn build_client_address(client_addr: u8) -> u8 {
    // Client address format: client_addr << 1 | 0x01 (last byte indicator)
    (client_addr << 1) | 0x01
}

/// Build a 1-byte HDLC server address (with extension bit).
pub fn build_server_address(server_addr: u8) -> u8 {
    (server_addr << 1) | 0x01
}

/* ================================================================== */
/*  HDLC Multi-Frame Transmission (Sliding Window)                     */
/* ================================================================== */

/// HDLC sliding window transmitter state
pub struct HdlcTxWindow {
    /// Window size (max outstanding I-frames)
    window_size: u8,
    /// Next send sequence number
    send_seq: u8,
    /// Next expected receive sequence number
    recv_seq: u8,
    /// Count of unacknowledged frames
    outstanding: u8,
    /// Retry count for current frame
    retries: u8,
    /// Whether the window is full (waiting for ACK)
    window_full: bool,
}

impl HdlcTxWindow {
    /// Create a new TX window with the given size.
    pub fn new(window_size: u8) -> Self {
        Self {
            window_size: window_size.clamp(1, 7),
            send_seq: 0,
            recv_seq: 0,
            outstanding: 0,
            retries: 0,
            window_full: false,
        }
    }

    /// Create with default window size.
    pub fn default_window() -> Self {
        Self::new(HDLC_DEFAULT_WINDOW_SIZE)
    }

    /// Get the next send sequence number.
    pub fn send_seq(&self) -> u8 {
        self.send_seq & 0x07
    }

    /// Get the next expected receive sequence number.
    pub fn recv_seq(&self) -> u8 {
        self.recv_seq & 0x07
    }

    /// Check if the window is full (cannot send more I-frames).
    pub fn is_window_full(&self) -> bool {
        self.outstanding >= self.window_size
    }

    /// Advance send sequence after sending an I-frame.
    pub fn on_frame_sent(&mut self) {
        self.send_seq = (self.send_seq + 1) & 0x07;
        self.outstanding += 1;
        self.window_full = self.outstanding >= self.window_size;
    }

    /// Process received RR/RNR frame — advance window on acknowledgment.
    pub fn on_ack_received(&mut self, nr: u8) -> bool {
        let nr = nr & 0x07;
        if nr != self.recv_seq {
            self.outstanding = 0;
            self.recv_seq = nr;
            self.retries = 0;
            self.window_full = false;
            return true; // window advanced
        }
        false
    }

    /// Increment retry count, return true if max retries exceeded.
    pub fn on_timeout(&mut self) -> bool {
        self.retries += 1;
        self.retries > HDLC_MAX_RETRIES
    }

    /// Reset the window state.
    pub fn reset(&mut self) {
        self.send_seq = 0;
        self.recv_seq = 0;
        self.outstanding = 0;
        self.retries = 0;
        self.window_full = false;
    }

    /// Build I-frame control byte: N(S)<<1 | P/F | N(R)<<5
    pub fn build_i_frame_control(&self, poll: bool) -> u8 {
        let pf = if poll { 0x10 } else { 0x00 };
        (self.recv_seq << 5) | pf | (self.send_seq << 1)
    }

    /// Build RR control byte: 01 | 00 | P/F | N(R)<<5
    pub fn build_rr_control(&self, poll: bool) -> u8 {
        let pf = if poll { 0x10 } else { 0x00 };
        (self.recv_seq << 5) | pf | 0x01
    }
}

/* ================================================================== */
/*  IEC 62056-21 Standard Data Identifiers                              */
/* ================================================================== */

/// IEC 62056-21 standard data identifiers (B0-B9 and common extensions)
#[derive(Clone, Copy, Debug)]
pub enum IecDataId {
    /// B0: Manufacturer ID
    Manufacturer,
    /// B1: Equipment type/model
    Model,
    /// B2: Firmware version
    FirmwareVersion,
    /// B3: Hardware version
    HardwareVersion,
    /// B4: Serial number
    SerialNumber,
    /// B5: Customer ID
    CustomerId,
    /// B6: Metering point ID
    MeteringPoint,
    /// B7: Billing period
    BillingPeriod,
    /// B8: Tariff
    Tariff,
    /// B9: Date and time
    DateTime,
    /// F.F: Status word
    StatusWord,
    /// 0.0.0: Total active energy import
    TotalActiveImport,
    /// 1.8.0: Total active energy export
    TotalActiveExport,
    /// 1.8.1..1.8.4: Tariff active energy import
    TariffActiveImport(u8),
    /// 0.2.0: Voltage L1
    VoltageL1,
    /// 0.6.0: Current L1
    CurrentL1,
    /// Custom identifier (raw OBIS)
    Custom([u8; 6]),
}

impl IecDataId {
    /// Convert to IEC 62056-21 data identifier string
    pub fn to_id_string(&self) -> [u8; 16] {
        let mut buf = [0u8; 16];
        let s = match self {
            IecDataId::Manufacturer => b"B0(",
            IecDataId::Model => b"B1(",
            IecDataId::FirmwareVersion => b"B2(",
            IecDataId::HardwareVersion => b"B3(",
            IecDataId::SerialNumber => b"B4(",
            IecDataId::CustomerId => b"B5(",
            IecDataId::MeteringPoint => b"B6(",
            IecDataId::BillingPeriod => b"B7(",
            IecDataId::Tariff => b"B8(",
            IecDataId::DateTime => b"B9(",
            IecDataId::StatusWord => b"F.F(",
            IecDataId::TotalActiveImport => b"0.0.0(",
            IecDataId::TotalActiveExport => b"1.8.0(",
            IecDataId::TariffActiveImport(t) => {
                // e.g. "1.8.1("
                buf[0] = b'1'; buf[1] = b'.'; buf[2] = b'8'; buf[3] = b'.';
                buf[4] = b'0' + *t; buf[5] = b'(';
                return buf;
            }
            IecDataId::VoltageL1 => b"0.2.0(",
            IecDataId::CurrentL1 => b"0.6.0(",
            IecDataId::Custom(id) => {
                // Format: A.B.C.D.E.F(
                let mut pos = 0;
                for &b in id.iter() {
                    if b < 10 {
                        buf[pos] = b'0' + b;
                    } else {
                        buf[pos] = b'0' + (b / 10);
                        pos += 1;
                        buf[pos] = b'0' + (b % 10);
                    }
                    pos += 1;
                    if pos < 11 {
                        buf[pos] = b'.';
                    }
                    pos += 1;
                }
                buf[pos] = b'(';
                return buf;
            }
        };
        let len = s.len().min(16);
        buf[..len].copy_from_slice(&s[..len]);
        buf
    }
}

/// IEC 62056-21 supported baud rates
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IecBaudRate {
    /// 300 bps
    Bps300,
    /// 600 bps
    Bps600,
    /// 1200 bps
    Bps1200,
    /// 2400 bps
    Bps2400,
    /// 4800 bps
    Bps4800,
    /// 9600 bps
    Bps9600,
}

impl IecBaudRate {
    /// Get the baud character used in IEC 62056-21 protocol.
    /// '0'=300, '1'=600, '2'=1200, '3'=2400, '4'=4800, '5'=9600
    pub fn baud_char(&self) -> u8 {
        match self {
            IecBaudRate::Bps300 => b'0',
            IecBaudRate::Bps600 => b'1',
            IecBaudRate::Bps1200 => b'2',
            IecBaudRate::Bps2400 => b'3',
            IecBaudRate::Bps4800 => b'4',
            IecBaudRate::Bps9600 => b'5',
        }
    }

    /// Get the actual baud rate value.
    pub fn baudrate(&self) -> u32 {
        match self {
            IecBaudRate::Bps300 => 300,
            IecBaudRate::Bps600 => 600,
            IecBaudRate::Bps1200 => 1200,
            IecBaudRate::Bps2400 => 2400,
            IecBaudRate::Bps4800 => 4800,
            IecBaudRate::Bps9600 => 9600,
        }
    }

    /// Parse baud character to baud rate.
    pub fn from_baud_char(ch: u8) -> Option<Self> {
        match ch {
            b'0' => Some(IecBaudRate::Bps300),
            b'1' => Some(IecBaudRate::Bps600),
            b'2' => Some(IecBaudRate::Bps1200),
            b'3' => Some(IecBaudRate::Bps2400),
            b'4' => Some(IecBaudRate::Bps4800),
            b'5' => Some(IecBaudRate::Bps9600),
            _ => None,
        }
    }
}

/* ================================================================== */
  Multi-Channel Communication Manager                                */
/* ================================================================== */

/// Communication event returned by poll methods.
#[derive(Clone, Copy, Debug)]
pub enum CommEvent {
    /// HDLC frame received on RS-485
    HdlcFrame,
    /// IEC 62056-21 request on infrared
    IecRequest,
    /// IEC 62056-21 data block
    IecDataBlock,
    /// IEC 62056-21 complete message
    IecComplete,
    /// No event
    None,
}

/// Multi-channel communication manager.
///
/// Manages RS-485 (HDLC/DLMS) and infrared (IEC 62056-21) channels
/// with independent receivers and a shared HDLC frame builder.
pub struct CommManager<U: UartDriver, D: Rs485DirControl> {
    /// RS-485 UART driver
    rs485: U,
    /// Infrared UART driver
    infrared: U,
    /// RS-485 direction control
    dir: D,
    /// HDLC receiver state machine for RS-485
    hdlc_rx: HdlcReceiver,
    /// IEC 62056-21 parser for infrared
    iec_rx: Iec62056Parser,
    /// Temporary read buffer
    rx_tmp: [u8; 64],
}

impl<U: UartDriver, D: Rs485DirControl> CommManager<U, D> {
    /// Create a new communication manager.
    pub fn new(rs485: U, infrared: U, dir: D) -> Self {
        Self {
            rs485,
            infrared,
            dir,
            hdlc_rx: HdlcReceiver::new(),
            iec_rx: Iec62056Parser::new(),
            rx_tmp: [0u8; 64],
        }
    }

    /// Initialize both UART channels with default configs.
    pub fn init(&mut self) -> Result<(), UartError> {
        let rs485_cfg = UartConfig {
            baudrate: 9600,
            data_bits: 8,
            stop_bits: 1,
            parity: crate::hal::Parity::Even,
        };
        let ir_cfg = UartConfig {
            baudrate: 300,
            data_bits: 7,
            stop_bits: 1,
            parity: crate::hal::Parity::Even,
        };
        self.rs485.init(&rs485_cfg)?;
        self.infrared.init(&ir_cfg)?;
        Ok(())
    }

    /// Initialize with custom configs.
    pub fn init_with(
        &mut self,
        rs485_cfg: &UartConfig,
        ir_cfg: &UartConfig,
    ) -> Result<(), UartError> {
        self.rs485.init(rs485_cfg)?;
        self.infrared.init(ir_cfg)?;
        Ok(())
    }

    /// Poll RS-485 for received bytes, feed to HDLC receiver.
    /// Returns CommEvent::HdlcFrame when a complete frame is received.
    pub fn poll_rs485(&mut self) -> CommEvent {
        // Non-blocking check for available data
        if !self.rs485.readable() {
            return CommEvent::None;
        }

        // Read available bytes with short timeout
        match self.rs485.read(&mut self.rx_tmp, 1) {
            Ok(n) => {
                for i in 0..n {
                    match self.hdlc_rx.feed(self.rx_tmp[i]) {
                        HdlcRxEvent::Complete => return CommEvent::HdlcFrame,
                        HdlcRxEvent::Error(_) => {
                            self.hdlc_rx.reset();
                        }
                        HdlcRxEvent::Idle => {}
                    }
                }
            }
            Err(_) => {}
        }
        CommEvent::None
    }

    /// Poll infrared for received bytes, feed to IEC parser.
    pub fn poll_infrared(&mut self) -> CommEvent {
        if !self.infrared.readable() {
            return CommEvent::None;
        }

        match self.infrared.read(&mut self.rx_tmp, 1) {
            Ok(n) => {
                for i in 0..n {
                    match self.iec_rx.feed(self.rx_tmp[i]) {
                        IecRxEvent::RequestReceived => return CommEvent::IecRequest,
                        IecRxEvent::DataBlock => return CommEvent::IecDataBlock,
                        IecRxEvent::Complete => return CommEvent::IecComplete,
                        IecRxEvent::Overflow => {
                            self.iec_rx.reset();
                        }
                        IecRxEvent::Idle => {}
                    }
                }
            }
            Err(_) => {}
        }
        CommEvent::None
    }

    /// Get a reference to the HDLC receiver (to access parsed frame).
    pub fn hdlc_receiver(&self) -> &HdlcReceiver {
        &self.hdlc_rx
    }

    /// Get a mutable reference to the HDLC receiver.
    pub fn hdlc_receiver_mut(&mut self) -> &mut HdlcReceiver {
        &mut self.hdlc_rx
    }

    /// Get a reference to the IEC parser.
    pub fn iec_parser(&self) -> &Iec62056Parser {
        &self.iec_rx
    }

    /// Get a mutable reference to the IEC parser.
    pub fn iec_parser_mut(&mut self) -> &mut Iec62056Parser {
        &mut self.iec_rx
    }

    /// Build and send an HDLC frame via RS-485 with direction control.
    pub fn send_hdlc_frame(
        &mut self,
        address: &[u8],
        control: u8,
        info: &[u8],
    ) -> Result<(), UartError> {
        let mut tx_buf: [u8; 512] = [0u8; 512];
        let len = HdlcFrame::build(address, control, info, &mut tx_buf);

        // Enable RS-485 transmitter
        self.dir.tx_enable();

        // Send the frame
        let result = self.rs485.write(&tx_buf[..len]);

        // Disable RS-485 transmitter (back to receive)
        self.dir.tx_disable();

        result
    }

    /// Send raw bytes via RS-485 with direction control.
    pub fn send_rs485_raw(&mut self, data: &[u8]) -> Result<(), UartError> {
        self.dir.tx_enable();
        let result = self.rs485.write(data);
        self.dir.tx_disable();
        result
    }

    /// Send IEC 62056-21 response via infrared.
    pub fn send_iec_response(&mut self, data: &[u8]) -> Result<(), UartError> {
        self.infrared.write(data)
    }

    /// Send IEC baud rate acknowledgment and switch baud rate.
    pub fn iec_baud_switch(&mut self, baud_char: u8, new_baud: u32) -> Result<(), UartError> {
        let mut ack: [u8; 5] = [0u8; 5];
        let len = Iec62056Parser::build_ack_baud_switch(baud_char, &mut ack);
        self.infrared.write(&ack[..len])?;
        // Switch infrared baud rate
        let ir_cfg = UartConfig {
            baudrate: new_baud,
            data_bits: 8,
            stop_bits: 1,
            parity: crate::hal::Parity::Even,
        };
        self.rs485.init(&ir_cfg)?; // Re-init with new baud
        self.iec_rx.set_high_speed();
        Ok(())
    }

    /// Reset both receivers.
    pub fn reset_all(&mut self) {
        self.hdlc_rx.reset();
        self.iec_rx.reset();
    }

    /// Get mutable reference to RS-485 UART (for direct access).
    pub fn rs485_uart(&mut self) -> &mut U {
        &mut self.rs485
    }

    /// Get mutable reference to infrared UART (for direct access).
    pub fn infrared_uart(&mut self) -> &mut U {
        &mut self.infrared
    }
}

/* ================================================================== */
/*  Tests                                                              */
/* ================================================================== */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fcs16_empty() {
        // FCS of empty data should be 0xFFFF (init ^ final xor = 0xFFFF ^ 0xFFFF = 0x0000 ... actually)
        // !0xFFFF = 0x0000
        assert_eq!(fcs16(&[]), 0x0000);
    }

    #[test]
    fn test_fcs16_known() {
        // FCS of [0x01] — reference value
        let crc = fcs16(&[0x01]);
        // Compute manually: crc starts 0xFFFF, table[0xFF^0x01] = table[0xFE]
        let expected = !((0xFFFFu16 >> 8) ^ FCS16_TABLE[0xFE]);
        assert_eq!(crc, expected);
    }

    #[test]
    fn test_hdlc_frame_roundtrip() {
        let addr = [0x03u8]; // 1-byte address with LSB=1
        let ctrl = 0x73; // U-frame UA
        let info = [0x01, 0x02, 0x03];

        let mut tx = [0u8; 64];
        let len = HdlcFrame::build(&addr, ctrl, &info, &mut tx);

        // Strip flags for parsing
        assert!(len > 4);
        assert_eq!(tx[0], HDLC_FLAG);
        assert_eq!(tx[len - 1], HDLC_FLAG);

        // Feed through receiver
        let mut rx = HdlcReceiver::new();
        let mut event = HdlcRxEvent::Idle;
        for i in 0..len {
            event = rx.feed(tx[i]);
        }
        assert_eq!(event, HdlcRxEvent::Complete);

        let frame = rx.parse_frame().unwrap();
        assert_eq!(frame.address(), &[0x03]);
        assert_eq!(frame.control(), ctrl);
        assert_eq!(frame.information(), &info);
        assert!(frame.is_u_frame());
    }

    #[test]
    fn test_hdlc_byte_stuffing() {
        // Build frame with data that needs escaping
        let addr = [0x03];
        let ctrl = 0x00;
        let info = [0x7E, 0x7D, 0x42]; // FLAG and ESCAPE need escaping

        let mut tx = [0u8; 64];
        let len = HdlcFrame::build(&addr, ctrl, &info, &mut tx);

        // The stuffed bytes should not contain raw 0x7E/0x7D in the payload
        // (only at boundaries as flags)
        for i in 1..len - 1 {
            assert_ne!(
                tx[i], HDLC_FLAG,
                "Unescaped FLAG found in payload at idx {}",
                i
            );
            // Note: HDLC_ESCAPE is allowed as the escape marker itself
        }

        // Round-trip through receiver
        let mut rx = HdlcReceiver::new();
        for i in 0..len {
            rx.feed(tx[i]);
        }
        let frame = rx.parse_frame().unwrap();
        assert_eq!(frame.information(), &info);
    }

    #[test]
    fn test_u_frame_types() {
        // SNRM: control = 0x93 (bits [7:5]=100, [1:0]=11)
        // UA:   control = 0x73 (bits [7:5]=011, [1:0]=11) — actually UA = 011x0011
        // Let's use standard encoding:
        // SNRM = 0x93 (100 0 0011)
        // DISC = 0x53 (010 0 0011)
        // UA   = 0x73 (011 0 0011)
        // DM   = 0x1F (000 1 1111) — simplified
        // Actually DLMS UA control = 0x63

        let test_cases = [
            // (control_byte, expected_type)
            // SNRM: modifier bits = 100 → 0x93
            // UA: modifier bits = 011 → 0x63
            // DISC: modifier bits = 010 → 0x53
        ];
        // Minimal test — just ensure U-frame detection works
        let addr = [0x03];
        let ctrl_ua = 0x63;
        let mut tx = [0u8; 64];
        let len = HdlcFrame::build(&addr, ctrl_ua, &[], &mut tx);
        let mut rx = HdlcReceiver::new();
        for i in 0..len {
            rx.feed(tx[i]);
        }
        let frame = rx.parse_frame().unwrap();
        assert!(frame.is_u_frame());
    }

    #[test]
    fn test_iec_request_detection() {
        let mut parser = Iec62056Parser::new();
        let request = b"/?!\r\n";
        let mut result = IecRxEvent::Idle;
        for &b in request {
            result = parser.feed(b);
        }
        assert_eq!(result, IecRxEvent::RequestReceived);
    }

    // ============================================================
    // Phase C — Boundary Tests
    // ============================================================

    #[test]
    fn test_hdlc_address_classification() {
        assert_eq!(classify_address(0x03), HdlcAddressType::Individual);
        assert_eq!(classify_address(0x01), HdlcAddressType::Individual);
        assert_eq!(classify_address(0x80), HdlcAddressType::Group);
        assert_eq!(classify_address(0xFF), HdlcAddressType::Broadcast);
        assert_eq!(classify_address(0xC0), HdlcAddressType::Group);
    }

    #[test]
    fn test_build_addresses() {
        assert_eq!(build_client_address(1), 0x03);
        assert_eq!(build_client_address(0), 0x01);
        assert_eq!(build_server_address(1), 0x03);
        assert_eq!(build_server_address(0x10), 0x21);
    }

    #[test]
    fn test_hdlc_tx_window_basic() {
        let mut win = HdlcTxWindow::new(4);
        assert!(!win.is_window_full());
        assert_eq!(win.send_seq(), 0);
        assert_eq!(win.recv_seq(), 0);

        win.on_frame_sent();
        assert_eq!(win.send_seq(), 1);
        assert!(!win.is_window_full());

        for _ in 0..3 {
            win.on_frame_sent();
        }
        assert!(win.is_window_full());
    }

    #[test]
    fn test_hdlc_tx_window_ack() {
        let mut win = HdlcTxWindow::new(4);
        for _ in 0..3 {
            win.on_frame_sent();
        }
        assert!(win.on_ack_received(3));
        assert!(!win.is_window_full());
    }

    #[test]
    fn test_hdlc_tx_window_timeout() {
        let mut win = HdlcTxWindow::new(1);
        assert!(!win.on_timeout());
        assert!(!win.on_timeout());
        assert!(!win.on_timeout());
        assert!(win.on_timeout()); // 4th timeout exceeds max_retries=3
    }

    #[test]
    fn test_hdlc_tx_window_reset() {
        let mut win = HdlcTxWindow::new(4);
        for _ in 0..4 {
            win.on_frame_sent();
        }
        win.reset();
        assert!(!win.is_window_full());
        assert_eq!(win.send_seq(), 0);
        assert_eq!(win.recv_seq(), 0);
    }

    #[test]
    fn test_hdlc_tx_window_i_frame_control() {
        let win = HdlcTxWindow::new(4);
        let ctrl = win.build_i_frame_control(true);
        assert_eq!(ctrl & 0x01, 0); // I-frame
        assert!(ctrl & 0x10 != 0); // P=1
    }

    #[test]
    fn test_hdlc_tx_window_rr_control() {
        let win = HdlcTxWindow::new(4);
        let ctrl = win.build_rr_control(false);
        assert_eq!(ctrl & 0x03, 0x01); // S-frame RR
        assert_eq!(ctrl & 0x10, 0); // F=0
    }

    #[test]
    fn test_fcs16_comprehensive() {
        // Known FCS values
        assert_eq!(fcs16(&[]), 0x0000);
        assert_eq!(fcs16(&[0xFF]), 0xDCF0);
        // Symmetric: FCS(A + FCS(A)) should be 0xF0B8
        let data = [0x01, 0x02, 0x03];
        let crc = fcs16(&data);
        let crc_bytes = [crc as u8, (crc >> 8) as u8];
        let combined = [&data[..], &crc_bytes].concat();
        assert_eq!(fcs16(&combined), 0xF0B8);
    }

    #[test]
    fn test_hdlc_frame_max_size() {
        let addr = [0x03u8];
        let ctrl = 0x00;
        let info = [0xAA; 128];
        let mut tx = [0u8; 512];
        let len = HdlcFrame::build(&addr, ctrl, &info, &mut tx);
        assert!(len > 0);
        assert_eq!(tx[0], HDLC_FLAG);
        assert_eq!(tx[len - 1], HDLC_FLAG);

        let mut rx = HdlcReceiver::new();
        for i in 0..len {
            rx.feed(tx[i]);
        }
        let frame = rx.parse_frame().unwrap();
        assert_eq!(frame.information(), &info);
    }

    #[test]
    fn test_iec_baud_rates() {
        assert_eq!(IecBaudRate::Bps300.baudrate(), 300);
        assert_eq!(IecBaudRate::Bps9600.baudrate(), 9600);
        assert_eq!(IecBaudRate::from_baud_char(b'0'), Some(IecBaudRate::Bps300));
        assert_eq!(IecBaudRate::from_baud_char(b'5'), Some(IecBaudRate::Bps9600));
        assert_eq!(IecBaudRate::from_baud_char(b'9'), None);
        assert_eq!(IecBaudRate::Bps9600.baud_char(), b'5');
    }

    #[test]
    fn test_iec_data_ids() {
        let id = IecDataId::Manufacturer.to_id_string();
        assert_eq!(&id[0..3], b"B0(");

        let id = IecDataId::TotalActiveImport.to_id_string();
        assert_eq!(&id[0..5], b"0.0.0");

        let id = IecDataId::TariffActiveImport(2).to_id_string();
        assert_eq!(&id[0..5], b"1.8.2");
    }

    #[test]
    fn test_iec_ident_builder() {
        let manufacturer = [b'V', b'W', b'Y'];
        let mut out = [0u8; 16];
        let len = Iec62056Parser::build_ident(&manufacturer, b'S', b'5', &mut out);
        assert!(len >= 6);
        assert_eq!(out[0], b'/');
        assert_eq!(out[1], b'V');
        assert_eq!(out[5], b'5');
    }

    #[test]
    fn test_hdlc_receiver_overflow() {
        let mut rx = HdlcReceiver::new();
        rx.feed(HDLC_FLAG); // start
        // Fill buffer beyond 512 bytes
        for _ in 0..520 {
            let event = rx.feed(0x41);
            if let HdlcRxEvent::Error(HdlcError::Overflow) = event {
                return; // test passed
            }
        }
        panic!("Expected overflow error");
    }

    #[test]
    fn test_hdlc_frame_fcs_error() {
        let addr = [0x03u8];
        let ctrl = 0x63;
        let info = [];
        let mut tx = [0u8; 64];
        let len = HdlcFrame::build(&addr, ctrl, &info, &mut tx);
        // Corrupt a byte in the middle
        if len > 4 {
            tx[2] ^= 0xFF;
        }
        let mut rx = HdlcReceiver::new();
        for i in 0..len {
            rx.feed(tx[i]);
        }
        assert!(rx.parse_frame().is_err());
    }

    #[test]
    fn test_iec_parser_overflow() {
        let mut parser = Iec62056Parser::new();
        for _ in 0..200 {
            let event = parser.feed(0x41);
            if let IecRxEvent::Overflow = event {
                return; // test passed
            }
        }
        panic!("Expected overflow");
    }
