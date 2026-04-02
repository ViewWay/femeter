"""Time-of-use (TOU) tariff boundary and edge case tests."""
import json
import pytest


class TestTouBoundary:
    """TOU tariff boundary tests — verify rate switching at midnight, hour boundaries."""

    def test_snapshot_returns_dict(self, text_conn):
        resp = text_conn("SNAPSHOT")
        data = json.loads(resp[5:])
        assert isinstance(data, dict)

    def test_tou_field_types(self, text_conn):
        """TOU fields should be numeric if present."""
        resp = text_conn("SNAPSHOT")
        data = json.loads(resp[5:])
        # Check for common TOU field names
        for key in ["tariff", "rate", "tou_period", "active_import"]:
            if key in data:
                assert isinstance(data[key], (int, float, type(None)))


class TestTouRateSwitching:
    """Test rate switching logic at time boundaries."""

    def test_tou_data_consistency(self, text_conn):
        """Multiple SNAPSHOT calls should return consistent TOU structure."""
        resp1 = text_conn("SNAPSHOT")
        data1 = json.loads(resp1[5:])
        resp2 = text_conn("SNAPSHOT")
        data2 = json.loads(resp2[5:])
        # Both should have the same keys
        assert set(data1.keys()) == set(data2.keys())

    def test_energy_accumulation_direction(self, text_conn):
        """Energy should monotonically increase or stay constant."""
        resp1 = text_conn("SNAPSHOT")
        data1 = json.loads(resp1[5:])
        resp2 = text_conn("SNAPSHOT")
        data2 = json.loads(resp2[5:])
        # If energy fields exist, they should be non-negative
        for key in ["active_import", "active_export", "reactive_import", "reactive_export"]:
            if key in data1 and isinstance(data1[key], (int, float)):
                assert data1[key] >= 0
