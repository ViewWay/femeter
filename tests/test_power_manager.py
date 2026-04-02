"""Power manager tests."""
import pytest


class TestPowerManager:
    def test_meter_stays_alive_after_multiple_requests(self, text_conn):
        """Verify meter doesn't leak resources or crash under repeated queries."""
        for i in range(100):
            resp = text_conn("SNAPSHOT")
            assert resp.startswith("DATA")
