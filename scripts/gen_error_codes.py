"""Generates `crates/djs-model/src/rest/error_code.rs` (D-097, D-098, C-041).

Joins the JSON error code table of the Discord API docs with the names of discord-api-types'
`RESTJSONErrorCodes`, writes acronyms as words (C-021) and uses the library's own descriptions
below (D-004); a new documented code needs a description here before the script succeeds.

Usage (from the repository root, with the references cloned into the git-ignored `.reference/`):

    python3 -I scripts/gen_error_codes.py \
        .reference/discord-api-docs/developers/topics/opcodes-and-status-codes.mdx \
        .reference/discord-api-types/rest/common.ts \
        crates/djs-model/src/rest/error_code.rs
"""
import json
import re
import sys

def load_rows(docs_path, types_path):
    docs = {}
    text = open(docs_path, encoding='utf-8').read()
    section = text[text.index('###### JSON Error Codes'):text.index('###### Example JSON Error Response')]
    for line in section.splitlines():
        m = re.match(r'^\|\s*(\d+)\s*\|\s*(.*?)\s*\|\s*$', line)
        if m:
            docs[int(m.group(1))] = m.group(2)
    types = {}
    src = open(types_path, encoding='utf-8').read()
    body = src[src.index('export enum RESTJSONErrorCodes'):]
    body = body[body.index('{')+1:body.index('\n}')]
    value = -1
    for line in body.splitlines():
        line = line.strip()
        if not line or line.startswith('//') or line.startswith('*') or line.startswith('/**'):
            continue
        m = re.match(r'^([A-Za-z0-9_]+)(?:\s*=\s*([\d_]+))?,?$', line)
        if not m:
            print('skip', line, file=sys.stderr); continue
        value = int(m.group(2).replace('_','')) if m.group(2) else value + 1
        types.setdefault(value, m.group(1))
    rows = []
    for code in sorted(set(docs) | set(types)):
        rows.append({'code': code, 'name': types.get(code), 'docs': docs.get(code)})
    print('docs', len(docs), 'types', len(types), 'only-docs', [r['code'] for r in rows if r['name'] is None], 'only-types', [r['code'] for r in rows if r['docs'] is None], file=sys.stderr)
    return rows


docs_path, types_path, out_path = sys.argv[1:4]
rows = load_rows(docs_path, types_path)

RENAME = {
    'UnknownSKU': 'UnknownSku',
    'MaximumNumberOfGroupDMsReached': 'MaximumNumberOfGroupDmsReached',
    'OnlyConsumableSKUsCanBeConsumed': 'OnlyConsumableSkusCanBeConsumed',
    'CannotExecuteActionOnDMChannel': 'CannotExecuteActionOnDmChannel',
    'InvalidMFALevel': 'InvalidMfaLevel',
    'InvalidAPIVersion': 'InvalidApiVersion',
    'InvalidSKU': 'InvalidSku',
    'RequestBodyContainsInvalidJSON': 'RequestBodyContainsInvalidJson',
    'APIResourceOverloaded': 'ApiResourceOverloaded',
    'NSFWChannelMessageReferenceNotAllowed': 'NsfwChannelMessageReferenceNotAllowed',
    'InvalidJSONForUploadedLottieFile': 'InvalidJsonForUploadedLottieFile',
    'IdTokenJWTExpired': 'IdTokenJwtExpired',
    'IdTokenJWTIssuerMismatch': 'IdTokenJwtIssuerMismatch',
    'IdTokenJWTAudienceMismatch': 'IdTokenJwtAudienceMismatch',
    'IdTokenJWTIssuedTooLongAgo': 'IdTokenJwtIssuedTooLongAgo',
}

# Documented codes discord-api-types does not name yet (names derived from the docs, D-098).
OWN_NAMES = {
    30084: 'InviteTargetUsersLimitReached',
    40114: 'InviteTargetUsersAlreadyBeingProcessed',
    40115: 'InviteTargetUsersFileNotProcessed',
}

