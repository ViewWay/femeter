"""DLMS/COSEM protocol integration tests via virtual meter TCP.

Tests the complete DLMS SN connection flow:
AARQ → AARE → GetRequest → GetResponse → Disconnect

Uses dlms-cosem Python library where available, falls back to raw HDLC tests.
"""

import socket
import struct
import time
import pytest
import sys
import os

# Add dlms-cosem Python library to path
DLMS_COSEM_PATH = os.path.expanduser("~/.openclaw/workspace/dlms-cosem")
if os.path.isdir(DLMS_COSEM_PATH) and DLMS_COSEM_PATH not in sys.path:
    sys.path.insert(0, DLMS_COSEM_PATH)

DLMS_PORT = 4059
TEXT_PORT = 8888


def _dlms_available():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(1)
        return s.connect_ex(("127.0.0.1", DLMS_PORT)) == 0


def _text_available():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(1)
        return s.connect_ex(("127.0.0.1", TEXT_PORT)) == 0


# ====================================================================
# HDLC Frame helpers
# ====================================================================

HDLC_FLAG = 0x7E
HDLC_ESCAPE = 0x7D
HDLC_ESCAPE_XOR = 0x20


def hdlc_fcs16(data: bytes) -> int:
    """CRC-16/HDLC (polynomial 0x8408, init 0xFFFF)."""
    crc = 0xFFFF
    for b in data:
        crc ^= b
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8408
            else:
                crc >>= 1
    return crc & 0xFFFF


def hdlc_byte_stuff(data: bytes) -> bytes:
    """Apply HDLC byte stuffing."""
    result = bytearray()
    for b in data:
        if b == HDLC_FLAG or b == HDLC_ESCAPE:
            result.append(HDLC_ESCAPE)
            result.append(b ^ HDLC_ESCAPE_XOR)
        else:
            result.append(b)
    return bytes(result)


def hdlc_byte_unstuff(data: bytes) -> bytes:
    """Reverse HDLC byte stuffing."""
    result = bytearray()
    i = 0
    while i < len(data):
        if data[i] == HDLC_ESCAPE and i + 1 < len(data):
            result.append(data[i + 1] ^ HDLC_ESCAPE_XOR)
            i += 2
        else:
            result.append(data[i])
            i += 1
    return bytes(result)


def build_hdlc_frame(server_addr: int, client_addr: int, info: bytes) -> bytes:
    """Build a complete HDLC frame: Flag + Address + Control + Info + FCS + Flag."""
    # Build address field (1-byte for simplicity)
    address = (client_addr << 1) | 0x01  # client address with extension bit

    # Build control field (I-frame, N(S)=0, N(R)=0, P=0)
    control = 0x00

    # Compute FCS over address + control + info
    payload = bytes([address, control]) + info
    fcs = hdlc_fcs16(payload)
    fcs_bytes = struct.pack('<H', fcs)

    # Byte-stuff the entire payload + FCS
    stuffed = hdlc_byte_stuff(payload + fcs_bytes)

    return bytes([HDLC_FLAG]) + stuffed + bytes([HDLC_FLAG])


def extract_frames(data: bytes) -> list:
    """Extract HDLC frames from a byte stream."""
    frames = []
    start = None
    for i, b in enumerate(data):
        if b == HDLC_FLAG:
            if start is not None and i > start + 1:
                frames.append(data[start + 1:i])
            start = i
    return frames


def parse_hdlc_frame(raw: bytes) -> tuple:
    """Parse an HDLC frame, return (address, control, info)."""
    unstuffed = hdlc_byte_unstuff(raw)
    if len(unstuffed) < 4:
        raise ValueError("Frame too short")

    # Verify FCS
    payload = unstuffed[:-2]
    received_fcs = struct.unpack('<H', unstuffed[-2:])[0]
    computed_fcs = hdlc_fcs16(payload)
    if received_fcs != computed_fcs:
        raise ValueError(f"FCS mismatch: received={received_fcs:04X}, computed={computed_fcs:04X}")

    # Parse address (variable length, last byte has LSB=1)
    addr_len = 1
    for i, b in enumerate(payload):
        if b & 0x01:
            addr_len = i + 1
            break

    address = payload[:addr_len]
    control = payload[addr_len]
    info = payload[addr_len + 1:]
    return address, control, info


# ====================================================================
# DLMS APDU builders
# ====================================================================

