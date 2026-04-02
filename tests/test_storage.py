"""Storage / persistence tests."""
import json
import pytest


class TestStorage:
    def test_persistence_after_reset(self, text_conn):
        """Read snapshot, reset, read again - verify basic functionality."""
        resp1 = text_conn("SNAPSHOT")
        assert resp1.startswith("DATA")

        resp2 = text_conn("RESET")
        assert "OK" in resp2

        resp3 = text_conn("SNAPSHOT")
        assert resp3.startswith("DATA")
        data3 = json.loads(resp3[5:])
        assert isinstance(data3, dict)
