//! [Invite endpoints](https://docs.discord.com/developers/resources/invite): resolving and
//! deleting invites, and the target users of invites only some users may accept.
//!
//! Invites are created per channel with
//! [`create_channel_invite`](Rest::create_channel_invite) and listed with
//! [`get_channel_invites`](Rest::get_channel_invites) and
//! [`get_guild_invites`](Rest::get_guild_invites).

use djs_model::{
    id::UserId,
    invite::{Invite, InviteTargetUsersJobStatus},
    rest::{BulkTargetUsers, FileUpload, GetInvite},
};

use crate::client::boxed;
use crate::{Error, ErrorKind, Request, Rest, Route};

/// Invites.
impl Rest {
    /// [Get Invite](https://docs.discord.com/developers/resources/invite#get-invite):
    /// resolves an invite code (the part after `discord.gg/`).
    ///
    /// discord.js counterpart: `Client#fetchInvite`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::GetInvite;
    ///
    /// let mut query = GetInvite::default();
    /// query.with_counts = Some(true);
    /// let invite = rest.get_invite("rust-lang", &query).await?;
    /// println!("{:?} members", invite.approximate_member_count);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10006` Unknown Invite.
    pub fn get_invite<'a>(
        &'a self,
        invite_code: &'a str,
        query: &'a GetInvite,
    ) -> impl Future<Output = Result<Invite, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::GetInvite { invite_code }).query(query))
                .await
        })
    }

    /// [Delete Invite](https://docs.discord.com/developers/resources/invite#delete-invite):
    /// revokes an invite and returns it. Needs `MANAGE_CHANNELS` on its channel or
    /// `MANAGE_GUILD`.
    ///
    /// `reason` is shown in the audit log (D-086).
    ///
    /// discord.js counterpart: `Invite#delete`, `GuildInviteManager#delete`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10006` Unknown Invite.
    pub fn delete_invite<'a>(
        &'a self,
        invite_code: &'a str,
        reason: Option<&'a str>,
    ) -> impl Future<Output = Result<Invite, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::DeleteInvite { invite_code }).reason(reason))
                .await
        })
    }
}

