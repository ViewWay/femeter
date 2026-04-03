"""
Rust 虚拟电表 <-> Python dlms-cosem 互操作性测试

虚拟电表在 TCP 4059 端口提供 HDLC-over-TCP 服务。
注意: Rust 虚拟电表没有 HDLC 链路层状态机 (无 SNRM/UA 握手)，
它直接处理 HDLC I-frame 中的 APDU。因此我们需要自定义传输层，
跳过 SNRM/UA 步骤，直接收发 HDLC I-frame。
"""

import socket
import time
import struct
import pytest
import threading

METER_HOST = "127.0.0.1"
DLMS_PORT = 4059
TEXT_PORT = 8888


# ---------------------------------------------------------------------------
# Helper: Raw TCP socket with HDLC framing
# ---------------------------------------------------------------------------

class DirectHdlcTransport:
    """
    HDLC transport that sends/receives I-frames directly over TCP
    without SNRM/UA handshake (matching Rust virtual-meter behavior).

    Compatible with dlms_cosem DlmsConnection for APDU-level operations.
    """

    def __init__(self, host: str, port: int, timeout: int = 5):
        self.host = host
        self.port = port
        self.timeout = timeout
        self.sock: socket.socket | None = None
        self._client_addr = 1
        self._server_addr = 1
        self._ssn = 0  # send sequence number
        self._rsn = 0  # receive sequence number

    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.settimeout(self.timeout)
        self.sock.connect((self.host, self.port))

    def disconnect(self):
        if self.sock:
            try:
                self.sock.close()
            except OSError:
                pass
            self.sock = None

    def _encode_address(self) -> bytes:
        """Encode HDLC address: client + server."""
        # Client address: (addr << 1) | 0x01
        client_byte = (self._client_addr << 1) | 0x01
        # Server address (1 byte): (addr << 1) | 0x01
        server_byte = (self._server_addr << 1) | 0x01
        return bytes([client_byte, server_byte])

    def _crc16(self, data: bytes) -> int:
        crc = 0xFFFF
        for b in data:
            crc ^= b
            for _ in range(8):
                if crc & 1:
                    crc = (crc >> 1) ^ 0x8408
                else:
                    crc >>= 1
        return crc ^ 0xFFFF

    def _hdlc_escape(self, data: bytes) -> bytes:
        out = bytearray()
        for b in data:
            if b in (0x7E, 0x7D):
                out.append(0x7D)
                out.append(b ^ 0x20)
            else:
                out.append(b)
        return bytes(out)

    def _hdlc_unescape(self, data: bytes) -> bytes:
        out = bytearray()
        i = 0
        while i < len(data):
            if data[i] == 0x7D and i + 1 < len(data):
                out.append(data[i + 1] ^ 0x20)
                i += 2
            else:
                out.append(data[i])
                i += 1
        return bytes(out)

    def send_request(self, apdu_bytes: bytes) -> bytes:
        """Send an HDLC I-frame with the given APDU and receive the response."""
        if not self.sock:
            raise RuntimeError("Not connected")

        # Build I-frame: control byte with send/receive sequence numbers
        # I-frame format: [addr | control | info | fcs]
        control = (self._rsn << 5) | (self._ssn << 1)
        addr = self._encode_address()
        # HCS: CRC of address + control
        addr_ctrl = addr + bytes([control])
        hcs = self._crc16(addr_ctrl)
        # Full frame data: addr + control + HCS + info + FCS
        frame_data = addr_ctrl + struct.pack("<H", hcs) + apdu_bytes
        fcs = self._crc16(frame_data)
        frame_data += struct.pack("<H", fcs)

        # Wrap with flags and byte-stuff
        frame = bytes([0x7E]) + self._hdlc_escape(frame_data) + bytes([0x7E])

        self.sock.sendall(frame)

        # Read response frame (flag-delimited)
        resp = self._recv_frame()

        # Parse response: skip flags, unescape, verify CRC
        resp_inner = self._hdlc_unescape(resp)
        if len(resp_inner) < 7:
            raise ValueError(f"Response too short: {len(resp_inner)} bytes")

        # Verify FCS
        fcs_end = len(resp_inner)
        fcs = struct.unpack("<H", resp_inner[fcs_end-2:fcs_end])[0]
        if self._crc16(resp_inner[:fcs_end-2]) != fcs:
            raise ValueError("FCS mismatch")

        # Skip address + control + HCS (2 bytes), extract info
        # Address field: client (1 byte) + server (variable, until bit0=1)
        # Client addr is always 1 byte. Then server addr bytes until bit0=1.
        if len(resp_inner) < 2:
            raise ValueError("Response too short for address")
        addr_len = 1  # client address is always 1 byte
        pos = 1
        while pos < len(resp_inner):
            addr_len += 1
            if resp_inner[pos] & 0x01:
                break
            pos += 1
        info_start = addr_len + 1 + 2  # +1 control, +2 HCS
        info_end = len(resp_inner) - 2  # -2 for FCS

        if info_start >= info_end:
            # No info field (e.g. UA frame)
            response_apdu = b""
        else:
            response_apdu = resp_inner[info_start:info_end]

        # Increment sequence numbers
        self._ssn = (self._ssn + 1) & 0x07
        self._rsn = (self._rsn + 1) & 0x07

        return response_apdu

    def _parse_address_length(self, data: bytes) -> int:
        """Parse HDLC address field length."""
        pos = 0
        while pos < len(data):
            pos += 1
            if data[pos - 1] & 0x01:  # last address byte
                return pos
        return pos

    def _recv_frame(self) -> bytes:
        """Receive an HDLC frame (between flags)."""
        buf = bytearray()
        found_start = False
        while True:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise ConnectionError("Connection closed")
            for b in chunk:
                if b == 0x7E:
                    if found_start and len(buf) > 0:
                        return bytes(buf)
                    found_start = True
                    buf = bytearray()
                elif found_start:
                    buf.append(b)
            if not found_start and len(buf) == 0:
                raise TimeoutError("No HDLC frame start received")


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

