#
# This file is licensed under the Affero General Public License (AGPL) version 3.
#
# Copyright (C) 2026 Wroote
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as
# published by the Free Software Foundation, either version 3 of the
# License, or (at your option) any later version.
#
# See the GNU Affero General Public License for more details:
# <https://www.gnu.org/licenses/agpl-3.0.html>.
#
from typing import Any

from synapse.config import ConfigError
from synapse.config.homeserver import HomeServerConfig

from tests.unittest import TestCase
from tests.utils import default_config


def _parse(wroote: Any = None, *, include: bool = True) -> HomeServerConfig:
    config_dict = default_config(server_name="test")
    if include:
        config_dict["wroote"] = wroote
    config = HomeServerConfig()
    config.parse_config_dict(config_dict, "", "")
    return config


class WrooteConfigTestCase(TestCase):
    def test_absent_section_means_upstream_behaviour(self) -> None:
        self.assertFalse(_parse(include=False).wroote.voip_call_push_rule)

    def test_empty_section_means_upstream_behaviour(self) -> None:
        self.assertFalse(_parse(None).wroote.voip_call_push_rule)
        self.assertFalse(_parse({}).wroote.voip_call_push_rule)

    def test_voip_call_push_rule_enabled(self) -> None:
        self.assertTrue(
            _parse({"voip_call_push_rule": True}).wroote.voip_call_push_rule
        )

    def test_rejects_a_non_boolean(self) -> None:
        # "true" as a string is a common YAML mistake; refuse it instead of guessing.
        with self.assertRaises(ConfigError):
            _parse({"voip_call_push_rule": "true"})

    def test_rejects_a_non_mapping_section(self) -> None:
        with self.assertRaises(ConfigError):
            _parse([{"voip_call_push_rule": True}])

    def test_rejects_falsy_non_mapping_values(self) -> None:
        # `wroote: false` or `wroote: ""` used to be read as an empty section.
        values: tuple[Any, ...] = (False, 0, "", [])
        for value in values:
            with self.assertRaises(ConfigError, msg=repr(value)):
                _parse(value)

    def test_rejects_unknown_options(self) -> None:
        # A typo must not silently leave the feature off.
        with self.assertRaises(ConfigError):
            _parse({"voip_call_pushrule": True})