/// Target users: the users allowed to see and accept an invite. Each endpoint needs the bot to
/// be the inviter or to have `MANAGE_GUILD` (reading also works with `VIEW_AUDIT_LOG`).
impl Rest {
    /// [Get Target Users](https://docs.discord.com/developers/resources/invite#get-target-users):
    /// returns the users allowed to accept the invite. Discord sends a CSV file, which is
    /// parsed (D-126).
    ///
    /// discord.js counterpart: `GuildInviteManager#fetchTargetUsers` (the raw file).
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; [`ErrorKind::Deserialize`] when a line of the file is not a user
    /// ID; `10129` when the invite has no target users.
    pub fn get_target_users<'a>(
        &'a self,
        invite_code: &'a str,
    ) -> impl Future<Output = Result<Vec<UserId>, Error>> + Send + 'a {
        boxed(async move {
            let (_, body) = self
                .execute(Request::new(Route::GetTargetUsers { invite_code }))
                .await?;
            parse_target_users(&body)
        })
    }

    /// [Add Target User](https://docs.discord.com/developers/resources/invite#add-target-user):
    /// allows one more user to accept the invite.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30084` at the limit of target users.
    pub fn add_target_user<'a>(
        &'a self,
        invite_code: &'a str,
        user_id: UserId,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::AddTargetUser {
                invite_code,
                user_id,
            }))
            .await
        })
    }

    /// [Remove Target User](https://docs.discord.com/developers/resources/invite#remove-target-user):
    /// no longer allows a user to accept the invite.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn remove_target_user<'a>(
        &'a self,
        invite_code: &'a str,
        user_id: UserId,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::RemoveTargetUser {
                invite_code,
                user_id,
            }))
            .await
        })
    }

    /// [Update Target Users](https://docs.discord.com/developers/resources/invite#update-target-users):
    /// replaces the users allowed to accept the invite with those of a CSV file — build it
    /// with [`FileUpload::target_users`] (D-127). Discord processes the file in the
    /// background; follow it with [`get_target_users_job_status`](Self::get_target_users_job_status).
    ///
    /// discord.js counterpart: `GuildInviteManager#updateTargetUsers`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, invite_code: &str, members: Vec<djs_model::id::UserId>) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::FileUpload;
    ///
    /// rest.update_target_users(invite_code, &FileUpload::target_users(members))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `400` listing the invalid lines of the file; `40114` while a
    /// previous file is still being processed.
    pub fn update_target_users<'a>(
        &'a self,
        invite_code: &'a str,
        file: &'a FileUpload,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.request_ignoring_body(
                Request::new(Route::UpdateTargetUsers { invite_code })
                    .form_file("target_users_file", file),
            )
            .await
        })
    }

    /// [Bulk-Add Target Users](https://docs.discord.com/developers/resources/invite#bulk-add-target-users):
    /// allows up to 1000 more users to accept the invite.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `30084` at the limit of target users.
    pub fn bulk_add_target_users<'a>(
        &'a self,
        invite_code: &'a str,
        body: &'a BulkTargetUsers,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::BulkAddTargetUsers { invite_code }).json(body)?)
                .await
        })
    }

    /// [Bulk-Delete Target Users](https://docs.discord.com/developers/resources/invite#bulk-delete-target-users):
    /// no longer allows up to 1000 users to accept the invite.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn bulk_delete_target_users<'a>(
        &'a self,
        invite_code: &'a str,
        body: &'a BulkTargetUsers,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::BulkDeleteTargetUsers { invite_code }).json(body)?)
                .await
        })
    }

    /// [Get Target Users Job Status](https://docs.discord.com/developers/resources/invite#get-target-users-job-status):
    /// returns the progress of processing the invite's target users file.
    ///
    /// discord.js counterpart: `GuildInviteManager#fetchTargetUsersJobStatus`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10124` when there is no job for the invite.
    pub fn get_target_users_job_status<'a>(
        &'a self,
        invite_code: &'a str,
    ) -> impl Future<Output = Result<InviteTargetUsersJobStatus, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::GetTargetUsersJobStatus { invite_code }))
                .await
        })
    }
}

/// Parses the target users CSV: an optional `user_id` header, then one user ID per line.
/// Blank lines and surrounding whitespace (including `\r`) are ignored.
fn parse_target_users(body: &[u8]) -> Result<Vec<UserId>, Error> {
    let text = core::str::from_utf8(body)
        .map_err(|source| Error::with_source(ErrorKind::Deserialize, source))?;
    let mut lines = text.lines().map(str::trim).enumerate().peekable();
    // Skip the header that Discord's files start with.
    lines.next_if(|(_, line)| *line == "user_id");
    lines
        .filter(|(_, line)| !line.is_empty())
        .map(|(index, line)| {
            line.parse().map_err(|source| {
                Error::with_source(
                    ErrorKind::Deserialize,
                    format!(
                        "line {} of the target users file is not a user ID: {source}",
                        index + 1
                    ),
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_users_csv() {
        let ids = |body: &str| parse_target_users(body.as_bytes()).unwrap();
        assert_eq!(
            ids("user_id\n7\n8\n"),
            [UserId::new(7), UserId::new(8)],
            "header and trailing newline"
        );
        assert_eq!(ids("7\r\n\r\n8"), [UserId::new(7), UserId::new(8)]);
        assert_eq!(ids("user_id\n"), Vec::<UserId>::new());
        assert_eq!(ids(""), Vec::<UserId>::new());

        let error = parse_target_users(b"user_id\n7\nferris\n").unwrap_err();
        assert!(matches!(error.kind(), ErrorKind::Deserialize));
        assert!(
            core::error::Error::source(&error)
                .unwrap()
                .to_string()
                .starts_with("line 3 ")
        );
    }
}
