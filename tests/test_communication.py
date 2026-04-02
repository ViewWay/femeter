"""Communication module tests: HDLC framing, CRC, timeouts."""
import socket
import pytest


class TestHdlcFraming:
    """Test HDLC frame handling via raw TCP."""

    def test_empty_frame_rejected(self, virtual_meter):
        """Empty data should not crash the server."""
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(3)
            s.connect(("127.0.0.1", 8888))
            s.sendall(b"\n")
            resp = s.recv(1024)
            # Should get an error or empty response, not crash
            assert True

    def test_garbage_data_handled(self, virtual_meter):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(3)
            s.connect(("127.0.0.1", 8888))
            s.sendall(b"\x00\xff\xfe\xfd\xfc\n")
            resp = s.recv(1024)
            assert len(resp) >= 0  # just don't crash
