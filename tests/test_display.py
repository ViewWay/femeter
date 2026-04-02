"""Display module tests."""
import pytest


class TestDisplay:
    def test_display_command(self, text_conn):
        resp = text_conn("HELP")
        assert "OK" in resp or len(resp) > 3
