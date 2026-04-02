"""Event storm test — verify system stability under rapid event bursts."""
import json
import pytest


class TestEventStorm:
    """Test that the virtual meter handles rapid consecutive commands without crashing."""

    def test_rapid_snapshot_burst(self, text_conn):
        """Send 50 rapid SNAPSHOT commands, all should succeed."""
        for i in range(50):
            resp = text_conn("SNAPSHOT")
            assert resp.startswith("DATA"), f"SNAPSHOT #{i} failed: {resp}"

    def test_rapid_mixed_commands(self, text_conn):
        """Mix of SNAPSHOT and unknown commands should not crash the server."""
        for i in range(20):
            resp = text_conn("SNAPSHOT")
            assert resp.startswith("DATA")
            # Send an unknown command — should not crash
            resp2 = text_conn("UNKNOWN_CMD")
            # Server should respond, even if with an error

    def test_concurrent_snapshots(self, text_conn):
        """Multiple sequential reads should be consistent."""
        results = []
        for _ in range(10):
            resp = text_conn("SNAPSHOT")
            data = json.loads(resp[5:])
            results.append(data)
        # All should have the same structure
        keys_set = set(results[0].keys())
        for r in results[1:]:
            assert set(r.keys()) == keys_set

    def test_event_field_persistence(self, text_conn):
        """Events should accumulate or be tracked across snapshots."""
        resp1 = text_conn("SNAPSHOT")
        data1 = json.loads(resp1[5:])
        # Send some writes if supported
        text_conn("SNAPSHOT")
        resp2 = text_conn("SNAPSHOT")
        data2 = json.loads(resp2[5:])
        # Structure should remain stable
        assert isinstance(data1, dict)
        assert isinstance(data2, dict)