# "Unknown X" codes: the resource named by the request does not exist (for the bot).
UNKNOWN = {
    10001: 'account', 10002: 'application', 10003: 'channel', 10004: 'guild',
    10005: 'integration', 10006: 'invite', 10007: 'guild member', 10008: 'message',
    10009: 'permission overwrite', 10010: 'provider', 10011: 'role', 10012: 'token',
    10013: 'user', 10014: 'emoji', 10015: 'webhook', 10016: 'webhook service',
    10020: 'session', 10021: 'asset', 10026: 'ban', 10027: 'SKU', 10028: 'store listing',
    10029: 'entitlement', 10030: 'build', 10031: 'lobby', 10032: 'branch',
    10033: 'store directory layout', 10036: 'redistributable', 10038: 'gift code',
    10049: 'stream', 10050: 'premium server subscription cooldown', 10057: 'guild template',
    10059: 'discoverable server category', 10060: 'sticker', 10061: 'sticker pack',
    10062: 'interaction', 10063: 'application command', 10065: 'voice state',
    10066: 'application command permissions', 10067: 'stage instance',
    10068: 'member verification form', 10069: 'welcome screen',
    10070: 'scheduled event', 10071: 'scheduled event user', 10087: 'forum tag',
    10097: 'soundboard sound',
}

# "Maximum number of X reached" codes: (what, limit or None).
MAXIMUM = {
    30001: ('guilds the user is in', '100'),
    30002: ('friends', '1000'),
    30003: ('pinned messages in the channel', '250'),
    30004: ('group DM recipients', '10'),
    30005: ('roles in the guild', '250'),
    30007: ('webhooks in the channel', '15'),
    30008: ('emojis in the guild', None),
    30010: ('different reactions on the message', '20'),
    30011: ('group DMs', '10'),
    30013: ('channels in the guild', '500'),
    30015: ('attachments in one message', '10'),
    30016: ('invites', '1000'),
    30018: ('animated emojis in the guild', None),
    30019: ('members in the guild', None),
    30030: ('server categories', '5'),
    30032: ('application commands', None),
    30033: ('thread participants', '1000'),
    30034: ('application commands created per day', '200'),
    30035: ('bans of users who are not members', None),
    30037: ('ban list fetches', None),
    30038: ('scheduled events that have not ended', '100'),
    30039: ('stickers in the guild', None),
    30045: ('soundboard sounds', None),
    30047: ('pinned threads in the forum channel', None),
    30048: ('tags in the forum channel', None),
    30056: ('premium (subscription) emojis', '25'),
    30058: ('webhooks in the guild', '1000'),
    30060: ('permission overwrites on the channel', '1000'),
}

