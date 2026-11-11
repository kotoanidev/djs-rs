//! [JSON error codes](https://docs.discord.com/developers/topics/opcodes-and-status-codes#json-json-error-codes)
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
        /// `0`: A general error, e.g. a malformed request body.
        #[doc(alias = "0")]
        GeneralError = 0,
        /// `10001`: The account was not found.
        #[doc(alias = "10001")]
        UnknownAccount = 10_001,
        /// `10002`: The application was not found.
        #[doc(alias = "10002")]
        UnknownApplication = 10_002,
        /// `10003`: The channel was not found.
        #[doc(alias = "10003")]
        UnknownChannel = 10_003,
        /// `10004`: The guild was not found.
        #[doc(alias = "10004")]
        UnknownGuild = 10_004,
        /// `10005`: The integration was not found.
        #[doc(alias = "10005")]
        UnknownIntegration = 10_005,
        /// `10006`: The invite was not found.
        #[doc(alias = "10006")]
        UnknownInvite = 10_006,
        /// `10007`: The guild member was not found.
        #[doc(alias = "10007")]
        UnknownMember = 10_007,
        /// `10008`: The message was not found.
        #[doc(alias = "10008")]
        UnknownMessage = 10_008,
        /// `10009`: The permission overwrite was not found.
        #[doc(alias = "10009")]
        UnknownPermissionOverwrite = 10_009,
        /// `10010`: The provider was not found.
        #[doc(alias = "10010")]
        UnknownProvider = 10_010,
        /// `10011`: The role was not found.
        #[doc(alias = "10011")]
        UnknownRole = 10_011,
        /// `10012`: The token was not found.
        #[doc(alias = "10012")]
        UnknownToken = 10_012,
        /// `10013`: The user was not found.
        #[doc(alias = "10013")]
        UnknownUser = 10_013,
        /// `10014`: The emoji was not found.
        #[doc(alias = "10014")]
        UnknownEmoji = 10_014,
        /// `10015`: The webhook was not found.
        #[doc(alias = "10015")]
        UnknownWebhook = 10_015,
        /// `10016`: The webhook service was not found.
        #[doc(alias = "10016")]
        UnknownWebhookService = 10_016,
        /// `10020`: The session was not found.
        #[doc(alias = "10020")]
        UnknownSession = 10_020,
        /// `10021`: The asset was not found.
        #[doc(alias = "10021")]
        UnknownAsset = 10_021,
        /// `10026`: The ban was not found.
        #[doc(alias = "10026")]
        UnknownBan = 10_026,
        /// `10027`: The SKU was not found.
        #[doc(alias = "UnknownSKU")]
        #[doc(alias = "10027")]
        UnknownSku = 10_027,
        /// `10028`: The store listing was not found.
        #[doc(alias = "10028")]
        UnknownStoreListing = 10_028,
        /// `10029`: The entitlement was not found.
        #[doc(alias = "10029")]
        UnknownEntitlement = 10_029,
        /// `10030`: The build was not found.
        #[doc(alias = "10030")]
        UnknownBuild = 10_030,
        /// `10031`: The lobby was not found.
        #[doc(alias = "10031")]
        UnknownLobby = 10_031,
        /// `10032`: The branch was not found.
        #[doc(alias = "10032")]
        UnknownBranch = 10_032,
        /// `10033`: The store directory layout was not found.
        #[doc(alias = "10033")]
        UnknownStoreDirectoryLayout = 10_033,
        /// `10036`: The redistributable was not found.
        #[doc(alias = "10036")]
        UnknownRedistributable = 10_036,
        /// `10038`: The gift code was not found.
        #[doc(alias = "10038")]
        UnknownGiftCode = 10_038,
        /// `10049`: The stream was not found.
        #[doc(alias = "10049")]
        UnknownStream = 10_049,
        /// `10050`: The premium server subscription cooldown was not found.
        #[doc(alias = "10050")]
        UnknownPremiumServerSubscribeCooldown = 10_050,
        /// `10057`: The guild template was not found.
        #[doc(alias = "10057")]
        UnknownGuildTemplate = 10_057,
        /// `10059`: The discoverable server category was not found.
        #[doc(alias = "10059")]
        UnknownDiscoverableServerCategory = 10_059,
        /// `10060`: The sticker was not found.
        #[doc(alias = "10060")]
        UnknownSticker = 10_060,
        /// `10061`: The sticker pack was not found.
        #[doc(alias = "10061")]
        UnknownStickerPack = 10_061,
        /// `10062`: The interaction was not found.
        #[doc(alias = "10062")]
        UnknownInteraction = 10_062,
        /// `10063`: The application command was not found.
        #[doc(alias = "10063")]
        UnknownApplicationCommand = 10_063,
        /// `10065`: The voice state was not found.
        #[doc(alias = "10065")]
        UnknownVoiceState = 10_065,
        /// `10066`: The application command permissions was not found.
        #[doc(alias = "10066")]
        UnknownApplicationCommandPermissions = 10_066,
        /// `10067`: The stage instance was not found.
        #[doc(alias = "10067")]
        UnknownStageInstance = 10_067,
        /// `10068`: The member verification form was not found.
        #[doc(alias = "10068")]
        UnknownGuildMemberVerificationForm = 10_068,
        /// `10069`: The welcome screen was not found.
        #[doc(alias = "10069")]
        UnknownGuildWelcomeScreen = 10_069,
        /// `10070`: The scheduled event was not found.
        #[doc(alias = "10070")]
        UnknownGuildScheduledEvent = 10_070,
        /// `10071`: The scheduled event user was not found.
        #[doc(alias = "10071")]
        UnknownGuildScheduledEventUser = 10_071,
        /// `10087`: The forum tag was not found.
        #[doc(alias = "10087")]
        UnknownTag = 10_087,
        /// `10097`: The soundboard sound was not found.
        #[doc(alias = "10097")]
        UnknownSound = 10_097,
        /// `10124`: The invite exists but no job for its target users was found.
        #[doc(alias = "10124")]
        UnknownInviteTargetUsersJob = 10_124,
        /// `10129`: The invite exists but has no target users.
        #[doc(alias = "10129")]
        UnknownInviteTargetUsers = 10_129,
        /// `20001`: This endpoint is not available to bots.
        #[doc(alias = "20001")]
        BotsCannotUseThisEndpoint = 20_001,
        /// `20002`: This endpoint is only available to bots.
        #[doc(alias = "20002")]
        OnlyBotsCanUseThisEndpoint = 20_002,
        /// `20009`: Explicit content cannot be sent to the recipient(s).
        #[doc(alias = "20009")]
        ExplicitContentCannotBeSentToTheDesiredRecipient = 20_009,
        /// `20012`: The current user may not perform this action on the application.
        #[doc(alias = "20012")]
        NotAuthorizedToPerformThisActionOnThisApplication = 20_012,
        /// `20015`: The action needs a premium subscription.
        #[doc(alias = "20015")]
        ThisActionRequiresAPremiumSubscription = 20_015,
        /// `20016`: The channel's slowmode does not allow the action yet.
        #[doc(alias = "20016")]
        ActionCannotBePerformedDueToSlowmodeRateLimit = 20_016,
        /// `20018`: Only the account's owner can perform this action.
        #[doc(alias = "20018")]
        OnlyTheOwnerOfThisAccountCanPerformThisAction = 20_018,
        /// `20022`: The message cannot be edited because of the announcement rate limits.
        #[doc(alias = "20022")]
        AnnouncementEditLimitExceeded = 20_022,
        /// `20024`: The user is below the minimum age.
        #[doc(alias = "20024")]
        UnderMinimumAge = 20_024,
        /// `20028`: The channel's write rate limit was hit.
        #[doc(alias = "20028")]
        ChannelWriteRateLimit = 20_028,
        /// `20029`: The server's write rate limit was hit.
        #[doc(alias = "20029")]
        ServerWriteRateLimit = 20_029,
        /// `20031`: The Stage topic, server name, server description or a channel name contains
        /// disallowed words.
        #[doc(alias = "20031")]
        StageTopicServerNameServerDescriptionOrChannelNamesContainDisallowedWords = 20_031,
        /// `20035`: The guild's boost level is too low.
        #[doc(alias = "20035")]
        GuildPremiumSubscriptionLevelTooLow = 20_035,
        /// `30001`: The limit of guilds the user is in (100) was reached.
        #[doc(alias = "30001")]
        MaximumNumberOfGuildsReached = 30_001,
        /// `30002`: The limit of friends (1000) was reached.
        #[doc(alias = "30002")]
        MaximumNumberOfFriendsReached = 30_002,
        /// `30003`: The limit of pinned messages in the channel (250) was reached.
        #[doc(alias = "30003")]
        MaximumNumberOfPinsReachedForTheChannel = 30_003,
        /// `30004`: The limit of group DM recipients (10) was reached.
        #[doc(alias = "30004")]
        MaximumNumberOfRecipientsReached = 30_004,
        /// `30005`: The limit of roles in the guild (250) was reached.
        #[doc(alias = "30005")]
        MaximumNumberOfGuildRolesReached = 30_005,
        /// `30007`: The limit of webhooks in the channel (15) was reached.
        #[doc(alias = "30007")]
        MaximumNumberOfWebhooksReached = 30_007,
        /// `30008`: The limit of emojis in the guild was reached.
        #[doc(alias = "30008")]
        MaximumNumberOfEmojisReached = 30_008,
        /// `30010`: The limit of different reactions on the message (20) was reached.
        #[doc(alias = "30010")]
        MaximumNumberOfReactionsReached = 30_010,
        /// `30011`: The limit of group DMs (10) was reached.
        #[doc(alias = "MaximumNumberOfGroupDMsReached")]
        #[doc(alias = "30011")]
        MaximumNumberOfGroupDmsReached = 30_011,
        /// `30013`: The limit of channels in the guild (500) was reached.
        #[doc(alias = "30013")]
        MaximumNumberOfGuildChannelsReached = 30_013,
        /// `30015`: The limit of attachments in one message (10) was reached.
        #[doc(alias = "30015")]
        MaximumNumberOfAttachmentsInAMessageReached = 30_015,
        /// `30016`: The limit of invites (1000) was reached.
        #[doc(alias = "30016")]
        MaximumNumberOfInvitesReached = 30_016,
        /// `30018`: The limit of animated emojis in the guild was reached.
        #[doc(alias = "30018")]
        MaximumNumberOfAnimatedEmojisReached = 30_018,
        /// `30019`: The limit of members in the guild was reached.
        #[doc(alias = "30019")]
        MaximumNumberOfServerMembersReached = 30_019,
        /// `30030`: The limit of server categories (5) was reached.
        #[doc(alias = "30030")]
        MaximumNumberOfServerCategoriesReached = 30_030,
        /// `30031`: The guild already has a template.
        #[doc(alias = "30031")]
        GuildAlreadyHasTemplate = 30_031,
        /// `30032`: The limit of application commands was reached.
        #[doc(alias = "30032")]
        MaximumNumberOfApplicationCommandsReached = 30_032,
        /// `30033`: The limit of thread participants (1000) was reached.
        #[doc(alias = "30033")]
        MaximumThreadParticipantsReached = 30_033,
        /// `30034`: The limit of application commands created per day (200) was reached.
        #[doc(alias = "30034")]
        MaximumDailyApplicationCommandCreatesReached = 30_034,
        /// `30035`: The limit of bans of users who are not members was reached.
        #[doc(alias = "30035")]
        MaximumNumberOfNonGuildMemberBansHasBeenExceeded = 30_035,
        /// `30037`: The limit of ban list fetches was reached.
        #[doc(alias = "30037")]
        MaximumNumberOfBanFetchesHasBeenReached = 30_037,
        /// `30038`: The limit of scheduled events that have not ended (100) was reached.
        #[doc(alias = "30038")]
        MaximumNumberOfUncompletedGuildScheduledEventsReached = 30_038,
        /// `30039`: The limit of stickers in the guild was reached.
        #[doc(alias = "30039")]
        MaximumNumberOfStickersReached = 30_039,
        /// `30040`: Too many prune requests; try again later.
        #[doc(alias = "30040")]
        MaximumNumberOfPruneRequestsHasBeenReached = 30_040,
        /// `30042`: Too many guild widget settings updates; try again later.
        #[doc(alias = "30042")]
        MaximumNumberOfGuildWidgetSettingsUpdatesHasBeenReached = 30_042,
        /// `30045`: The limit of soundboard sounds was reached.
        #[doc(alias = "30045")]
        MaximumNumberOfSoundboardSoundsReached = 30_045,
        /// `30046`: Too many edits to messages older than one hour; try again later.
        #[doc(alias = "30046")]
        MaximumNumberOfEditsToMessagesOlderThanOneHourReached = 30_046,
        /// `30047`: The limit of pinned threads in the forum channel was reached.
        #[doc(alias = "30047")]
        MaximumNumberOfPinnedThreadsInForumHasBeenReached = 30_047,
        /// `30048`: The limit of tags in the forum channel was reached.
        #[doc(alias = "30048")]
        MaximumNumberOfTagsInForumHasBeenReached = 30_048,
        /// `30052`: The bitrate is too high for this type of channel.
        #[doc(alias = "30052")]
        BitrateIsTooHighForChannelOfThisType = 30_052,
        /// `30056`: The limit of premium (subscription) emojis (25) was reached.
        #[doc(alias = "30056")]
        MaximumNumberOfPremiumEmojisReached = 30_056,
        /// `30058`: The limit of webhooks in the guild (1000) was reached.
        #[doc(alias = "30058")]
        MaximumNumberOfWebhooksPerGuildReached = 30_058,
        /// `30060`: The limit of permission overwrites on the channel (1000) was reached.
        #[doc(alias = "30060")]
        MaximumNumberOfChannelPermissionOverwritesReached = 30_060,
        /// `30061`: The guild's channels are too large.
        #[doc(alias = "30061")]
        TheChannelsForThisGuildAreTooLarge = 30_061,
        /// `30084`: The guild's limit of invite target users was reached.
        #[doc(alias = "30084")]
        InviteTargetUsersLimitReached = 30_084,
        /// `40001`: Unauthorized: send a valid token.
        #[doc(alias = "40001")]
        Unauthorized = 40_001,
        /// `40002`: The account must be verified first.
        #[doc(alias = "40002")]
        VerifyYourAccount = 40_002,
        /// `40003`: Direct messages are being opened too quickly.
        #[doc(alias = "40003")]
        OpeningDirectMessagesTooFast = 40_003,
        /// `40004`: Sending messages is temporarily disabled.
        #[doc(alias = "40004")]
        SendMessagesHasBeenTemporarilyDisabled = 40_004,
        /// `40005`: The request is too large; send something smaller.
        #[doc(alias = "40005")]
        RequestEntityTooLarge = 40_005,
        /// `40006`: This feature is temporarily disabled on Discord's side.
        #[doc(alias = "40006")]
        FeatureTemporarilyDisabledServerSide = 40_006,
        /// `40007`: The user is banned from this guild.
        #[doc(alias = "40007")]
        UserBannedFromThisGuild = 40_007,
        /// `40009`: The `parent_id` of only one channel can be changed at a time.
        #[doc(alias = "40009")]
        OnlyOneChannelCanHaveAParentIdModifiedAtATime = 40_009,
        /// `40012`: The connection was revoked.
        #[doc(alias = "40012")]
        ConnectionHasBeenRevoked = 40_012,
        /// `40018`: Only consumable SKUs can be consumed.
        #[doc(alias = "OnlyConsumableSKUsCanBeConsumed")]
        #[doc(alias = "40018")]
        OnlyConsumableSkusCanBeConsumed = 40_018,
        /// `40019`: Only test (sandbox) entitlements can be deleted.
        #[doc(alias = "40019")]
        YouCanOnlyDeleteSandboxEntitlements = 40_019,
        /// `40032`: The target user is not in a voice channel.
        #[doc(alias = "40032")]
        TargetUserIsNotConnectedToVoice = 40_032,
        /// `40033`: The message was already crossposted (published).
        #[doc(alias = "40033")]
        ThisMessageWasAlreadyCrossposted = 40_033,
        /// `40041`: An application command with this name already exists.
        #[doc(alias = "40041")]
        ApplicationCommandWithThatNameAlreadyExists = 40_041,
        /// `40043`: The application interaction could not be sent.
        #[doc(alias = "40043")]
        ApplicationInteractionFailedToSend = 40_043,
        /// `40058`: Messages cannot be sent in a forum channel (create a post instead).
        #[doc(alias = "40058")]
        CannotSendAMessageInAForumChannel = 40_058,
        /// `40060`: The interaction was already acknowledged (responded to).
        #[doc(alias = "40060")]
        InteractionHasAlreadyBeenAcknowledged = 40_060,
        /// `40061`: Forum tag names must be unique.
        #[doc(alias = "40061")]
        TagNamesMustBeUnique = 40_061,
        /// `40062`: The service resource is rate limited.
        #[doc(alias = "40062")]
        ServiceResourceIsBeingRateLimited = 40_062,
        /// `40066`: No forum tags are available that non-moderators can set.
        #[doc(alias = "40066")]
        ThereAreNoTagsAvailableThatCanBeSetByNonModerators = 40_066,
        /// `40067`: Creating a post in this forum channel requires a tag.
        #[doc(alias = "40067")]
        TagRequiredToCreateAForumPostInThisChannel = 40_067,
        /// `40074`: An entitlement was already granted for this resource.
        #[doc(alias = "40074")]
        AnEntitlementHasAlreadyBeenGrantedForThisResource = 40_074,
        /// `40094`: The interaction already has the maximum number of follow-up messages.
        #[doc(alias = "40094")]
        ThisInteractionHasHitTheMaximumNumberOfFollowUpMessages = 40_094,
        /// `40114`: The invite's target users are already being processed.
        #[doc(alias = "40114")]
        InviteTargetUsersAlreadyBeingProcessed = 40_114,
        /// `40115`: The invite's target users file was not processed yet.
        #[doc(alias = "40115")]
        InviteTargetUsersFileNotProcessed = 40_115,
        /// `40333`: Cloudflare blocked the request; a proper
        /// [User-Agent](https://docs.discord.com/developers/reference#user-agent) often fixes it.
        #[doc(alias = "40333")]
        CloudflareIsBlockingYourRequest = 40_333,
        /// `50001`: The bot has no access to the resource (e.g. it cannot see the channel).
        #[doc(alias = "50001")]
        MissingAccess = 50_001,
        /// `50002`: The account type is not valid for this action.
        #[doc(alias = "50002")]
        InvalidAccountType = 50_002,
        /// `50003`: The action is not possible in a DM channel.
        #[doc(alias = "CannotExecuteActionOnDMChannel")]
        #[doc(alias = "50003")]
        CannotExecuteActionOnDmChannel = 50_003,
        /// `50004`: The guild's widget is disabled.
        #[doc(alias = "50004")]
        GuildWidgetDisabled = 50_004,
        /// `50005`: Messages of other users cannot be edited.
        #[doc(alias = "50005")]
        CannotEditMessageAuthoredByAnotherUser = 50_005,
        /// `50006`: A message cannot be empty.
        #[doc(alias = "50006")]
        CannotSendAnEmptyMessage = 50_006,
        /// `50007`: The user does not accept messages from the bot (e.g. DMs are closed).
        #[doc(alias = "50007")]
        CannotSendMessagesToThisUser = 50_007,
        /// `50008`: Messages can only be sent in text-based channels.
        #[doc(alias = "50008")]
        CannotSendMessagesInNonTextChannel = 50_008,
        /// `50009`: The channel's verification level is too high for the user.
        #[doc(alias = "50009")]
        ChannelVerificationLevelTooHighForYouToGainAccess = 50_009,
        /// `50010`: The `OAuth2` application has no bot.
        #[doc(alias = "50010")]
        OAuth2ApplicationDoesNotHaveBot = 50_010,
        /// `50011`: The limit of `OAuth2` applications was reached.
        #[doc(alias = "50011")]
        OAuth2ApplicationLimitReached = 50_011,
        /// `50012`: The `OAuth2` state is invalid.
        #[doc(alias = "50012")]
        InvalidOAuth2State = 50_012,
        /// `50013`: The bot lacks the permissions for this action.
        #[doc(alias = "50013")]
        MissingPermissions = 50_013,
        /// `50014`: The authentication token is invalid.
        #[doc(alias = "50014")]
        InvalidToken = 50_014,
        /// `50015`: The note is too long.
        #[doc(alias = "50015")]
        NoteWasTooLong = 50_015,
        /// `50016`: Bulk deletes take between 2 and 100 messages.
        #[doc(alias = "50016")]
        ProvidedTooFewOrTooManyMessagesToDelete = 50_016,
        /// `50017`: The MFA level is invalid.
        #[doc(alias = "InvalidMFALevel")]
        #[doc(alias = "50017")]
        InvalidMfaLevel = 50_017,
        /// `50019`: A message can only be pinned in its own channel.
        #[doc(alias = "50019")]
        MessageCanOnlyBePinnedInTheChannelItWasSentIn = 50_019,
        /// `50020`: The invite code is invalid or already taken.
        #[doc(alias = "50020")]
        InviteCodeInvalidOrTaken = 50_020,
        /// `50021`: The action is not possible on a system message.
        #[doc(alias = "50021")]
        CannotExecuteActionOnSystemMessage = 50_021,
        /// `50024`: The action is not possible on this type of channel.
        #[doc(alias = "50024")]
        CannotExecuteActionOnThisChannelType = 50_024,
        /// `50025`: The `OAuth2` access token is invalid.
        #[doc(alias = "50025")]
        InvalidOAuth2AccessToken = 50_025,
        /// `50026`: A required `OAuth2` scope is missing.
        #[doc(alias = "50026")]
        MissingRequiredOAuth2Scope = 50_026,
        /// `50027`: The webhook token is invalid.
        #[doc(alias = "50027")]
        InvalidWebhookToken = 50_027,
        /// `50028`: The role is invalid.
        #[doc(alias = "50028")]
        InvalidRole = 50_028,
        /// `50033`: The recipient(s) are invalid.
        #[doc(alias = "50033")]
        InvalidRecipients = 50_033,
        /// `50034`: A message is too old to be bulk deleted (two weeks at most).
        #[doc(alias = "50034")]
        OneOfTheMessagesProvidedWasTooOldForBulkDelete = 50_034,
        /// `50035`: The form body (JSON or multipart) or the `Content-Type` is invalid; the field
        /// errors tell which fields.
        #[doc(alias = "50035")]
        InvalidFormBodyOrContentType = 50_035,
        /// `50036`: An invite was accepted to a guild the application's bot is not in.
        #[doc(alias = "50036")]
        InviteAcceptedToGuildWithoutTheBotBeingIn = 50_036,
        /// `50039`: The activity action is invalid.
        #[doc(alias = "50039")]
        InvalidActivityAction = 50_039,
        /// `50041`: The API version is invalid.
        #[doc(alias = "InvalidAPIVersion")]
        #[doc(alias = "50041")]
        InvalidApiVersion = 50_041,
        /// `50045`: The uploaded file is larger than allowed.
        #[doc(alias = "50045")]
        FileUploadedExceedsMaximumSize = 50_045,
        /// `50046`: The uploaded file is invalid.
        #[doc(alias = "50046")]
        InvalidFileUploaded = 50_046,
        /// `50054`: The gift cannot be redeemed by its own sender.
        #[doc(alias = "50054")]
        CannotSelfRedeemThisGift = 50_054,
        /// `50055`: The guild is invalid.
        #[doc(alias = "50055")]
        InvalidGuild = 50_055,
        /// `50057`: The SKU is invalid.
        #[doc(alias = "InvalidSKU")]
        #[doc(alias = "50057")]
        InvalidSku = 50_057,
        /// `50067`: The request origin is invalid.
        #[doc(alias = "50067")]
        InvalidRequestOrigin = 50_067,
        /// `50068`: The message type is invalid.
        #[doc(alias = "50068")]
        InvalidMessageType = 50_068,
        /// `50070`: Redeeming the gift requires a payment source.
        #[doc(alias = "50070")]
        PaymentSourceRequiredToRedeemGift = 50_070,
        /// `50073`: System webhooks cannot be modified.
        #[doc(alias = "50073")]
        CannotModifyASystemWebhook = 50_073,
        /// `50074`: Community guilds need this channel, so it cannot be deleted.
        #[doc(alias = "50074")]
        CannotDeleteChannelRequiredForCommunityGuilds = 50_074,
        /// `50080`: The stickers of a message cannot be edited.
        #[doc(alias = "50080")]
        CannotEditStickersWithinMessage = 50_080,
        /// `50081`: The sticker is invalid.
        #[doc(alias = "50081")]
        InvalidStickerSent = 50_081,
        /// `50083`: The thread is archived, so the operation (e.g. editing a message or adding a
        /// member) is not possible.
        #[doc(alias = "50083")]
        InvalidActionOnArchivedThread = 50_083,
        /// `50084`: The thread notification settings are invalid.
        #[doc(alias = "50084")]
        InvalidThreadNotificationSettings = 50_084,
        /// `50085`: The `before` value is earlier than the thread's creation.
        #[doc(alias = "50085")]
        ParameterEarlierThanCreation = 50_085,
        /// `50086`: Channels of community servers must be text channels.
        #[doc(alias = "50086")]
        CommunityServerChannelsMustBeTextChannels = 50_086,
        /// `50091`: The event's entity type differs from the entity it is being started for.
        #[doc(alias = "50091")]
        TheEntityTypeOfTheEventIsDifferentFromTheEntityYouAreTryingToStartTheEventFor = 50_091,
        /// `50095`: The server is not available in the user's location.
        #[doc(alias = "50095")]
        ServerNotAvailableInYourLocation = 50_095,
        /// `50097`: The server needs monetization enabled for this action.
        #[doc(alias = "50097")]
        ServerNeedsMonetizationEnabledToPerformThisAction = 50_097,
        /// `50101`: The server needs more boosts for this action.
        #[doc(alias = "50101")]
        ServerNeedsMoreBoostsToPerformThisAction = 50_101,
        /// `50109`: The request body is not valid JSON.
        #[doc(alias = "RequestBodyContainsInvalidJSON")]
        #[doc(alias = "50109")]
        RequestBodyContainsInvalidJson = 50_109,
        /// `50110`: The file is invalid.
        #[doc(alias = "50110")]
        ProvidedFileIsInvalid = 50_110,
        /// `50123`: The file type is invalid.
        #[doc(alias = "50123")]
        ProvidedFileTypeIsInvalid = 50_123,
        /// `50124`: The file is longer than the maximum duration (5.2 seconds).
        #[doc(alias = "50124")]
        ProvidedFileDurationExceedsMaximumLength = 50_124,
        /// `50131`: The owner cannot be a pending member.
        #[doc(alias = "50131")]
        OwnerCannotBePendingMember = 50_131,
        /// `50132`: A bot user cannot become the owner.
        #[doc(alias = "50132")]
        OwnershipCannotBeMovedToABotUser = 50_132,
        /// `50138`: The asset could not be resized below the maximum size (262 144).
        #[doc(alias = "50138")]
        FailedToResizeAssetBelowTheMaximumSize = 50_138,
        /// `50144`: An emoji cannot mix subscription and non-subscription roles.
        #[doc(alias = "50144")]
        CannotMixSubscriptionAndNonSubscriptionRolesForAnEmoji = 50_144,
        /// `50145`: An emoji cannot be converted between premium and normal.
        #[doc(alias = "50145")]
        CannotConvertBetweenPremiumEmojiAndNormalEmoji = 50_145,
        /// `50146`: The uploaded file was not found.
        #[doc(alias = "50146")]
        UploadedFileNotFound = 50_146,
        /// `50151`: The emoji is invalid.
        #[doc(alias = "50151")]
        SpecifiedEmojiIsInvalid = 50_151,
        /// `50159`: A voice message cannot have other content.
        #[doc(alias = "50159")]
        VoiceMessagesDoNotSupportAdditionalContent = 50_159,
        /// `50160`: A voice message needs exactly one audio attachment.
        #[doc(alias = "50160")]
        VoiceMessagesMustHaveASingleAudioAttachment = 50_160,
        /// `50161`: A voice message needs its metadata (duration and waveform).
        #[doc(alias = "50161")]
        VoiceMessagesMustHaveSupportingMetadata = 50_161,
        /// `50162`: Voice messages cannot be edited.
        #[doc(alias = "50162")]
        VoiceMessagesCannotBeEdited = 50_162,
        /// `50163`: The guild subscription integration cannot be deleted.
        #[doc(alias = "50163")]
        CannotDeleteGuildSubscriptionIntegration = 50_163,
        /// `50167`: No voice effect can be sent while the user is server muted, deafened or
        /// suppressed.
        #[doc(alias = "50167")]
        CannotSendVoiceEffectWhenUserIsServerMutedDeafenedOrSuppressed = 50_167,
        /// `50173`: Voice messages cannot be sent in this channel.
        #[doc(alias = "50173")]
        YouCannotSendVoiceMessagesInThisChannel = 50_173,
        /// `50178`: The user account must be verified first.
        #[doc(alias = "50178")]
        TheUserAccountMustFirstBeVerified = 50_178,
        /// `50192`: The file has no valid duration.
        #[doc(alias = "50192")]
        ProvidedFileDoesNotHaveAValidDuration = 50_192,
        /// `50278`: The user shares no guild with the bot, so it cannot message them.
        #[doc(alias = "50278")]
        CannotSendMessagesToThisUserDueToHavingNoMutualGuilds = 50_278,
        /// `50600`: The bot may not send this sticker.
        #[doc(alias = "50600")]
        YouDoNotHavePermissionToSendThisSticker = 50_600,
        /// `60003`: The operation requires two-factor authentication.
        #[doc(alias = "60003")]
        TwoFactorAuthenticationIsRequired = 60_003,
        /// `80004`: No user with this Discord tag exists.
        #[doc(alias = "80004")]
        NoUsersWithDiscordTagExist = 80_004,
        /// `90001`: The reaction was blocked.
        #[doc(alias = "90001")]
        ReactionWasBlocked = 90_001,
        /// `90002`: The user cannot use super (burst) reactions.
        #[doc(alias = "90002")]
        UserCannotUseBurstReactions = 90_002,
        /// `110000`: The index is not available yet; try again later.
        #[doc(alias = "110000")]
        IndexNotYetAvailable = 110_000,
        /// `110001`: The application is not available yet; try again later.
        #[doc(alias = "110001")]
        ApplicationNotYetAvailable = 110_001,
        /// `130000`: The API resource is overloaded; try again a little later.
        #[doc(alias = "APIResourceOverloaded")]
        #[doc(alias = "130000")]
        ApiResourceOverloaded = 130_000,
        /// `150006`: The Stage is already open.
        #[doc(alias = "150006")]
        TheStageIsAlreadyOpen = 150_006,
        /// `160002`: Replying requires the permission to read the message history.
        #[doc(alias = "160002")]
        CannotReplyWithoutPermissionToReadMessageHistory = 160_002,
        /// `160004`: This message already has a thread.
        #[doc(alias = "160004")]
        ThreadAlreadyCreatedForMessage = 160_004,
        /// `160005`: The thread is locked.
        #[doc(alias = "160005")]
        ThreadLocked = 160_005,
        /// `160006`: The limit of active threads was reached.
        #[doc(alias = "160006")]
        MaximumActiveThreads = 160_006,
        /// `160007`: The limit of active announcement threads was reached.
        #[doc(alias = "160007")]
        MaximumActiveAnnouncementThreads = 160_007,
        /// `160009`: Referencing a message requires the permission to read the message history.
        #[doc(alias = "160009")]
        CannotReferenceAMessageWithoutPermissionToReadMessageHistory = 160_009,
        /// `160010`: Messages of NSFW channels cannot be referenced here.
        #[doc(alias = "NSFWChannelMessageReferenceNotAllowed")]
        #[doc(alias = "160010")]
        NsfwChannelMessageReferenceNotAllowed = 160_010,
        /// `160014`: A message whose content the user cannot read cannot be forwarded.
        #[doc(alias = "160014")]
        CannotForwardMessageWithUnreadableContent = 160_014,
        /// `170001`: The uploaded Lottie file is not valid JSON.
        #[doc(alias = "InvalidJSONForUploadedLottieFile")]
        #[doc(alias = "170001")]
        InvalidJsonForUploadedLottieFile = 170_001,
        /// `170002`: An uploaded Lottie cannot contain raster images (e.g. PNG or JPEG).
        #[doc(alias = "170002")]
        UploadedLottiesCannotContainRasterizedImages = 170_002,
        /// `170003`: The sticker's frame rate is above the maximum.
        #[doc(alias = "170003")]
        StickerMaximumFramerateExceeded = 170_003,
        /// `170004`: The sticker has more than 1000 frames.
        #[doc(alias = "170004")]
        StickerFrameCountExceedsMaximumOf1000Frames = 170_004,
        /// `170005`: The Lottie animation is larger than the maximum dimensions.
        #[doc(alias = "170005")]
        LottieAnimationMaximumDimensionsExceeded = 170_005,
        /// `170006`: The sticker's frame rate is too low or too high.
        #[doc(alias = "170006")]
        StickerFramerateIsTooSmallOrTooLarge = 170_006,
        /// `170007`: The sticker's animation is longer than 5 seconds.
        #[doc(alias = "170007")]
        StickerAnimationDurationExceedsMaximumOf5Seconds = 170_007,
        /// `180000`: A finished event cannot be updated.
        #[doc(alias = "180000")]
        CannotUpdateAFinishedEvent = 180_000,
        /// `180002`: The Stage for the Stage event could not be created.
        #[doc(alias = "180002")]
        FailedToCreateStageNeededForStageEvent = 180_002,
        /// `200000`: `AutoMod` blocked the message.
        #[doc(alias = "200000")]
        MessageWasBlockedByAutomaticModeration = 200_000,
        /// `200001`: `AutoMod` blocked the title.
        #[doc(alias = "200001")]
        TitleWasBlockedByAutomaticModeration = 200_001,
        /// `220001`: A webhook message in a forum channel needs a `thread_name` or a `thread_id`.
        #[doc(alias = "220001")]
        WebhooksPostedToForumChannelsMustHaveAThreadNameOrThreadId = 220_001,
        /// `220002`: A webhook message in a forum channel cannot have both a `thread_name` and a
        /// `thread_id`.
        #[doc(alias = "220002")]
        WebhooksPostedToForumChannelsCannotHaveBothAThreadNameAndThreadId = 220_002,
        /// `220003`: Webhooks can only create threads in forum channels.
        #[doc(alias = "220003")]
        WebhooksCanOnlyCreateThreadsInForumChannels = 220_003,
        /// `220004`: Webhook services cannot be used in forum channels.
        #[doc(alias = "220004")]
        WebhookServicesCannotBeUsedInForumChannels = 220_004,
        /// `240000`: The harmful links filter blocked the message.
        #[doc(alias = "240000")]
        MessageBlockedByHarmfulLinksFilter = 240_000,
        /// `340015`: The user is limited from joining new servers.
        #[doc(alias = "340015")]
        AccessToJoiningNewServersHasBeenLimitedForTheUser = 340_015,
        /// `350000`: Onboarding cannot be enabled: its requirements are not met.
        #[doc(alias = "350000")]
        CannotEnableOnboardingRequirementsAreNotMet = 350_000,
        /// `350001`: Onboarding cannot be updated while it is below the requirements.
        #[doc(alias = "350001")]
        CannotUpdateOnboardingWhileBelowRequirements = 350_001,
        /// `400001`: File uploads are limited for this guild.
        #[doc(alias = "400001")]
        AccessToFileUploadsHasBeenLimitedForThisGuild = 400_001,
        /// `500000`: The users could not be banned.
        #[doc(alias = "500000")]
        FailedToBanUsers = 500_000,
        /// `520000`: Voting in the poll is blocked.
        #[doc(alias = "520000")]
        PollVotingBlocked = 520_000,
        /// `520001`: The poll has expired.
        #[doc(alias = "520001")]
        PollExpired = 520_001,
        /// `520002`: Polls cannot be created in this type of channel.
        #[doc(alias = "520002")]
        InvalidChannelTypeForPollCreation = 520_002,
        /// `520003`: A poll message cannot be edited.
        #[doc(alias = "520003")]
        CannotEditAPollMessage = 520_003,
        /// `520004`: An emoji that is part of the poll cannot be used.
        #[doc(alias = "520004")]
        CannotUseAnEmojiIncludedWithThePoll = 520_004,
        /// `520006`: Only poll messages can be expired.
        #[doc(alias = "520006")]
        CannotExpireANonPollMessage = 520_006,
        /// `530000`: The application may not use provisional accounts.
        #[doc(alias = "530000")]
        ProvisionalAccountsPermissionNotGranted = 530_000,
        /// `530001`: The ID token (JWT) has expired; get a new one from the identity provider.
        #[doc(alias = "IdTokenJWTExpired")]
        #[doc(alias = "530001")]
        IdTokenJwtExpired = 530_001,
        /// `530002`: The ID token's issuer does not match the configured one.
        #[doc(alias = "IdTokenJWTIssuerMismatch")]
        #[doc(alias = "530002")]
        IdTokenJwtIssuerMismatch = 530_002,
        /// `530003`: The ID token's audience does not match the OIDC configuration; fix the
        /// configuration or use a token issued to the application.
        #[doc(alias = "IdTokenJWTAudienceMismatch")]
        #[doc(alias = "530003")]
        IdTokenJwtAudienceMismatch = 530_003,
        /// `530004`: The ID token was issued more than a week ago; get a new one from the identity
        /// provider.
        #[doc(alias = "IdTokenJWTIssuedTooLongAgo")]
        #[doc(alias = "530004")]
        IdTokenJwtIssuedTooLongAgo = 530_004,
        /// `530006`: No unique username could be generated in time; retrying usually works.
        #[doc(alias = "530006")]
        FailedToGenerateUniqueUsername = 530_006,
        /// `530007`: The client secret is invalid; check it or create a new one.
        #[doc(alias = "530007")]
        InvalidClientSecret = 530_007,
    }
}
