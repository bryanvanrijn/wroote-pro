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
"""Tests for the Wroote Pro server-default push rule for EuroChat call invites
(`wroote.voip_call_push_rule`)."""

from twisted.internet.testing import MemoryReactor

from synapse.push.bulk_push_rule_evaluator import BulkPushRuleEvaluator
from synapse.rest import admin
from synapse.rest.client import login, push_rule, room
from synapse.server import HomeServer
from synapse.storage.databases.main.event_push_actions import _deserialize_action
from synapse.types import JsonDict, create_requester
from synapse.util.clock import Clock

from tests.unittest import HomeserverTestCase, override_config

CALL_INVITE_TYPE = "eu.eurochat.call.invite"
RULE_ID = ".eu.eurochat.rule.call_invite"
ENABLED = {"wroote": {"voip_call_push_rule": True}}

# What the EuroChat web client sends to start ringing and to cancel
# (packages/web/src/services/matrixClient.ts, sendStateEvent with state key "").
RINGING = {"active": True, "status": "ringing", "call_id": "c1"}
CANCELLED = {"active": False, "call_id": "c1"}

RING_ACTIONS = [
    "notify",
    {"set_tweak": "sound", "value": "ring"},
    {"set_tweak": "highlight", "value": False},
]


class WrooteCallPushRuleTestCase(HomeserverTestCase):
    servlets = [
        admin.register_servlets_for_client_rest_resource,
        room.register_servlets,
        login.register_servlets,
        push_rule.register_servlets,
    ]

    def prepare(
        self, reactor: MemoryReactor, clock: Clock, homeserver: HomeServer
    ) -> None:
        self.alice = self.register_user("alice", "pass")
        self.token = self.login(self.alice, "pass")
        self.requester = create_requester(self.alice)
        self.room_id = self.helper.create_room_as(self.alice, tok=self.token)
        self.event_creation_handler = self.hs.get_event_creation_handler()

    def _actions_for_alice(self, content: JsonDict) -> list | None:
        """Run the push rules for a call-invite state event sent by someone else
        and return the actions Alice gets, or None when nothing notifies."""
        event, unpersisted_context = self.get_success(
            self.event_creation_handler.create_event(
                self.requester,
                {
                    "type": CALL_INVITE_TYPE,
                    "state_key": "",
                    "room_id": self.room_id,
                    "content": content,
                    "sender": f"@bob:{self.hs.hostname}",
                },
            )
        )
        context = self.get_success(unpersisted_context.persist(event))
        bulk_evaluator = BulkPushRuleEvaluator(self.hs)
        self.get_success(bulk_evaluator.action_for_events_by_user([(event, context)]))

        rows = self.get_success(
            self.hs.get_datastores().main.db_pool.simple_select_list(
                table="event_push_actions_staging",
                keyvalues={"event_id": event.event_id, "user_id": self.alice},
                retcols=("actions", "highlight"),
                desc="test_wroote_call_push_rule",
            )
        )
        if not rows:
            return None
        self.assertEqual(len(rows), 1)
        actions, highlight = rows[0]
        return _deserialize_action(actions, bool(highlight))

    def _override_rule_ids(self) -> list[str]:
        channel = self.make_request("GET", "/pushrules/", access_token=self.token)
        self.assertEqual(channel.code, 200, channel.json_body)
        return [rule["rule_id"] for rule in channel.json_body["global"]["override"]]

    def test_disabled_by_default(self) -> None:
        """Without the option the server behaves like upstream: no rule, no ring."""
        self.assertNotIn(RULE_ID, self._override_rule_ids())
        self.assertIsNone(self._actions_for_alice(RINGING))

    @override_config(ENABLED)
    def test_ringing_call_invite_rings(self) -> None:
        self.assertEqual(self._actions_for_alice(RINGING), RING_ACTIONS)

    @override_config(ENABLED)
    def test_cancelled_call_invite_does_not_ring(self) -> None:
        self.assertIsNone(self._actions_for_alice(CANCELLED))

    @override_config(ENABLED)
    def test_rule_is_listed_directly_after_master(self) -> None:
        """Clients see the rule, in the position where it is evaluated."""
        rule_ids = self._override_rule_ids()
        self.assertEqual(rule_ids[:2], [".m.rule.master", RULE_ID])

    @override_config(ENABLED)
    def test_user_can_disable_the_rule(self) -> None:
        channel = self.make_request(
            "PUT",
            f"/pushrules/global/override/{RULE_ID}/enabled",
            {"enabled": False},
            access_token=self.token,
        )
        self.assertEqual(channel.code, 200, channel.json_body)
        self.assertIsNone(self._actions_for_alice(RINGING))

    @override_config(ENABLED)
    def test_wins_over_a_user_override_rule(self) -> None:
        """A user override matching the same event (like the rule the EuroChat API
        writes for the iOS call-signaling pusher) must not demote the call: only the
        `sound` tweak makes the HTTP pusher send `prio: high`."""
        channel = self.make_request(
            "PUT",
            "/pushrules/global/override/eurochat.call-signaling",
            {
                "conditions": [
                    {"kind": "event_match", "key": "type", "pattern": CALL_INVITE_TYPE},
                    {
                        "kind": "event_match",
                        "key": "content.status",
                        "pattern": "ringing",
                    },
                ],
                "actions": ["notify", {"set_tweak": "priority", "value": "high"}],
            },
            access_token=self.token,
        )
        self.assertEqual(channel.code, 200, channel.json_body)
        self.assertEqual(self._actions_for_alice(RINGING), RING_ACTIONS)