DESC = {
    0: 'A general error, e.g. a malformed request body.',
    10124: 'The invite exists but no job for its target users was found.',
    10129: 'The invite exists but has no target users.',
    20001: 'This endpoint is not available to bots.',
    20002: 'This endpoint is only available to bots.',
    20009: 'Explicit content cannot be sent to the recipient(s).',
    20012: 'The current user may not perform this action on the application.',
    20015: 'The action needs a premium subscription.',
    20016: 'The channel\'s slowmode does not allow the action yet.',
    20018: 'Only the account\'s owner can perform this action.',
    20022: 'The message cannot be edited because of the announcement rate limits.',
    20024: 'The user is below the minimum age.',
    20028: 'The channel\'s write rate limit was hit.',
    20029: 'The server\'s write rate limit was hit.',
    20031: 'The Stage topic, server name, server description or a channel name contains '
           'disallowed words.',
    20035: 'The guild\'s boost level is too low.',
    30031: 'The guild already has a template.',
    30040: 'Too many prune requests; try again later.',
    30042: 'Too many guild widget settings updates; try again later.',
    30046: 'Too many edits to messages older than one hour; try again later.',
    30052: 'The bitrate is too high for this type of channel.',
    30061: 'The guild\'s channels are too large.',
    30084: 'The guild\'s limit of invite target users was reached.',
    40001: 'Unauthorized: send a valid token.',
    40002: 'The account must be verified first.',
    40003: 'Direct messages are being opened too quickly.',
    40004: 'Sending messages is temporarily disabled.',
    40005: 'The request is too large; send something smaller.',
    40006: 'This feature is temporarily disabled on Discord\'s side.',
    40007: 'The user is banned from this guild.',
    40009: 'The `parent_id` of only one channel can be changed at a time.',
    40012: 'The connection was revoked.',
    40018: 'Only consumable SKUs can be consumed.',
    40019: 'Only test (sandbox) entitlements can be deleted.',
    40032: 'The target user is not in a voice channel.',
    40033: 'The message was already crossposted (published).',
    40041: 'An application command with this name already exists.',
    40043: 'The application interaction could not be sent.',
    40058: 'Messages cannot be sent in a forum channel (create a post instead).',
    40060: 'The interaction was already acknowledged (responded to).',
    40061: 'Forum tag names must be unique.',
    40062: 'The service resource is rate limited.',
    40066: 'No forum tags are available that non-moderators can set.',
    40067: 'Creating a post in this forum channel requires a tag.',
    40074: 'An entitlement was already granted for this resource.',
    40094: 'The interaction already has the maximum number of follow-up messages.',
    40114: 'The invite\'s target users are already being processed.',
    40115: 'The invite\'s target users file was not processed yet.',
    40333: 'Cloudflare blocked the request; a proper '
           '[User-Agent](https://docs.discord.com/developers/reference#user-agent) often fixes it.',
    50001: 'The bot has no access to the resource (e.g. it cannot see the channel).',
    50002: 'The account type is not valid for this action.',
    50003: 'The action is not possible in a DM channel.',
    50004: 'The guild\'s widget is disabled.',
    50005: 'Messages of other users cannot be edited.',
    50006: 'A message cannot be empty.',
    50007: 'The user does not accept messages from the bot (e.g. DMs are closed).',
    50008: 'Messages can only be sent in text-based channels.',
    50009: 'The channel\'s verification level is too high for the user.',
    50010: 'The OAuth2 application has no bot.',
    50011: 'The limit of OAuth2 applications was reached.',
    50012: 'The OAuth2 state is invalid.',
    50013: 'The bot lacks the permissions for this action.',
    50014: 'The authentication token is invalid.',
    50015: 'The note is too long.',
    50016: 'Bulk deletes take between 2 and 100 messages.',
    50017: 'The MFA level is invalid.',
    50019: 'A message can only be pinned in its own channel.',
    50020: 'The invite code is invalid or already taken.',
    50021: 'The action is not possible on a system message.',
    50024: 'The action is not possible on this type of channel.',
    50025: 'The OAuth2 access token is invalid.',
    50026: 'A required OAuth2 scope is missing.',
    50027: 'The webhook token is invalid.',
    50028: 'The role is invalid.',
    50033: 'The recipient(s) are invalid.',
    50034: 'A message is too old to be bulk deleted (two weeks at most).',
    50035: 'The form body (JSON or multipart) or the `Content-Type` is invalid; the field '
           'errors tell which fields.',
    50036: 'An invite was accepted to a guild the application\'s bot is not in.',
    50039: 'The activity action is invalid.',
    50041: 'The API version is invalid.',
    50045: 'The uploaded file is larger than allowed.',
    50046: 'The uploaded file is invalid.',
    50054: 'The gift cannot be redeemed by its own sender.',
    50055: 'The guild is invalid.',
    50057: 'The SKU is invalid.',
    50067: 'The request origin is invalid.',
    50068: 'The message type is invalid.',
    50070: 'Redeeming the gift requires a payment source.',
    50073: 'System webhooks cannot be modified.',
    50074: 'Community guilds need this channel, so it cannot be deleted.',
    50080: 'The stickers of a message cannot be edited.',
    50081: 'The sticker is invalid.',
    50083: 'The thread is archived, so the operation (e.g. editing a message or adding a member) '
           'is not possible.',
    50084: 'The thread notification settings are invalid.',
    50085: 'The `before` value is earlier than the thread\'s creation.',
    50086: 'Channels of community servers must be text channels.',
    50091: 'The event\'s entity type differs from the entity it is being started for.',
    50095: 'The server is not available in the user\'s location.',
    50097: 'The server needs monetization enabled for this action.',
    50101: 'The server needs more boosts for this action.',
    50109: 'The request body is not valid JSON.',
    50110: 'The file is invalid.',
    50123: 'The file type is invalid.',
    50124: 'The file is longer than the maximum duration (5.2 seconds).',
    50131: 'The owner cannot be a pending member.',
    50132: 'A bot user cannot become the owner.',
    50138: 'The asset could not be resized below the maximum size (262 144).',
    50144: 'An emoji cannot mix subscription and non-subscription roles.',
    50145: 'An emoji cannot be converted between premium and normal.',
    50146: 'The uploaded file was not found.',
    50151: 'The emoji is invalid.',
    50159: 'A voice message cannot have other content.',
    50160: 'A voice message needs exactly one audio attachment.',
    50161: 'A voice message needs its metadata (duration and waveform).',
    50162: 'Voice messages cannot be edited.',
    50163: 'The guild subscription integration cannot be deleted.',
    50167: 'No voice effect can be sent while the user is server muted, deafened or '
           'suppressed.',
    50173: 'Voice messages cannot be sent in this channel.',
    50178: 'The user account must be verified first.',
    50192: 'The file has no valid duration.',
    50278: 'The user shares no guild with the bot, so it cannot message them.',
    50600: 'The bot may not send this sticker.',
    60003: 'The operation requires two-factor authentication.',
    80004: 'No user with this Discord tag exists.',
    90001: 'The reaction was blocked.',
    90002: 'The user cannot use super (burst) reactions.',
    110000: 'The index is not available yet; try again later.',
    110001: 'The application is not available yet; try again later.',
    130000: 'The API resource is overloaded; try again a little later.',
    150006: 'The Stage is already open.',
    160002: 'Replying requires the permission to read the message history.',
    160004: 'This message already has a thread.',
    160005: 'The thread is locked.',
    160006: 'The limit of active threads was reached.',
    160007: 'The limit of active announcement threads was reached.',
    160009: 'Referencing a message requires the permission to read the message history.',
    160010: 'Messages of NSFW channels cannot be referenced here.',
    160014: 'A message whose content the user cannot read cannot be forwarded.',
    170001: 'The uploaded Lottie file is not valid JSON.',
    170002: 'An uploaded Lottie cannot contain raster images (e.g. PNG or JPEG).',
    170003: 'The sticker\'s frame rate is above the maximum.',
    170004: 'The sticker has more than 1000 frames.',
    170005: 'The Lottie animation is larger than the maximum dimensions.',
    170006: 'The sticker\'s frame rate is too low or too high.',
    170007: 'The sticker\'s animation is longer than 5 seconds.',
    180000: 'A finished event cannot be updated.',
    180002: 'The Stage for the Stage event could not be created.',
    200000: 'AutoMod blocked the message.',
    200001: 'AutoMod blocked the title.',
    220001: 'A webhook message in a forum channel needs a `thread_name` or a `thread_id`.',
    220002: 'A webhook message in a forum channel cannot have both a `thread_name` and a '
            '`thread_id`.',
    220003: 'Webhooks can only create threads in forum channels.',
    220004: 'Webhook services cannot be used in forum channels.',
    240000: 'The harmful links filter blocked the message.',
    340015: 'The user is limited from joining new servers.',
    350000: 'Onboarding cannot be enabled: its requirements are not met.',
    350001: 'Onboarding cannot be updated while it is below the requirements.',
    400001: 'File uploads are limited for this guild.',
    500000: 'The users could not be banned.',
    520000: 'Voting in the poll is blocked.',
    520001: 'The poll has expired.',
    520002: 'Polls cannot be created in this type of channel.',
    520003: 'A poll message cannot be edited.',
    520004: 'An emoji that is part of the poll cannot be used.',
    520006: 'Only poll messages can be expired.',
    530000: 'The application may not use provisional accounts.',
    530001: 'The ID token (JWT) has expired; get a new one from the identity provider.',
    530002: 'The ID token\'s issuer does not match the configured one.',
    530003: 'The ID token\'s audience does not match the OIDC configuration; fix the '
            'configuration or use a token issued to the application.',
    530004: 'The ID token was issued more than a week ago; get a new one from the identity '
            'provider.',
    530006: 'No unique username could be generated in time; retrying usually works.',
    530007: 'The client secret is invalid; check it or create a new one.',
}


