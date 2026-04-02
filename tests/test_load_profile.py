"""Load profile tests."""
import json
import pytest


class TestLoadProfile:
    def test_load_profile_data(self, text_conn):
        resp = text_conn("SNAPSHOT")
        data = json.loads(resp[5:])
        assert isinstance(data, dict)
