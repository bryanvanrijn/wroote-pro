/*
 * This file is licensed under the Affero General Public License (AGPL) version 3.
 *
 * Copyright 2022, 2023 The Matrix.org Foundation C.I.C.
 * Copyright (C) 2023 New Vector, Ltd
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * See the GNU Affero General Public License for more details:
 * <https://www.gnu.org/licenses/agpl-3.0.html>.
 *
 * Originally licensed under the Apache License, Version 2.0:
 * <http://www.apache.org/licenses/LICENSE-2.0>.
 *
 * [This file includes modifications made by New Vector Limited]
 *
 */

//! Contains the definitions of the "base" push rules.

use std::borrow::Cow;
use std::collections::HashMap;

use lazy_static::lazy_static;
use serde_json::Value;

use super::KnownCondition;
use crate::push::RelatedEventMatchTypeCondition;
use crate::push::SetTweak;
use crate::push::TweakValue;
use crate::push::{Action, EventPropertyIsCondition, SimpleJsonValue};
use crate::push::{Condition, EventMatchTypeCondition};
use crate::push::{EventMatchCondition, EventMatchPatternType};
use crate::push::{EventPropertyIsTypeCondition, PushRule};

const HIGHLIGHT_ACTION: Action = Action::SetTweak(SetTweak {
    set_tweak: Cow::Borrowed("highlight"),
    value: None,
    other_keys: Value::Null,
});

const HIGHLIGHT_FALSE_ACTION: Action = Action::SetTweak(SetTweak {
    set_tweak: Cow::Borrowed("highlight"),
    value: Some(TweakValue::Other(Value::Bool(false))),
    other_keys: Value::Null,
});

const SOUND_ACTION: Action = Action::SetTweak(SetTweak {
    set_tweak: Cow::Borrowed("sound"),
    value: Some(TweakValue::String(Cow::Borrowed("default"))),
    other_keys: Value::Null,
});

const RING_ACTION: Action = Action::SetTweak(SetTweak {
    set_tweak: Cow::Borrowed("sound"),
    value: Some(TweakValue::String(Cow::Borrowed("ring"))),
    other_keys: Value::Null,
});

/// Wroote Pro: server-default rule for EuroChat call invites.
///
/// EuroChat clients ring on `eu.eurochat.call.invite` events whose
/// `content.status` is `ringing`. Cancellations carry `active: false` and no
/// `status`, so they never match. Shipping this as a server-default rule
/// replaces a per-user rule that a module used to write on every login, which
/// forced all logins onto the push-rules writer.
///
/// It sits directly after `.m.rule.master`, before the user's own override
/// rules, so it always wins. The `sound` tweak is what makes the HTTP pusher
/// send the push with `prio: high` (see `synapse/push/httppusher.py`); a
/// user override that matched first could otherwise demote a ringing call to
/// a low-priority push. Users can still disable the rule or change its
/// actions through the push rules API, and the master rule still mutes all.
///
/// The rule is filtered out unless the `wroote.voip_call_push_rule` config
/// option is enabled, so a server without that option behaves exactly like
/// upstream Synapse.
pub const WROOTE_VOIP_CALL_RULE_ID: &str = "global/override/.eu.eurochat.rule.call_invite";

pub const BASE_PREPEND_OVERRIDE_RULES: &[PushRule] = &[
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.master"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[]),
        actions: Cow::Borrowed(&[]),
        default: true,
        default_enabled: false,
    },
    PushRule {
        rule_id: Cow::Borrowed(WROOTE_VOIP_CALL_RULE_ID),
        priority_class: 5,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("eu.eurochat.call.invite"),
            })),
            // Only the state event (state key "") rings. Sending state needs
            // `state_default` (power level 50 by default); a timeline event of
            // this type would only need `events_default` (0), so any member
            // could make everyone's phone ring.
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("state_key"),
                pattern: Cow::Borrowed(""),
            })),
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("content.status"),
                pattern: Cow::Borrowed("ringing"),
            })),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, RING_ACTION, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
];

