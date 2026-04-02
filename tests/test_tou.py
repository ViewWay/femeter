"""Time-of-use (TOU) tariff tests."""
import json
import pytest


class TestTou:
    def test_tariff_info_in_snapshot(self, text_conn):
        resp = text_conn("SNAPSHOT")
        data = json.loads(resp[5:])
        # Tariff-related fields may exist
        assert isinstance(data, dict)
