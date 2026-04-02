"""OTA firmware update tests."""
import pytest


class TestOta:
    def test_meter_responds_after_boot(self, text_conn):
        """Verify meter is responsive (implies successful boot)."""
        resp = text_conn("ID")
        assert resp.startswith("OK")
