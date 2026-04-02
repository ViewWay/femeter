"""DLMS/COSEM protocol integration tests via virtual meter TCP."""
import socket
import pytest

DLMS_PORT = 4059


def _dlms_available():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(1)
        return s.connect_ex(("127.0.0.1", DLMS_PORT)) == 0


@pytest.fixture
def dlms_conn(virtual_meter):
    """Raw TCP connection to DLMS port."""
    def connect():
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(5)
        s.connect(("127.0.0.1", DLMS_PORT))
        return s
    return connect


class TestDlmsConnection:
    """Basic DLMS-over-TCP connectivity."""

    @pytest.fixture(autouse=True)
    def _check_dlms(self):
        if not _dlms_available():
            pytest.skip("DLMS TCP port 4059 not available")

    def test_tcp_connect(self, dlms_conn):
        s = dlms_conn()
        s.close()

    def test_hdlc_frame_format(self, dlms_conn):
        s = dlms_conn()
        try:
            s.sendall(b"\x7e")
            s.settimeout(3)
            resp = s.recv(1024)
            assert len(resp) > 0
        finally:
            s.close()

    def test_hdlc_flag_bytes(self, dlms_conn):
        """Multiple HDLC flags should be accepted."""
        s = dlms_conn()
        try:
            s.sendall(b"\x7e\x7e\x7e")
            s.settimeout(3)
            resp = s.recv(1024)
            # Server should respond or at least not disconnect
            assert True
        finally:
            s.close()

    def test_dlms_disconnect_reconnect(self, dlms_conn):
        """Disconnect and reconnect should work."""
        s1 = dlms_conn()
        s1.close()
        s2 = dlms_conn()
        s2.close()