def description(code):
    if code in DESC:
        return DESC[code]
    if code in UNKNOWN:
        return f'The {UNKNOWN[code]} was not found.'
    if code in MAXIMUM:
        what, limit = MAXIMUM[code]
        return f'The limit of {what}{f" ({limit})" if limit else ""} was reached.'
    raise SystemExit(f'no description for {code}')


def wrap(text, indent, width=100):
    words = text.split(' ')
    lines, line = [], indent + '///'
    for word in words:
        if len(line) + 1 + len(word) > width and line != indent + '///':
            lines.append(line)
            line = indent + '///'
        line += ' ' + word
    lines.append(line)
    return lines


variants = []
for row in rows:
    code, docs = row['code'], row['docs']
    if docs is None:
        continue  # not documented (D-097 covers documented codes)
    original = row['name'] or OWN_NAMES[code]
    name = RENAME.get(original, original)
    assert re.fullmatch(r'[A-Z][A-Za-z0-9]*', name), name
    text = description(code).replace('OAuth2', '`OAuth2`').replace('AutoMod', '`AutoMod`')
    lines = wrap(f'`{code}`: {text}', '        ')
    if row['name'] and name != original:
        lines.append(f'        #[doc(alias = "{original}")]')
    lines.append(f'        #[doc(alias = "{code}")]')
    literal = f'{code:_}' if code >= 10_000 else str(code)
    lines.append(f'        {name} = {literal},')
    variants.append('\n'.join(lines))