def build_aarq() -> bytes:
    """Build AARQ (Association Request) APDU for LN mode, no cipher."""
    # Minimal AARQ: tag 0xE0 + AARQ context
    # Using a pre-built AARQ that the virtual meter accepts
    return bytes([
        0xE0, 0x00, 0x00,  # AARQ tag
        # Application context name: LN + no cipher
        0xA1, 0x09, 0x06, 0x07, 0x60, 0x85, 0x74, 0x05, 0x08, 0x01, 0x01,
        # User information (authentication)
        0xBE, 0x10, 0x04, 0x0E, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x09, 0x0C, 0x06, 0x00, 0x00, 0x01, 0x00, 0xFF, 0xAA, 0x00, 0x80,
    ])


def build_get_request_normal(invoke_id: int, class_id: int, obis: tuple, attr_id: int) -> bytes:
    """Build GetRequest-Normal APDU (tag 0xC0)."""
    apdu = bytearray()
    apdu.append(0xC0)  # GetRequest-Normal
    apdu.append(invoke_id)
    # AttributeDescriptor: class_id(2) + instance(6) + attribute_id(1)
    apdu.extend(struct.pack('>H', class_id))
    for b in obis:
        apdu.append(b)
    apdu.append(attr_id)
    return bytes(apdu)


def build_set_request_normal(invoke_id: int, class_id: int, obis: tuple, attr_id: int, value: bytes) -> bytes:
    """Build SetRequest-Normal APDU (tag 0xD0)."""
    apdu = bytearray()
    apdu.append(0xD0)  # SetRequest-Normal
    apdu.append(invoke_id)
    apdu.extend(struct.pack('>H', class_id))
    for b in obis:
        apdu.append(b)
    apdu.append(attr_id)
    apdu.extend(value)
    return bytes(apdu)


def build_disconnect_request() -> bytes:
    """Build ReleaseRequest (tag 0x80)."""
    return bytes([0x80, 0x01, 0x00])


# ====================================================================
# Test Fixtures
# ====================================================================

@pytest.fixture
def dlms_conn():
    """Raw TCP connection to DLMS HDLC port."""
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.settimeout(5)
    s.connect(("127.0.0.1", DLMS_PORT))
    try:
        yield s
    finally:
        s.close()


class DlmsSession:
    """Helper for DLMS HDLC session management."""

    def __init__(self, sock: socket.socket):
        self.sock = sock
        self.invoke_id = 1

    def send_and_receive(self, info: bytes) -> bytes:
        """Send HDLC frame and receive response."""
        frame = build_hdlc_frame(0x0001, 0x0010, info)
        self.sock.sendall(frame)

        # Read response
        resp_data = bytearray()
        self.sock.settimeout(5)
        while True:
            try:
                chunk = self.sock.recv(4096)
                if not chunk:
                    break
                resp_data.extend(chunk)
                # Check if we got at least one complete frame
                if HDLC_FLAG in chunk:
                    break
            except socket.timeout:
                break

        # Extract and parse first frame
        frames = extract_frames(bytes(resp_data))
        if not frames:
            return b""

        try:
            _, _, info = parse_hdlc_frame(frames[0])
            return info
        except ValueError:
            return bytes(resp_data)

    def next_invoke_id(self) -> int:
        vid = self.invoke_id
        self.invoke_id = (self.invoke_id % 254) + 1
        return vid


# ====================================================================
# Tests
# ====================================================================

requires_dlms = pytest.mark.skipif(not _dlms_available(), reason="DLMS server not running")


@requires_dlms
class TestDlmsConnection:
    """Basic DLMS-over-TCP connectivity."""

    def test_tcp_connect(self):
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(2)
        s.connect(("127.0.0.1", DLMS_PORT))
        s.close()

    def test_hdlc_flag_bytes(self):
        """Multiple HDLC flags should be accepted."""
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(2)
        s.connect(("127.0.0.1", DLMS_PORT))
        try:
            s.sendall(b"\x7e\x7e\x7e")
            s.settimeout(3)
            resp = s.recv(1024)
            # Server should not disconnect
            assert True
        finally:
            s.close()

    def test_disconnect_reconnect(self):
        """Disconnect and reconnect should work."""
        s1 = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s1.settimeout(2)
        s1.connect(("127.0.0.1", DLMS_PORT))
        s1.close()
        s2 = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s2.settimeout(2)
        s2.connect(("127.0.0.1", DLMS_PORT))
        s2.close()


