"""pytest fixtures for Femeter virtual meter integration tests."""
import subprocess
import socket
import time
import pytest
import os

VIRTUAL_METER_BIN = os.path.join(
    os.path.dirname(__file__), "..", "target", "release", "test_server"
)
TEXT_PORT = 8888
DLMS_PORT = 4059


def _is_port_open(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(1)
        return s.connect_ex(("127.0.0.1", port)) == 0


@pytest.fixture(scope="session")
def virtual_meter():
    """Start virtual meter process, yield TCP connection."""
    if not os.path.isfile(VIRTUAL_METER_BIN):
        pytest.skip("virtual_meter binary not built. Run: cargo build -p virtual-meter --release")

    proc = subprocess.Popen(
        [VIRTUAL_METER_BIN],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=os.path.dirname(VIRTUAL_METER_BIN),
    )

    # Wait for TCP server to be ready
    for _ in range(20):
        if _is_port_open(TEXT_PORT):
            break
        time.sleep(0.5)
    else:
        proc.kill()
        pytest.skip("virtual_meter failed to start within 10s")

    yield proc

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


@pytest.fixture
def text_conn(virtual_meter):
    """Return a function that sends a text command and returns the response."""
    def send(cmd: str, port: int = TEXT_PORT) -> str:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(5)
            s.connect(("127.0.0.1", port))
            s.sendall((cmd + "\n").encode())
            data = b""
            while True:
                chunk = s.recv(4096)
                if not chunk:
                    break
                data += chunk
                if b"\n" in data:
                    break
        return data.decode().strip()
    return send
