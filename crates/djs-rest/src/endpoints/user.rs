//! [User endpoints](https://docs.discord.com/developers/resources/user): the current user,
//! other users, the current user's guilds, DMs, connections and role connections.
//!
//! Several of them act for a user through `OAuth2` and need a client built with
//! [`RestBuilder::bearer_token`](crate::RestBuilder::bearer_token) (D-108); their docs say so.

use djs_model::{
    channel::Channel,
    guild::Member,
    id::{ApplicationId, GuildId, UserId},
    rest::{CreateDm, CreateGroupDm, CurrentUserGuild, GetCurrentUserGuilds, ModifyCurrentUser},
    user::{ApplicationRoleConnection, Connection, CurrentUser, User},
};

use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [Get Current User](https://docs.discord.com/developers/resources/user#get-current-user):
    /// returns the bot's own user (or, with a bearer token, the authorizing user's; the
    /// email needs the `email` scope).
    ///
    /// discord.js counterpart: `Client#user` (from the `READY` event).
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `401 Unauthorized` for an invalid token.
    pub async fn get_current_user(&self) -> Result<CurrentUser, Error> {
        self.request(Request::new(Route::GetCurrentUser)).await
    }

    /// [Get User](https://docs.discord.com/developers/resources/user#get-user): returns any
    /// user by ID.
    ///
    /// discord.js counterpart: `UserManager#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10013` Unknown User.
    pub async fn get_user(&self, user_id: UserId) -> Result<User, Error> {
        self.request(Request::new(Route::GetUser { user_id })).await
    }

    /// [Modify Current User](https://docs.discord.com/developers/resources/user#modify-current-user):
    /// changes the bot's username, avatar or banner.
    ///
    /// discord.js counterpart: `ClientUser#edit`, `#setUsername`, `#setAvatar`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest) -> Result<(), Box<dyn std::error::Error>> {
    /// use djs_model::{rest::ModifyCurrentUser, util::{ImageData, Patch}};
    ///
    /// let mut body = ModifyCurrentUser::default();
    /// body.avatar = Patch::Set(ImageData::new(std::fs::read("avatar.png")?)?);
    /// rest.modify_current_user(&body).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50035` when the username is taken or changed too often.
    pub async fn modify_current_user(
        &self,
        body: &ModifyCurrentUser,
    ) -> Result<CurrentUser, Error> {
        self.request(Request::new(Route::ModifyCurrentUser).json(body)?)
            .await
    }

    /// [Get Current User Guilds](https://docs.discord.com/developers/resources/user#get-current-user-guilds):
    /// returns a page of the current user's guilds (200 by default), sorted by ID. With a
    /// bearer token, needs the `guilds` scope.
    ///
    /// Bots with large bot sharding must set [`shard`](GetCurrentUserGuilds::shard).
    ///
    /// discord.js counterpart: `GuildManager#fetch()`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn get_current_user_guilds(
        &self,
        query: &GetCurrentUserGuilds,
    ) -> Result<Vec<CurrentUserGuild>, Error> {
        self.request(Request::new(Route::GetCurrentUserGuilds).query(query))
            .await
    }

    /// [Get Current User Guild Member](https://docs.discord.com/developers/resources/user#get-current-user-guild-member):
    /// returns the authorizing user's membership of a guild. Needs a
    /// [bearer token](crate::RestBuilder::bearer_token) with the `guilds.members.read` scope.
    ///
    /// discord.js counterpart: none (`REST#get(Routes.userGuildMember(guildId))`).
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10004` Unknown Guild when the user is not in it.
    pub async fn get_current_user_guild_member(&self, guild_id: GuildId) -> Result<Member, Error> {
        self.request(Request::new(Route::GetCurrentUserGuildMember { guild_id }))
            .await
    }

    /// [Leave Guild](https://docs.discord.com/developers/resources/user#leave-guild): makes
    /// the bot leave a guild.
    ///
    /// discord.js counterpart: `Guild#leave`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10004` Unknown Guild.
    pub async fn leave_guild(&self, guild_id: GuildId) -> Result<(), Error> {
        self.request(Request::new(Route::LeaveGuild { guild_id }))
            .await
    }

    /// [Create DM](https://docs.discord.com/developers/resources/user#create-dm): opens (or
    /// returns the existing) DM channel with a user. Send messages to it with
    /// [`create_message`](Self::create_message).
    ///
    /// DMs should follow a user action; opening many quickly gets the bot rate limited or
    /// blocked from opening more.
    ///
    /// discord.js counterpart: `User#createDM`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, user_id: djs_model::id::UserId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::{CreateDm, CreateMessage};
    ///
    /// let dm = rest.create_dm(&CreateDm::new(user_id)).await?;
    /// rest.create_message(dm.id(), &CreateMessage::from("Your report was received."))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `50007` when the user does not accept DMs from the bot (on
    /// the message, not here).
    pub async fn create_dm(&self, body: &CreateDm) -> Result<Channel, Error> {
        self.request(Request::new(Route::CreateDm).json(body)?)
            .await
    }

    /// [Create Group DM](https://docs.discord.com/developers/resources/user#create-group-dm):
    /// creates a group DM with users who granted the application `gdm.join` (max 10 active
    /// group DMs; meant for the deprecated `GameBridge` SDK).
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn create_group_dm(&self, body: &CreateGroupDm) -> Result<Channel, Error> {
        self.request(Request::new(Route::CreateGroupDm).json(body)?)
            .await
    }

    /// [Get Current User Connections](https://docs.discord.com/developers/resources/user#get-current-user-connections):
    /// returns the accounts the authorizing user connected (Twitch, GitHub, …). Needs a
    /// [bearer token](crate::RestBuilder::bearer_token) with the `connections` scope.
    ///
    /// discord.js counterpart: none (`REST#get(Routes.userConnections())`).
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn get_current_user_connections(&self) -> Result<Vec<Connection>, Error> {
        self.request(Request::new(Route::GetCurrentUserConnections))
            .await
    }

    /// [Get Current User Application Role Connection](https://docs.discord.com/developers/resources/user#get-current-user-application-role-connection):
    /// returns the role connection (linked-roles data) the application stored for the
    /// authorizing user. Needs a [bearer token](crate::RestBuilder::bearer_token) with the
    /// `role_connections.write` scope.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn get_current_user_application_role_connection(
        &self,
        application_id: ApplicationId,
    ) -> Result<ApplicationRoleConnection, Error> {
        self.request(Request::new(
            Route::GetCurrentUserApplicationRoleConnection { application_id },
        ))
        .await
    }

    /// [Update Current User Application Role Connection](https://docs.discord.com/developers/resources/user#update-current-user-application-role-connection):
    /// replaces the role connection of the authorizing user and returns it. Needs a
    /// [bearer token](crate::RestBuilder::bearer_token) with the `role_connections.write`
    /// scope.
    ///
    /// The metadata values are stringified (`"1"` for true, ISO 8601 for dates), keyed by
    /// the application's role connection metadata keys.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(user: djs_rest::Rest, application_id: djs_model::id::ApplicationId) -> Result<(), djs_rest::Error> {
    /// use djs_model::user::ApplicationRoleConnection;
    ///
    /// let mut connection = ApplicationRoleConnection::default();
    /// connection.platform_name = Some("Chess Club".into());
    /// connection.metadata.insert("rating".into(), "1850".into());
    /// user.update_current_user_application_role_connection(application_id, &connection)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn update_current_user_application_role_connection(
        &self,
        application_id: ApplicationId,
        body: &ApplicationRoleConnection,
    ) -> Result<ApplicationRoleConnection, Error> {
        self.request(
            Request::new(Route::UpdateCurrentUserApplicationRoleConnection { application_id })
                .json(body)?,
        )
        .await
    }

    /// [Delete Current User Application Role Connection](https://docs.discord.com/developers/resources/user#delete-current-user-application-role-connection):
    /// removes the role connection of the authorizing user. Needs a
    /// [bearer token](crate::RestBuilder::bearer_token) with the `role_connections.write`
    /// scope.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn delete_current_user_application_role_connection(
        &self,
        application_id: ApplicationId,
    ) -> Result<(), Error> {
        self.request_ignoring_body(Request::new(
            Route::DeleteCurrentUserApplicationRoleConnection { application_id },
        ))
        .await
    }
}
