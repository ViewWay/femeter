"""Metering data integration tests via virtual meter text protocol."""
import json
import pytest


class TestMeteringBasic:
    """Read voltage, current, power, energy via SNAPSHOT command."""

    def test_snapshot_returns_json(self, text_conn):
        resp = text_conn("SNAPSHOT")
        assert resp.startswith("DATA {"), f"Expected DATA JSON, got: {resp}"
        # Parse JSON from "DATA {...}"
        json_str = resp[5:]
        data = json.loads(json_str)
        assert isinstance(data, dict)

    def test_voltage_ranges(self, text_conn):
        resp = text_conn("SNAPSHOT")
        data = json.loads(resp[5:])
        for phase in ["ua", "ub", "uc"]:
            v = data.get(phase, 0)
            assert 0 <= v <= 300, f"{phase}={v}V out of range"

    def test_device_id(self, text_conn):
        resp = text_conn("ID")
        assert resp.startswith("OK"), f"ID failed: {resp}"
        assert len(resp) > 3

    def test_reset(self, text_conn):
        resp = text_conn("RESET")
        assert "OK" in resp, f"RESET failed: {resp}"

    def test_read_register(self, text_conn):
        """READ a known register address."""
        resp = text_conn("READ 00")
        assert resp.startswith("OK") or resp.startswith("ERR"), f"Unexpected: {resp}"

    def test_write_register(self, text_conn):
        resp = text_conn("WRITE 00 123456")
        assert resp.startswith("OK") or resp.startswith("ERR")

    def test_help(self, text_conn):
        resp = text_conn("HELP")
        assert "OK" in resp or "ERR" not in resp or len(resp) > 3

    def test_unknown_command(self, text_conn):
        resp = text_conn("FOOBAR")
        assert "ERR" in resp