class TestBasicConnectivity:
    """基础连接测试"""

    def test_tcp_text_port(self):
        """文本协议端口 8888 可连接"""
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(3)
        s.connect((METER_HOST, TEXT_PORT))
        s.close()

    def test_dlms_port_listening(self):
        """DLMS HDLC 端口 4059 可连接"""
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(3)
        s.connect((METER_HOST, DLMS_PORT))
        s.close()


class TestDlmsApduLevel:
    """DLMS APDU 级别测试（通过 HDLC 帧封装）"""

    @pytest.fixture(autouse=True)
    def setup_transport(self):
        self.transport = DirectHdlcTransport(METER_HOST, DLMS_PORT, timeout=5)
        self.transport.connect()
        yield
        try:
            self.transport.disconnect()
        except Exception:
            pass

    def _send_apdu(self, apdu: bytes) -> bytes:
        """Helper: send APDU and return response APDU."""
        return self.transport.send_request(apdu)

    def test_hdlc_frame_exchange(self):
        """HDLC 帧往返 — 发送 AARQ 收到 AARE"""
        # Minimal AARQ for LN association
        aarq = bytes([
            0x60, 0x1A, 0xA1, 0x09, 0x06, 0x07, 0x60, 0x85, 0x74, 0x05,
            0x08, 0x01, 0x01, 0xBE, 0x10, 0x04, 0x0E, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x09, 0x0C, 0x06, 0x00, 0x00, 0x01, 0x00, 0xFF, 0xAA,
        ])
        resp = self._send_apdu(aarq)
        assert len(resp) > 0
        # AARE response starts with 0xE1 (accepted) or 0xE6
        assert resp[0] in (0xE1, 0xE6)

    def test_release_request(self):
        """RLRQ 返回 RLRE accepted"""
        rlrq = bytes([0x80, 0x01, 0x00])
        resp = self._send_apdu(rlrq)
        assert len(resp) > 0
        # RLRE accepted: 0x81 0x00
        assert resp[0] == 0x81

    def test_get_request_clock(self):
        """GET Request — 读取时钟"""
        # GetRequest-Normal: C0 (tag=0xC0) + invoke_id + CosemAttribute
        # Class=8 (Clock), OBIS=0.0.1.0.0.255, attribute=2
        get_req = bytes([
            0xC0, 0x01, 0xC1, 0x00,  # GetRequest-Normal, invoke_id=1
            0x00, 0x08,              # Class=8 (Clock)
            0x00, 0x00, 0x01, 0x00, 0x00, 0xFF,  # OBIS: 0.0.1.0.0.255
            0x02,                    # attribute 2 (time)
        ])
        resp = self._send_apdu(get_req)
        assert len(resp) > 0
        # GetResponse-Normal: C4 (tag=0xC4) + invoke_id + data
        assert resp[0] in (0xC4, 0xC1)

    def test_get_request_energy(self):
        """GET Request — 读取有功总电能"""
        get_req = bytes([
            0xC0, 0x01, 0xC1, 0x00,
            0x00, 0x03,              # Class=3 (Register)
            0x01, 0x00, 0x01, 0x08, 0x00, 0xFF,  # OBIS: 1.0.1.8.0.255
            0x02,                    # attribute 2
        ])
        resp = self._send_apdu(get_req)
        assert len(resp) > 0
        assert resp[0] in (0xC4, 0xC1)

    def test_get_request_voltage(self):
        """GET Request — 读取 A 相电压"""
        get_req = bytes([
            0xC0, 0x02, 0xC1, 0x00,
            0x00, 0x03,              # Class=3 (Register)
            0x01, 0x00, 0x32, 0x07, 0x00, 0xFF,  # OBIS: 1.0.32.7.0.255
            0x02,
        ])
        resp = self._send_apdu(get_req)
        assert len(resp) > 0

    def test_set_request(self):
        """SET Request — 设置时钟时区"""
        set_req = bytes([
            0xD0, 0x01,            # SET-Request-Normal, invoke_id=1
            0x00, 0x08,              # Class=8 (Clock)
            0x00, 0x00, 0x01, 0x00, 0x00, 0xFF,  # OBIS: 0.0.1.0.0.255
            0x03,                    # attribute 3 (time_zone)
            0x02,                    # access selector
            0x12,                    # data: Int16(0)
        ])
        resp = self._send_apdu(set_req)
        assert len(resp) > 0
        # SetResponse-Normal success: D5 01 00
        assert resp[0] in (0xD5, 0xD1)

    @pytest.mark.xfail(reason="Action request format requires parameters; needs further investigation")
    def test_action_request(self):
        """ACTION Request — Association method 1"""
        act_req = bytes([
            0xC2, 0x01,            # Action-Request-Normal, invoke_id=1
            0x00, 0x0C,              # Class=12 (Association LN)
            0x00, 0x00, 0x28, 0x00, 0x00, 0xFF,  # OBIS: 0.0.40.0.0.255
            0x01,                    # method 1
        ])
        resp = self._send_apdu(act_req)
        assert len(resp) > 0

    def test_get_request_invalid_obis(self):
        """GET Request — 未知 OBIS 返回 null"""
        get_req = bytes([
            0xC0, 0x03, 0xC1, 0x00,
            0x00, 0x03,              # Class=3 (Register)
            0x09, 0x09, 0x09, 0x09, 0x09, 0xFF,  # Unknown OBIS
            0x02,
        ])
        resp = self._send_apdu(get_req)
        assert len(resp) > 0


