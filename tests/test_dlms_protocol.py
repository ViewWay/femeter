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


@pytest.mark.skipif(not _dlms_available(), reason="DLMS TCP port 4059 not available")
class TestDlmsConnection:
    """Basic DLMS-over-TCP connectivity."""

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