names = [v.splitlines()[-1].strip().split(' ')[0] for v in variants]
assert len(names) == len(set(names)), 'duplicate names'

out = '''//! [JSON error codes](https://docs.discord.com/developers/topics/opcodes-and-status-codes#json-json-error-codes)
//! of REST API error responses.
//!
//! Generated from the docs' table, with names from discord-api-types (D-097, D-098); the
//! descriptions are the library's own.

use crate::macros::int_enum;

int_enum! {
    /// The [JSON error code](https://docs.discord.com/developers/topics/opcodes-and-status-codes#json-json-error-codes)
    /// of an error response: what went wrong, in more detail than the HTTP status (D-097).
    ///
    /// Variants are named like discord-api-types' `RESTJSONErrorCodes` (D-098), with acronyms
    /// written as words (`UnknownSku`); rustdoc search also finds them by their original name
    /// and by their number. Codes added by Discord later decode as
    /// [`Unknown`](Self::Unknown).
    ///
    /// discord.js counterpart: `RESTJSONErrorCodes`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::rest::ErrorCode;
    ///
    /// let code: ErrorCode = serde_json::from_str("50013").unwrap();
    /// assert_eq!(code, ErrorCode::MissingPermissions);
    /// assert_eq!(code.value(), 50013);
    /// assert_eq!(ErrorCode::from(10008), ErrorCode::UnknownMessage);
    /// assert!(ErrorCode::from(1).is_unknown());
    /// ```
    #[doc(alias = "RESTJSONErrorCodes")]
    pub enum ErrorCode: u32 {
''' + '\n'.join(variants) + '''
    }
}
'''
open(out_path, 'w', encoding='utf-8').write(out)
print(len(variants), 'variants', file=sys.stderr)
