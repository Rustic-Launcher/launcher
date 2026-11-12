# 0145. "Add a friend" dialog, exact lookup and friend suggestions from players met

- Status: accepted (cloud + launcher, 2026-11-11)
- Builds on: ADR 0019 (minors and chat), ADR 0087 (age gates, `capabilities`), ADR 0088 (realtime
  presence), ADR 0089 (GDPR), ADR 0022 (overlay policy). Legal: LEGAL.md §2.1, §2.2, §4.

## Context

The Friends tab only had an inline "Rustic username + Send" field: no way to check who you are
about to add, no way to add the player you just played with when you only know their in-game
(Minecraft) name, and nothing suggesting people met in parties or on servers. The API already
accepted `minecraft_name` on `POST /v1/friends/requests` (names come from `minecraft_accounts`,
linked when the player signs in to the launcher with Microsoft), but nothing recorded who was
where: presence lives in Redis with a 90 s TTL and the server address is only kept when the user
shares it.

Players are often minors; co-presence is location-like personal data (LEGAL §2.4 recommends a
DPIA for presence). Whatever we build must be minimal, transparent, opt-out-able and off for
minors, and must not let anyone probe who blocked them.

## Decision

**Dialog.** "Add a friend" is a modal dialog (product owner request; an explicit exception to ADR
0022, which reserves dialogs for confirmations: this flow has a focused task, a result to confirm,
and must open the same way from the Friends tab, the command menu and the empty friends list).
One host per window (`AddFriendHost`, opened through `useUi().addFriendOpen`): the launcher shell
and the detachable Social window. Radix gives the focus trap, Escape and the labelled title.

**Lookup UX: an explicit choice, not auto-detection.** A segmented control "Rustic name /
Minecraft name" above one field. Both namespaces use the same characters (`[A-Za-z0-9_]`), so the
same string can be two different people: guessing would send requests to the wrong person. The
choice is explicit, the field's label, placeholder and hint follow it, and a **result card**
(initials avatar, display name, `@username`, linked Minecraft name) is shown before "Send
request". `GET /v1/users/lookup?username=` | `?minecraft_name=`: exact, case-insensitive match
only (no prefix search), 30/min per user, and the **same checks and codes as
`friend_request_send`**: `not_found`, `cannot_add_self`, `friend_requests_disabled` (caller),
`cannot_add_friend` for blocked-either-way **and** "parental controls forbid requests"
(identical bodies, tested). Already friends / pending / incoming request are returned as
`relation` (the card shows "Already friends", "Request pending" or an "Accept" button) instead of
errors. A minor's linked Minecraft name is only echoed when it is what the caller typed.

**Suggestions.** `GET /v1/friends/suggestions` returns two lists of at most 10, most recent first:

- *Played with recently*: same party at overlapping times, or the same (shared) server for **at
  least 10 minutes in total** over the last 14 days (cumulative across sessions and servers;
  the reason shown is the server where they spent the most time, or "in your party").
- *Seen recently*: same server at overlapping times (any overlap, 60 s tolerance because writes
  are throttled) in the last 14 days, not already in the first list.

Excluded: yourself, friends, pending requests either way, blocks either way, inactive accounts,
anyone opted out, and anyone whose `capabilities` deny it (`discoverable_allowed` or
`friend_requests_allowed` false; re-checked at read time). Each item carries the user, the linked
Minecraft name, the reason (`party` | `server`), the **viewer's own** label for the server (from
the viewer's presence, so nothing about the other person's naming leaks) and `seen_at`.

**Data: `sightings`** (migration 0008): `user_id, kind (server|party), place_hash, label,
first_seen, last_seen`. `place_hash` = SHA-256 of `"rustic-sighting/v1\n<kind>\n<key>"` where the
key is the normalised server address (lower case, no trailing dot, no default port 25565, IPv6
brackets only kept with a port) or the party id; the raw address is never stored. Recorded from presence
updates (HTTP `PUT /v1/presence` and realtime heartbeats, both through `sightings::record`) only
when the status is online/away/busy, the activity is playing/hosting with a server address the
user **shares** (`presence::build` drops it otherwise), the account is active, the setting is on
and the account is adult; parties from current membership. Throttled to one write per user per
minute (store counter `sight:<user>`); a session row is extended while its `last_seen` is less
than 5 minutes old, otherwise a new row starts.

**Privacy.** New account setting `users.discoverable` ("Appear in suggestions of players I meet",
Settings › Privacy & data), **default on for adults** (legal basis: legitimate interest with an
easy opt-out, LEGAL §2.1), exposed as `Me.discoverable` (effective value) and
`capabilities.discoverable_allowed` = not a minor. Minors (under 18, guardian or not, unknown date
of birth included) are **never recorded nor suggested** and cannot opt in
(`discoverable_not_allowed`). Turning the setting off deletes the user's sightings at once and
empties their own suggestions (`discoverable: false`). Retention **30 days** after `last_seen`
(`jobs::cleanup` → `sightings::purge_expired`). GDPR: the table is in the export (`sightings`,
hash in hex), erased at the deletion request and cascaded at the purge.

## Consequences

- The launcher shows suggestions only when both sides allow it; users not sharing servers in their
  activity only appear through parties.
- The realtime service now needs the age rules too (`AGE_RULES_JSON`, same value as the API).
- Hashing is pseudonymisation, not anonymisation: a public address (or an IPv4) can be found again
  by hashing candidates. The table is therefore treated as personal data (retention, export,
  erasure, opt-out); a keyed hash (HMAC with a server-side pepper shared by the API and the
  realtime service) is the next hardening step and needs a new deployment secret.
- `label` stores the user's own server name (max 64 chars), shown only to that user.
- The website's generated client (`apps/web/src/api/schema.d.ts`) is regenerated by its owner.

## Alternatives

- Auto-detecting Rustic vs Minecraft names, or searching both: ambiguous and could add the wrong
  person; rejected for an explicit choice.
- Prefix search in the dialog (like `/v1/users/search`): enumerates accounts; the dialog only needs
  an exact match.
- Storing raw addresses or the presence history: more than needed; rejected.
- Opt-in by default for adults: safer but suggestions would stay empty for nearly everyone; the
  balancing test favours opt-out with transparent wording and immediate erasure. Minors stay off.
- Computing suggestions from live presence only (no table): misses everyone who left a minute ago;
  rejected.