pub const BASE_APPEND_OVERRIDE_RULES: &[PushRule] = &[
    PushRule {
        rule_id: Cow::Borrowed("global/override/.org.matrix.msc4028.encrypted_event"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventMatch(
            EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("m.room.encrypted"),
            },
        ))]),
        actions: Cow::Borrowed(&[Action::Notify]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.suppress_notices"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventMatch(
            EventMatchCondition {
                key: Cow::Borrowed("content.msgtype"),
                pattern: Cow::Borrowed("m.notice"),
            },
        ))]),
        actions: Cow::Borrowed(&[]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.invite_for_me"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("m.room.member"),
            })),
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("content.membership"),
                pattern: Cow::Borrowed("invite"),
            })),
            Condition::Known(KnownCondition::EventMatchType(EventMatchTypeCondition {
                key: Cow::Borrowed("state_key"),
                pattern_type: Cow::Borrowed(&EventMatchPatternType::UserId),
            })),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_FALSE_ACTION, SOUND_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.member_event"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventMatch(
            EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("m.room.member"),
            },
        ))]),
        actions: Cow::Borrowed(&[]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.im.nheko.msc3664.reply"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::RelatedEventMatchType(
            RelatedEventMatchTypeCondition {
                key: Cow::Borrowed("sender"),
                pattern_type: Cow::Borrowed(&EventMatchPatternType::UserId),
                rel_type: Cow::Borrowed("m.in_reply_to"),
                include_fallbacks: None,
            },
        ))]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_ACTION, SOUND_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.org.matrix.msc4075.rule.rtc.invite_for_me"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("org.matrix.msc4075.rtc.notification"),
            })),
            Condition::Known(KnownCondition::ExactEventPropertyContainsType(
                EventPropertyIsTypeCondition {
                    key: Cow::Borrowed(r"content.m\.mentions.user_ids"),
                    value_type: Cow::Borrowed(&EventMatchPatternType::UserId),
                },
            )),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, RING_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.org.matrix.msc4075.rule.rtc.invite_for_room"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("org.matrix.msc4075.rtc.notification"),
            })),
            Condition::Known(KnownCondition::EventPropertyIs(EventPropertyIsCondition {
                key: Cow::Borrowed(r"content.m\.mentions.room"),
                value: Cow::Owned(SimpleJsonValue::Bool(true)),
            })),
            Condition::Known(KnownCondition::SenderNotificationPermission {
                key: Cow::Borrowed("room"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, RING_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.is_user_mention"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[Condition::Known(
            KnownCondition::ExactEventPropertyContainsType(EventPropertyIsTypeCondition {
                key: Cow::Borrowed(r"content.m\.mentions.user_ids"),
                value_type: Cow::Borrowed(&EventMatchPatternType::UserId),
            }),
        )]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_ACTION, SOUND_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.contains_display_name"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::ContainsDisplayName)]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_ACTION, SOUND_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.is_room_mention"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventPropertyIs(EventPropertyIsCondition {
                key: Cow::Borrowed(r"content.m\.mentions.room"),
                value: Cow::Owned(SimpleJsonValue::Bool(true)),
            })),
            Condition::Known(KnownCondition::SenderNotificationPermission {
                key: Cow::Borrowed("room"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.roomnotif"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::SenderNotificationPermission {
                key: Cow::Borrowed("room"),
            }),
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("content.body"),
                pattern: Cow::Borrowed("@room"),
            })),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.tombstone"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("m.room.tombstone"),
            })),
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("state_key"),
                pattern: Cow::Borrowed(""),
            })),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.reaction"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventMatch(
            EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("m.reaction"),
            },
        ))]),
        actions: Cow::Borrowed(&[]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.room.server_acl"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("m.room.server_acl"),
            })),
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("state_key"),
                pattern: Cow::Borrowed(""),
            })),
        ]),
        actions: Cow::Borrowed(&[]),
        default: true,
        default_enabled: true,
    },
    // We don't want to notify on edits *unless* the edit directly mentions a
    // user, which is handled above.
    PushRule {
        rule_id: Cow::Borrowed("global/override/.m.rule.suppress_edits"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventPropertyIs(
            EventPropertyIsCondition {
                key: Cow::Borrowed(r"content.m\.relates_to.rel_type"),
                value: Cow::Owned(SimpleJsonValue::Str(Cow::Borrowed("m.replace"))),
            },
        ))]),
        actions: Cow::Borrowed(&[]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/override/.org.matrix.msc3930.rule.poll_response"),
        priority_class: 5,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventMatch(
            EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("org.matrix.msc3381.poll.response"),
            },
        ))]),
        actions: Cow::Borrowed(&[]),
        default: true,
        default_enabled: true,
    },
];