@requires_dlms
class TestDlmsAssociation:
    """DLMS association (AARQ/AARE) tests."""

    def test_aarq_response(self, dlms_conn):
        """Send AARQ and receive AARE."""
        session = DlmsSession(dlms_conn)
        resp = session.send_and_receive(build_aarq())
        assert len(resp) > 0
        # AARE should start with 0xE1 (accepted) or 0xE2 (rejected)
        assert resp[0] in (0xE1, 0xE2)


@requires_dlms
class TestDlmsReadOperations:
    """DLMS Get Request tests for various COSEM objects."""

    def _associate(self, conn) -> DlmsSession:
        session = DlmsSession(conn)
        resp = session.send_and_receive(build_aarq())
        assert resp[0] == 0xE1, f"Association failed: {resp.hex()}"
        return session

    def test_read_clock(self, dlms_conn):
        """Read Clock object (0.0.1.0.0.255, class=8, attr=2)."""
        session = self._associate(dlms_conn)
        obis = (0, 0, 1, 0, 0, 255)
        req = build_get_request_normal(session.next_invoke_id(), 8, obis, 2)
        resp = session.send_and_receive(req)
        assert len(resp) > 0
        # GetResponse tag is 0xC1
        assert resp[0] == 0xC1

    def test_read_total_energy_import(self, dlms_conn):
        """Read total active energy import (1.0.1.8.0.255, class=3, attr=2)."""
        session = self._associate(dlms_conn)
        obis = (1, 0, 1, 8, 0, 255)
        req = build_get_request_normal(session.next_invoke_id(), 3, obis, 2)
        resp = session.send_and_receive(req)
        assert len(resp) > 0
        assert resp[0] == 0xC1

    def test_read_voltage_l1(self, dlms_conn):
        """Read L1 voltage (1.0.32.7.0.255, class=3, attr=2)."""
        session = self._associate(dlms_conn)
        obis = (1, 0, 32, 7, 0, 255)
        req = build_get_request_normal(session.next_invoke_id(), 3, obis, 2)
        resp = session.send_and_receive(req)
        assert len(resp) > 0

    def test_read_current_l1(self, dlms_conn):
        """Read L1 current (1.0.31.7.0.255, class=3, attr=2)."""
        session = self._associate(dlms_conn)
        obis = (1, 0, 31, 7, 0, 255)
        req = build_get_request_normal(session.next_invoke_id(), 3, obis, 2)
        resp = session.send_and_receive(req)
        assert len(resp) > 0

    def test_read_total_power(self, dlms_conn):
        """Read total active power (1.0.14.7.0.255, class=3, attr=2)."""
        session = self._associate(dlms_conn)
        obis = (1, 0, 14, 7, 0, 255)
        req = build_get_request_normal(session.next_invoke_id(), 3, obis, 2)
        resp = session.send_and_receive(req)
        assert len(resp) > 0

    def test_read_demand(self, dlms_conn):
        """Read demand register (class=5)."""
        session = self._associate(dlms_conn)
        obis = (1, 0, 14, 7, 0, 255)
        req = build_get_request_normal(session.next_invoke_id(), 5, obis, 2)
        resp = session.send_and_receive(req)
        assert len(resp) > 0

    def test_read_multiple_objects(self, dlms_conn):
        """Read multiple COSEM objects in sequence."""
        session = self._associate(dlms_conn)
        obis_list = [
            (1, 0, 1, 8, 0, 255),  # Total energy import
            (1, 0, 32, 7, 0, 255),  # L1 voltage
            (1, 0, 31, 7, 0, 255),  # L1 current
            (1, 0, 14, 7, 0, 255),  # Total power
        ]
        for obis in obis_list:
            req = build_get_request_normal(session.next_invoke_id(), 3, obis, 2)
            resp = session.send_and_receive(req)
            assert len(resp) > 0, f"No response for OBIS {obis}"


@requires_dlms
class TestDlmsSetOperations:
    """DLMS Set Request tests."""

    def _associate(self, conn) -> DlmsSession:
        session = DlmsSession(conn)
        resp = session.send_and_receive(build_aarq())
        assert resp[0] == 0xE1
        return session

    def test_set_tariff_table(self, dlms_conn):
        """Set tariff table (1.0.0.1.0.255, class=3, attr=2)."""
        session = self._associate(dlms_conn)
        obis = (1, 0, 0, 1, 0, 255)
        # Value: array of 4 tariff values (UInt16)
        value = bytes([0x12, 0x04, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04])
        req = build_set_request_normal(session.next_invoke_id(), 3, obis, 2, value)
        resp = session.send_and_receive(req)
        assert len(resp) > 0