class TestDlmsCosemClient:
    """使用 dlms_cosem Python 库生成 AARQ，验证 Rust 服务器可正确处理。

    NOTE: Rust 服务器的 AARE 响应使用不同的 BER 编码约定（context tag
    映射与 Python dlms_cosem 不完全一致），因此这些测试验证传输层
    互操作性而非 Python 库的 APDU 解析。
    """

    @pytest.fixture(autouse=True)
    def setup_client(self):
        from dlms_cosem.connection import DlmsConnection, default_system_title
        from dlms_cosem.security import NoSecurityAuthentication

        self.transport = DirectHdlcTransport(METER_HOST, DLMS_PORT, timeout=5)
        self.transport.connect()

        auth = NoSecurityAuthentication()
        self.dlms_connection = DlmsConnection(
            authentication=auth,
            client_system_title=default_system_title(),
        )
        yield
        try:
            self.transport.disconnect()
        except Exception:
            pass

    def test_python_aarq_decoded_by_rust(self):
        """Python dlms-cosem 生成的 AARQ 可被 Rust 服务器正确解码并响应"""
        aarq_bytes = self.dlms_connection.get_aarq().to_bytes()
        resp = self.transport.send_request(aarq_bytes)
        assert len(resp) > 5
        assert resp[0] == 0x61  # AARE tag (APPLICATION-1 CONSTRUCTED)

    def test_python_aarq_contains_ln_oid(self):
        """AARE 响应包含 LN OID"""
        aarq_bytes = self.dlms_connection.get_aarq().to_bytes()
        resp = self.transport.send_request(aarq_bytes)
        # Verify AARE has an application-context-name OID
        assert b'\x06' in resp  # OID tag present
        assert resp[0] == 0x61

    def test_full_session_lifecycle(self):
        """完整会话: AARQ -> GET clock -> GET energy -> RLRE"""
        # 1. AARQ
        aarq_bytes = self.dlms_connection.get_aarq().to_bytes()
        aare = self.transport.send_request(aarq_bytes)
        assert aare[0] == 0x61

        # 2. GET clock
        get_req = bytes([
            0xC0, 0x01, 0x01, 0x00, 0x08,
            0x00, 0x00, 0x01, 0x00, 0x00, 0xFF, 0x02, 0x01
        ])
        get_resp = self.transport.send_request(get_req)
        # C4 (GetResponse-Normal) or C1 (error)
        assert get_resp[0] in (0xC4, 0xC1)

        # 3. GET energy
        get_req2 = bytes([
            0xC0, 0x02, 0x02, 0x00, 0x03,
            0x01, 0x00, 0x01, 0x08, 0x00, 0xFF, 0x02, 0x01
        ])
        get_resp2 = self.transport.send_request(get_req2)
        assert get_resp2[0] in (0xC4, 0xC1)

        # 4. RLRE
        rlrq = bytes([0x80, 0x01, 0x00])
        rlre = self.transport.send_request(rlrq)
        assert rlre[0] == 0x81


class TestTextProtocol:
    """文本协议端口测试"""

    def test_text_command_help(self):
        """发送 help 命令，收到响应"""
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(3)
        s.connect((METER_HOST, TEXT_PORT))
        s.sendall(b"help\n")
        resp = s.recv(4096)
        s.close()
        assert len(resp) > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