pub const BASE_APPEND_CONTENT_RULES: &[PushRule] = &[PushRule {
    rule_id: Cow::Borrowed("global/content/.m.rule.contains_user_name"),
    priority_class: 4,
    conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventMatchType(
        EventMatchTypeCondition {
            key: Cow::Borrowed("content.body"),
            pattern_type: Cow::Borrowed(&EventMatchPatternType::UserLocalpart),
        },
    ))]),
    actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_ACTION, SOUND_ACTION]),
    default: true,
    default_enabled: true,
}];

pub const BASE_APPEND_POSTCONTENT_RULES: &[PushRule] = &[
    PushRule {
        rule_id: Cow::Borrowed("global/postcontent/.io.element.msc4306.rule.unsubscribed_thread"),
        priority_class: 6,
        conditions: Cow::Borrowed(&[Condition::Known(
            KnownCondition::Msc4306ThreadSubscription { subscribed: false },
        )]),
        actions: Cow::Borrowed(&[]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/postcontent/.io.element.msc4306.rule.subscribed_thread"),
        priority_class: 6,
        conditions: Cow::Borrowed(&[Condition::Known(
            KnownCondition::Msc4306ThreadSubscription { subscribed: true },
        )]),
        actions: Cow::Borrowed(&[Action::Notify, SOUND_ACTION]),
        default: true,
        default_enabled: true,
    },
];

pub const BASE_APPEND_UNDERRIDE_RULES: &[PushRule] = &[
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.m.rule.call"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventMatch(
            EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("m.call.invite"),
            },
        ))]),
        actions: Cow::Borrowed(&[Action::Notify, RING_ACTION, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.m.rule.encrypted_room_one_to_one"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("m.room.encrypted"),
            })),
            Condition::Known(KnownCondition::RoomMemberCount {
                is: Some(Cow::Borrowed("2")),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, SOUND_ACTION, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.m.rule.room_one_to_one"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("m.room.message"),
            })),
            Condition::Known(KnownCondition::RoomMemberCount {
                is: Some(Cow::Borrowed("2")),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, SOUND_ACTION, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed(
            "global/underride/.org.matrix.msc3933.rule.extensible.encrypted_room_one_to_one",
        ),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("org.matrix.msc1767.encrypted"),
            })),
            Condition::Known(KnownCondition::RoomMemberCount {
                is: Some(Cow::Borrowed("2")),
            }),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, SOUND_ACTION, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed(
            "global/underride/.org.matrix.msc3933.rule.extensible.message.room_one_to_one",
        ),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("org.matrix.msc1767.message"),
            })),
            Condition::Known(KnownCondition::RoomMemberCount {
                is: Some(Cow::Borrowed("2")),
            }),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, SOUND_ACTION, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed(
            "global/underride/.org.matrix.msc3933.rule.extensible.file.room_one_to_one",
        ),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("org.matrix.msc1767.file"),
            })),
            Condition::Known(KnownCondition::RoomMemberCount {
                is: Some(Cow::Borrowed("2")),
            }),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, SOUND_ACTION, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed(
            "global/underride/.org.matrix.msc3933.rule.extensible.image.room_one_to_one",
        ),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("org.matrix.msc1767.image"),
            })),
            Condition::Known(KnownCondition::RoomMemberCount {
                is: Some(Cow::Borrowed("2")),
            }),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, SOUND_ACTION, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed(
            "global/underride/.org.matrix.msc3933.rule.extensible.video.room_one_to_one",
        ),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("org.matrix.msc1767.video"),
            })),
            Condition::Known(KnownCondition::RoomMemberCount {
                is: Some(Cow::Borrowed("2")),
            }),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, SOUND_ACTION, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed(
            "global/underride/.org.matrix.msc3933.rule.extensible.audio.room_one_to_one",
        ),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("org.matrix.msc1767.audio"),
            })),
            Condition::Known(KnownCondition::RoomMemberCount {
                is: Some(Cow::Borrowed("2")),
            }),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, SOUND_ACTION, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.m.rule.message"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventMatch(
            EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("m.room.message"),
            },
        ))]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.m.rule.encrypted"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventMatch(
            EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("m.room.encrypted"),
            },
        ))]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.org.matrix.msc1767.rule.extensible.encrypted"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("m.encrypted"),
            })),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.org.matrix.msc1767.rule.extensible.message"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("m.message"),
            })),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.org.matrix.msc1767.rule.extensible.file"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("m.file"),
            })),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.org.matrix.msc1767.rule.extensible.image"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("m.image"),
            })),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.org.matrix.msc1767.rule.extensible.video"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("m.video"),
            })),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.org.matrix.msc1767.rule.extensible.audio"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                // MSC3933: Type changed from template rule - see MSC.
                pattern: Cow::Borrowed("m.audio"),
            })),
            // MSC3933: Add condition on top of template rule - see MSC.
            Condition::Known(KnownCondition::RoomVersionSupports {
                // RoomVersionFeatures::ExtensibleEvents.as_str(), ideally
                feature: Cow::Borrowed("org.matrix.msc3932.extensible_events"),
            }),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.im.vector.jitsi"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("im.vector.modular.widgets"),
            })),
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("content.type"),
                pattern: Cow::Borrowed("jitsi"),
            })),
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("state_key"),
                pattern: Cow::Borrowed("*"),
            })),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, HIGHLIGHT_FALSE_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.org.matrix.msc3930.rule.poll_start_one_to_one"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::RoomMemberCount {
                is: Some(Cow::Borrowed("2")),
            }),
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("org.matrix.msc3381.poll.start"),
            })),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, SOUND_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.org.matrix.msc3930.rule.poll_start"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventMatch(
            EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("org.matrix.msc3381.poll.start"),
            },
        ))]),
        actions: Cow::Borrowed(&[Action::Notify]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.org.matrix.msc3930.rule.poll_end_one_to_one"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[
            Condition::Known(KnownCondition::RoomMemberCount {
                is: Some(Cow::Borrowed("2")),
            }),
            Condition::Known(KnownCondition::EventMatch(EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("org.matrix.msc3381.poll.end"),
            })),
        ]),
        actions: Cow::Borrowed(&[Action::Notify, SOUND_ACTION]),
        default: true,
        default_enabled: true,
    },
    PushRule {
        rule_id: Cow::Borrowed("global/underride/.org.matrix.msc3930.rule.poll_end"),
        priority_class: 1,
        conditions: Cow::Borrowed(&[Condition::Known(KnownCondition::EventMatch(
            EventMatchCondition {
                key: Cow::Borrowed("type"),
                pattern: Cow::Borrowed("org.matrix.msc3381.poll.end"),
            },
        ))]),
        actions: Cow::Borrowed(&[Action::Notify]),
        default: true,
        default_enabled: true,
    },
];