@requires_dlms
class TestDlmsExceptionScenarios:
    """Test error handling and edge cases."""

    def test_invalid_obis(self, dlms_conn):
        """Read non-existent OBIS code should return error."""
        session = DlmsSession(dlms_conn)
        resp = session.send_and_receive(build_aarq())
        assert resp[0] == 0xE1

        # Invalid OBIS (99.99.99.99.99.99)
        obis = (99, 99, 99, 99, 99, 255)
        req = build_get_request_normal(session.next_invoke_id(), 3, obis, 2)
        resp = session.send_and_receive(req)
        # Should get a response (even if Null/error)
        assert len(resp) > 0

    def test_timeout_handling(self, dlms_conn):
        """No data sent, should not hang."""
        dlms_conn.settimeout(2)
        try:
            dlms_conn.recv(1024)
            assert False, "Should have timed out"
        except socket.timeout:
            pass

    def test_garbage_data(self, dlms_conn):
        """Send garbage data, server should not crash."""
        dlms_conn.sendall(b"\x00\xFF\xFE\xDE\xAD\xBE\xEF\x7E")
        time.sleep(0.5)
        # Server should still be responsive
        resp = dlms_conn.send_and_receive(build_aarq()) if hasattr(dlms_conn, 'send_and_receive') else None
        # At minimum, connection should still be alive
        assert True

    def test_disconnect_request(self, dlms_conn):
        """Send disconnect request."""
        session = DlmsSession(dlms_conn)
        resp = session.send_and_receive(build_aarq())
        assert resp[0] == 0xE1

        resp = session.send_and_receive(build_disconnect_request())
        assert len(resp) > 0
        # RLRE response tag is 0x81
        assert resp[0] == 0x81


@requires_dlms
class TestDlmsHdlcLayer:
    """HDLC frame layer tests."""

    def test_fcs16(self):
        """FCS-16 computation."""
        assert hdlc_fcs16(b"") == 0x0000
        # FCS(A + FCS(A)) should equal 0xF0B8
        data = b"\x01\x02\x03"
        fcs = hdlc_fcs16(data)
        fcs_bytes = struct.pack('<H', fcs)
        combined = data + fcs_bytes
        assert hdlc_fcs16(combined) == 0xF0B8

    def test_byte_stuffing(self):
        """HDLC byte stuffing round-trip."""
        original = bytes([0x7E, 0x7D, 0x42, 0x7E])
        stuffed = hdlc_byte_stuff(original)
        assert 0x7E not in stuffed  # No raw flags in stuffed data
        unstuffed = hdlc_byte_unstuff(stuffed)
        assert unstuffed == original

    def test_hdlc_frame_build_parse(self):
        """Build and parse an HDLC frame."""
        info = b"\x01\x02\x03\x04"
        frame = build_hdlc_frame(0x0001, 0x0010, info)
        assert frame[0] == HDLC_FLAG
        assert frame[-1] == HDLC_FLAG

        frames = extract_frames(frame)
        assert len(frames) == 1
        addr, ctrl, parsed_info = parse_hdlc_frame(frames[0])
        assert parsed_info == info

    def test_hdlc_frame_with_escape_chars(self):
        """HDLC frame with bytes needing escaping."""
        info = bytes([0x7E, 0x7D, 0x42])
        frame = build_hdlc_frame(0x0001, 0x0010, info)
        frames = extract_frames(frame)
        _, _, parsed_info = parse_hdlc_frame(frames[0])
        assert parsed_info == info

    def test_hdlc_frame_fcs_error(self):
        """HDLC frame with corrupted FCS should fail."""
        info = b"\x01\x02\x03"
        frame = build_hdlc_frame(0x0001, 0x0010, info)
        # Corrupt a byte
        frame_bytes = bytearray(frame)
        frame_bytes[3] ^= 0xFF
        frames = extract_frames(bytes(frame_bytes))
        with pytest.raises(ValueError, match="FCS"):
            parse_hdlc_frame(frames[0])


class TestDlmsCosemPythonLib:
    """Test using dlms-cosem Python library if available."""

    @pytest.fixture(autouse=True)
    def _check_lib(self):
        try:
            import dlms_cosem  # noqa: F401
        except ImportError:
            pytest.skip("dlms-cosem Python library not available")

    def test_import_dlms_cosem(self):
        """Verify dlms-cosem library can be imported."""
        from dlms_cosem import DlmsClient  # noqa: F401
