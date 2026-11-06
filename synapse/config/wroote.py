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
"""Options that only exist in Wroote Pro.

Everything Wroote Pro adds on top of upstream Synapse is switched on here, and
every option defaults to upstream behaviour. A config without a `wroote`
section therefore runs exactly like upstream Synapse.
"""

from typing import Any

from synapse.types import JsonDict

from ._base import Config, ConfigError

_KNOWN_OPTIONS = frozenset({"voip_call_push_rule"})


class WrooteConfig(Config):
    section = "wroote"

    def read_config(self, config: JsonDict, **kwargs: Any) -> None:
        wroote_config = config.get("wroote")
        if wroote_config is None:
            wroote_config = {}
        if not isinstance(wroote_config, dict):
            raise ConfigError("must be a dictionary", ("wroote",))
        # Unknown keys are refused rather than ignored: a typo would otherwise
        # silently leave a feature switched off.
        unknown = sorted(set(wroote_config) - _KNOWN_OPTIONS)
        if unknown:
            raise ConfigError(
                f"unknown option(s): {', '.join(unknown)}; known: "
                f"{', '.join(sorted(_KNOWN_OPTIONS))}",
                ("wroote",),
            )

        # Server-default push rule that rings on EuroChat call invites
        # (`eu.eurochat.call.invite` with `content.status: ringing`). Replaces the
        # per-user rule the `eurochat_voip_pushrules` module wrote on every login.
        voip_call_push_rule = wroote_config.get("voip_call_push_rule", False)
        if not isinstance(voip_call_push_rule, bool):
            raise ConfigError("must be a boolean", ("wroote", "voip_call_push_rule"))
        self.voip_call_push_rule: bool = voip_call_push_rule