lazy_static! {
    pub static ref BASE_RULES_BY_ID: HashMap<&'static str, &'static PushRule> =
        BASE_PREPEND_OVERRIDE_RULES
            .iter()
            .chain(BASE_APPEND_OVERRIDE_RULES.iter())
            .chain(BASE_APPEND_CONTENT_RULES.iter())
            .chain(BASE_APPEND_POSTCONTENT_RULES.iter())
            .chain(BASE_APPEND_UNDERRIDE_RULES.iter())
            .map(|rule| { (&*rule.rule_id, rule) })
            .collect();
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    use super::{HIGHLIGHT_FALSE_ACTION, RING_ACTION, WROOTE_VOIP_CALL_RULE_ID};
    use crate::push::evaluator::PushRuleEvaluator;
    use crate::push::{
        Action, Condition, EventMatchCondition, FilteredPushRules, JsonValue, KnownCondition,
        PushRule, PushRules, SetTweak, SimpleJsonValue, TweakValue,
    };

    const RTC_NOTIFICATION_TYPE: &str = "org.matrix.msc4075.rtc.notification";
    const EUROCHAT_CALL_INVITE_TYPE: &str = "eu.eurochat.call.invite";
    const ALICE: &str = "@alice:example.org";
    const BOB: &str = "@bob:example.org";

    /// The default push rules with no user-defined rules.
    ///
    /// The generic mention rules are disabled so that they cannot mask the
    /// MSC4075 rules, which have identical actions and come immediately
    /// before them in the override list.
    fn push_rules(msc4075_enabled: bool) -> FilteredPushRules {
        push_rules_with(msc4075_enabled, false)
    }

    fn push_rules_with(
        msc4075_enabled: bool,
        wroote_voip_call_push_rule: bool,
    ) -> FilteredPushRules {
        push_rules_custom(
            msc4075_enabled,
            wroote_voip_call_push_rule,
            vec![],
            BTreeMap::new(),
        )
    }

    fn push_rules_custom(
        msc4075_enabled: bool,
        wroote_voip_call_push_rule: bool,
        user_rules: Vec<PushRule>,
        extra_enabled: BTreeMap<String, bool>,
    ) -> FilteredPushRules {
        let mut enabled_map = BTreeMap::new();
        enabled_map.insert("global/override/.m.rule.is_user_mention".to_string(), false);
        enabled_map.insert("global/override/.m.rule.is_room_mention".to_string(), false);
        enabled_map.extend(extra_enabled);

        FilteredPushRules::py_new(
            PushRules::new(user_rules),
            enabled_map,
            false, // msc1767_enabled
            false, // msc3381_polls_enabled
            false, // msc3664_enabled
            false, // msc4028_push_encrypted_events
            msc4075_enabled,
            false, // msc4210_enabled
            false, // msc4306_enabled
            wroote_voip_call_push_rule,
        )
    }

    /// Builds an evaluator for a state event (state key "") of the given type
    /// with an optional `content.status` field, sent to a room of ten members
    /// without mentions. This is how EuroChat clients send call invites.
    fn build_status_evaluator(
        event_type: &'static str,
        status: Option<&'static str>,
    ) -> PushRuleEvaluator {
        build_call_evaluator(event_type, status, Some(""))
    }

    /// Like `build_status_evaluator`, with an explicit state key; `None` builds
    /// a timeline (non-state) event.
    fn build_call_evaluator(
        event_type: &'static str,
        status: Option<&'static str>,
        state_key: Option<&'static str>,
    ) -> PushRuleEvaluator {
        let mut flattened_keys = BTreeMap::new();
        flattened_keys.insert(
            "type".to_string(),
            JsonValue::Value(SimpleJsonValue::Str(Cow::Borrowed(event_type))),
        );
        if let Some(state_key) = state_key {
            flattened_keys.insert(
                "state_key".to_string(),
                JsonValue::Value(SimpleJsonValue::Str(Cow::Borrowed(state_key))),
            );
        }
        if let Some(status) = status {
            flattened_keys.insert(
                "content.status".to_string(),
                JsonValue::Value(SimpleJsonValue::Str(Cow::Borrowed(status))),
            );
        }

        PushRuleEvaluator::py_new(
            flattened_keys,
            false,
            10,
            Some(0),
            BTreeMap::new(),
            BTreeMap::new(),
            false,
            vec![],
            false,
            false,
            false,
        )
        .unwrap()
    }

    /// Builds an evaluator for an event of the given type whose `m.mentions`
    /// lists the given user IDs and optionally mentions the room, sent by a
    /// user with the given power level.
    fn build_evaluator(
        event_type: &'static str,
        mentioned_user_ids: &[&'static str],
        mentions_room: bool,
        sender_power_level: i64,
    ) -> PushRuleEvaluator {
        let mut flattened_keys = BTreeMap::new();
        flattened_keys.insert(
            "type".to_string(),
            JsonValue::Value(SimpleJsonValue::Str(Cow::Borrowed(event_type))),
        );
        flattened_keys.insert(
            r"content.m\.mentions.user_ids".to_string(),
            JsonValue::Array(
                mentioned_user_ids
                    .iter()
                    .map(|&user_id| SimpleJsonValue::Str(Cow::Borrowed(user_id)))
                    .collect(),
            ),
        );
        if mentions_room {
            flattened_keys.insert(
                r"content.m\.mentions.room".to_string(),
                JsonValue::Value(SimpleJsonValue::Bool(true)),
            );
        }

        PushRuleEvaluator::py_new(
            flattened_keys,
            true,
            10,
            Some(sender_power_level),
            BTreeMap::new(),
            BTreeMap::new(),
            false,
            vec![],
            false,
            false,
            false,
        )
        .unwrap()
    }

    #[test]
    fn test_rtc_invite_for_me_notifies_mentioned_user() {
        let evaluator = build_evaluator(RTC_NOTIFICATION_TYPE, &[ALICE], false, 0);

        let actions = evaluator.run(&push_rules(true), Some(ALICE), None, None);
        assert_eq!(actions, vec![Action::Notify, RING_ACTION]);
    }

    #[test]
    fn test_rtc_invite_for_me_ignores_other_users() {
        let evaluator = build_evaluator(RTC_NOTIFICATION_TYPE, &[BOB], false, 0);

        let actions = evaluator.run(&push_rules(true), Some(ALICE), None, None);
        assert_eq!(actions, vec![]);
    }

    #[test]
    fn test_rtc_invite_for_room_notifies_with_permission() {
        // The default `notifications.room` power level is 50.
        let evaluator = build_evaluator(RTC_NOTIFICATION_TYPE, &[], true, 50);

        let actions = evaluator.run(&push_rules(true), Some(ALICE), None, None);
        assert_eq!(actions, vec![Action::Notify, RING_ACTION]);
    }

    #[test]
    fn test_rtc_invite_for_room_requires_permission() {
        let evaluator = build_evaluator(RTC_NOTIFICATION_TYPE, &[], true, 0);

        let actions = evaluator.run(&push_rules(true), Some(ALICE), None, None);
        assert_eq!(actions, vec![]);
    }

    #[test]
    fn test_rtc_rules_ignore_other_event_types() {
        let evaluator = build_evaluator("org.example.other", &[ALICE], true, 50);

        let actions = evaluator.run(&push_rules(true), Some(ALICE), None, None);
        assert_eq!(actions, vec![]);
    }

    #[test]
    fn test_rtc_rules_are_omitted_when_msc4075_is_disabled() {
        let evaluator = build_evaluator(RTC_NOTIFICATION_TYPE, &[ALICE], true, 50);

        let actions = evaluator.run(&push_rules(false), Some(ALICE), None, None);
        assert_eq!(actions, vec![]);
    }

    #[test]
    fn test_wroote_call_invite_rings_when_enabled() {
        let evaluator = build_status_evaluator(EUROCHAT_CALL_INVITE_TYPE, Some("ringing"));

        let actions = evaluator.run(&push_rules_with(false, true), Some(ALICE), None, None);
        assert_eq!(
            actions,
            vec![Action::Notify, RING_ACTION, HIGHLIGHT_FALSE_ACTION]
        );
    }

    #[test]
    fn test_wroote_call_invite_ignores_cancellations() {
        // A cancellation has no `status` (and `active: false`); an ended call
        // has another status. Neither may ring.
        for status in [None, Some("ended")] {
            let evaluator = build_status_evaluator(EUROCHAT_CALL_INVITE_TYPE, status);

            let actions = evaluator.run(&push_rules_with(false, true), Some(ALICE), None, None);
            assert_eq!(actions, vec![], "status {status:?} must not ring");
        }
    }

    #[test]
    fn test_wroote_call_invite_ignores_timeline_events() {
        // Same type and content, but sent as a timeline event: must not ring,
        // otherwise any member with power level 0 could ring everyone.
        let evaluator = build_call_evaluator(EUROCHAT_CALL_INVITE_TYPE, Some("ringing"), None);

        let actions = evaluator.run(&push_rules_with(false, true), Some(ALICE), None, None);
        assert_eq!(actions, vec![]);
    }

    #[test]
    fn test_wroote_call_invite_ignores_non_empty_state_keys() {
        let evaluator =
            build_call_evaluator(EUROCHAT_CALL_INVITE_TYPE, Some("ringing"), Some("@bob:test"));

        let actions = evaluator.run(&push_rules_with(false, true), Some(ALICE), None, None);
        assert_eq!(actions, vec![]);
    }

    #[test]
    fn test_wroote_call_invite_ignores_other_event_types() {
        let evaluator = build_status_evaluator("org.example.other", Some("ringing"));

        let actions = evaluator.run(&push_rules_with(false, true), Some(ALICE), None, None);
        assert_eq!(actions, vec![]);
    }

    /// A user override rule that also matches call invites, like the rule the
    /// EuroChat API writes for the iOS call-signaling pusher.
    fn user_call_rule_with_priority_tweak() -> PushRule {
        PushRule {
            rule_id: Cow::Borrowed("global/override/eurochat.call-signaling"),
            priority_class: 5,
            conditions: Cow::Owned(vec![Condition::Known(KnownCondition::EventMatch(
                EventMatchCondition {
                    key: Cow::Borrowed("type"),
                    pattern: Cow::Borrowed(EUROCHAT_CALL_INVITE_TYPE),
                },
            ))]),
            actions: Cow::Owned(vec![
                Action::Notify,
                Action::SetTweak(SetTweak {
                    set_tweak: Cow::Borrowed("priority"),
                    value: Some(TweakValue::String(Cow::Borrowed("high"))),
                    other_keys: serde_json::Value::Null,
                }),
            ]),
            default: false,
            default_enabled: true,
        }
    }

    #[test]
    fn test_wroote_call_invite_wins_over_user_override_rules() {
        // The server rule must come before user overrides, so the `sound`
        // tweak (which makes the pusher send `prio: high`) is always applied.
        let evaluator = build_status_evaluator(EUROCHAT_CALL_INVITE_TYPE, Some("ringing"));
        let rules = push_rules_custom(
            false,
            true,
            vec![user_call_rule_with_priority_tweak()],
            BTreeMap::new(),
        );

        let actions = evaluator.run(&rules, Some(ALICE), None, None);
        assert_eq!(
            actions,
            vec![Action::Notify, RING_ACTION, HIGHLIGHT_FALSE_ACTION]
        );
    }

    #[test]
    fn test_wroote_call_invite_respects_master_rule_and_user_disable() {
        let evaluator = build_status_evaluator(EUROCHAT_CALL_INVITE_TYPE, Some("ringing"));

        // Master rule on: nothing notifies.
        let mut master_on = BTreeMap::new();
        master_on.insert("global/override/.m.rule.master".to_string(), true);
        let actions = evaluator.run(
            &push_rules_custom(false, true, vec![], master_on),
            Some(ALICE),
            None,
            None,
        );
        assert_eq!(actions, vec![]);

        // The user disabled the server rule: no ring from it.
        let mut rule_off = BTreeMap::new();
        rule_off.insert(WROOTE_VOIP_CALL_RULE_ID.to_string(), false);
        let actions = evaluator.run(
            &push_rules_custom(false, true, vec![], rule_off),
            Some(ALICE),
            None,
            None,
        );
        assert_eq!(actions, vec![]);
    }

    #[test]
    fn test_wroote_call_invite_rule_is_omitted_when_disabled() {
        let evaluator = build_status_evaluator(EUROCHAT_CALL_INVITE_TYPE, Some("ringing"));

        let actions = evaluator.run(&push_rules_with(false, false), Some(ALICE), None, None);
        assert_eq!(actions, vec![]);

        // The rule must not even be visible to clients when the option is off.
        let visible = push_rules_with(false, false)
            .rules()
            .iter()
            .any(|(rule, _)| rule.rule_id == WROOTE_VOIP_CALL_RULE_ID);
        assert!(!visible);
        let visible = push_rules_with(false, true)
            .rules()
            .iter()
            .any(|(rule, _)| rule.rule_id == WROOTE_VOIP_CALL_RULE_ID);
        assert!(visible);
    }
}
