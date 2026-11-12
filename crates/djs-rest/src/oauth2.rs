//! [OAuth2](https://docs.discord.com/developers/topics/oauth2) helpers: the client credentials
//! of an application (D-132) and authorization URLs (D-133).
//!
//! The token endpoints themselves are methods of [`Rest`](crate::Rest):
//! [`exchange_code`](crate::Rest::exchange_code),
//! [`refresh_access_token`](crate::Rest::refresh_access_token),
//! [`client_credentials_grant`](crate::Rest::client_credentials_grant) and
//! [`revoke_token`](crate::Rest::revoke_token).
//!
//! # Examples
//!
//! The authorization code grant, from the URL a user opens to the user's own client:
//!
//! ```no_run
//! # async fn example(rest: djs_rest::Rest, code: &str) -> Result<(), djs_rest::Error> {
//! use djs_model::{id::ApplicationId, oauth2::Scope};
//! use djs_rest::{
//!     Rest,
//!     oauth2::{AuthorizeUrl, OAuth2Credentials},
//! };
//!
//! let client_id = ApplicationId::new(157_730_590_492_196_864);
//! let redirect_uri = "https://example.com/callback";
//! // 1. Send the user to the authorization page.
//! let url = AuthorizeUrl::code(client_id, redirect_uri)
//!     .scopes([Scope::Identify, Scope::Guilds])
//!     .state("a-random-value-tied-to-the-session")
//!     .build();
//! // 2. Discord redirects back with `?code=…&state=…`; exchange the code.
//! let credentials = OAuth2Credentials::new(client_id, std::env::var("CLIENT_SECRET").unwrap());
//! let token = rest.exchange_code(&credentials, code, redirect_uri).await?;
//! // 3. Act for the user.
//! let user_rest = Rest::builder().bearer_token(token.access_token).build();
//! let user = user_rest.get_current_user().await?;
//! # Ok(())
//! # }
//! ```

use core::fmt;

use djs_model::{
    application::IntegrationType,
    id::{ApplicationId, GuildId},
    oauth2::Scope,
    permissions::Permissions,
};

use crate::route::write_text;

/// The base [authorization URL](https://docs.discord.com/developers/topics/oauth2#shared-resources-oauth2-urls).
pub const AUTHORIZE_URL: &str = "https://discord.com/oauth2/authorize";

/// An application's client ID and client secret, which authenticate the token endpoints
/// (D-132).
///
/// The secret is never shown in the `Debug` output. Keep it out of source code; load it from
/// the environment or a secret store.
///
/// # Examples
///
/// ```
/// use djs_model::id::ApplicationId;
/// use djs_rest::oauth2::OAuth2Credentials;
///
/// let credentials = OAuth2Credentials::new(ApplicationId::new(1), "client-secret");
/// assert!(!format!("{credentials:?}").contains("client-secret"));
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct OAuth2Credentials {
    client_id: ApplicationId,
    client_secret: String,
}

impl OAuth2Credentials {
    /// The credentials of the application `client_id` (its application ID).
    #[must_use]
    pub fn new(client_id: ApplicationId, client_secret: impl Into<String>) -> Self {
        Self {
            client_id,
            client_secret: client_secret.into(),
        }
    }

    /// The application's client ID.
    #[must_use]
    pub const fn client_id(&self) -> ApplicationId {
        self.client_id
    }
}

impl fmt::Debug for OAuth2Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuth2Credentials")
            .field("client_id", &self.client_id)
            .field("client_secret", &"<redacted>")
            .finish()
    }
}

/// What kind of token [`revoke_token`](crate::Rest::revoke_token) revokes. Discord revokes
/// every token of the authorization either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TokenTypeHint {
    /// An access token.
    AccessToken,
    /// A refresh token.
    RefreshToken,
}

impl TokenTypeHint {
    /// The hint as Discord expects it (`access_token` or `refresh_token`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AccessToken => "access_token",
            Self::RefreshToken => "refresh_token",
        }
    }
}

/// How the authorization page treats a user who already authorized the same scopes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Prompt {
    /// Ask the user to approve again.
    Consent,
    /// Skip the page and redirect at once (not for `bot` and `webhook.incoming`, which always
    /// ask).
    None,
}

impl Prompt {
    /// The value of the `prompt` parameter.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Consent => "consent",
            Self::None => "none",
        }
    }
}

/// Builds an [authorization URL](https://docs.discord.com/developers/topics/oauth2#authorization-code-grant)
/// (`https://discord.com/oauth2/authorize?…`) that adds a bot to a guild or asks a user to
/// authorize the application (D-133).
///
/// discord.js counterpart: `Client#generateInvite` (bot invites only).
///
/// # Examples
///
/// ```
/// use djs_model::{
///     id::{ApplicationId, GuildId},
///     oauth2::Scope,
///     permissions::Permissions,
/// };
/// use djs_rest::oauth2::AuthorizeUrl;
///
/// let invite = AuthorizeUrl::bot(ApplicationId::new(157))
///     .scopes([Scope::ApplicationsCommands])
///     .permissions(Permissions::SEND_MESSAGES | Permissions::EMBED_LINKS)
///     .guild_id(GuildId::new(42))
///     .disable_guild_select(true);
/// assert_eq!(
///     invite.build(),
///     "https://discord.com/oauth2/authorize?client_id=157&scope=bot%20applications.commands\
///      &permissions=18432&guild_id=42&disable_guild_select=true"
/// );
///
/// let login = AuthorizeUrl::code(ApplicationId::new(157), "https://example.com/callback")
///     .scopes([Scope::Identify])
///     .state("xyz");
/// assert_eq!(
///     login.to_string(),
///     "https://discord.com/oauth2/authorize?response_type=code&client_id=157&scope=identify\
///      &state=xyz&redirect_uri=https%3A%2F%2Fexample.com%2Fcallback"
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "an authorization URL builder does nothing until it is built"]
pub struct AuthorizeUrl {
    client_id: ApplicationId,
    code: bool,
    scopes: Vec<Scope>,
    state: Option<String>,
    redirect_uri: Option<String>,
    prompt: Option<Prompt>,
    permissions: Option<Permissions>,
    guild_id: Option<GuildId>,
    disable_guild_select: bool,
    integration_type: Option<IntegrationType>,
}

impl AuthorizeUrl {
    /// A URL with only the client ID: Discord uses the application's default install settings
    /// from the Developer Portal (unless scopes, an integration type or a redirect URI are
    /// added).
    pub const fn new(client_id: ApplicationId) -> Self {
        Self {
            client_id,
            code: false,
            scopes: Vec::new(),
            state: None,
            redirect_uri: None,
            prompt: None,
            permissions: None,
            guild_id: None,
            disable_guild_select: false,
            integration_type: None,
        }
    }

    /// A [bot authorization](https://docs.discord.com/developers/topics/oauth2#bot-authorization-flow)
    /// URL: the `bot` scope, which adds the bot to a guild the user picks. No redirect is
    /// needed.
    pub fn bot(client_id: ApplicationId) -> Self {
        Self::new(client_id).scopes([Scope::Bot])
    }

    /// An [authorization code grant](https://docs.discord.com/developers/topics/oauth2#authorization-code-grant)
    /// URL: Discord redirects the user to `redirect_uri` (one registered for the application)
    /// with a `code` to exchange with [`exchange_code`](crate::Rest::exchange_code).
    pub fn code(client_id: ApplicationId, redirect_uri: impl Into<String>) -> Self {
        let mut url = Self::new(client_id);
        url.code = true;
        url.redirect_uri(redirect_uri)
    }

    /// Adds scopes to request.
    pub fn scopes(mut self, scopes: impl IntoIterator<Item = Scope>) -> Self {
        self.scopes.extend(scopes);
        self
    }

    /// Sets the [`state`](https://docs.discord.com/developers/topics/oauth2#state-and-security)
    /// that Discord sends back with the redirect: a value tied to the user's session, which
    /// protects against cross-site request forgery.
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Sets where Discord redirects the user after the authorization.
    pub fn redirect_uri(mut self, redirect_uri: impl Into<String>) -> Self {
        self.redirect_uri = Some(redirect_uri.into());
        self
    }

    /// Sets how a user who already authorized the scopes is treated.
    pub const fn prompt(mut self, prompt: Prompt) -> Self {
        self.prompt = Some(prompt);
        self
    }

    /// Sets the permissions requested for the bot (with the `bot` scope).
    pub const fn permissions(mut self, permissions: Permissions) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Preselects a guild in the guild picker (with the `bot` scope).
    pub const fn guild_id(mut self, guild_id: GuildId) -> Self {
        self.guild_id = Some(guild_id);
        self
    }

    /// Whether the user may not pick another guild than [`guild_id`](Self::guild_id).
    pub const fn disable_guild_select(mut self, disable: bool) -> Self {
        self.disable_guild_select = disable;
        self
    }

    /// Sets where the application is installed: in a guild or for the user (only relevant
    /// with the `applications.commands` scope).
    pub const fn integration_type(mut self, integration_type: IntegrationType) -> Self {
        self.integration_type = Some(integration_type);
        self
    }

    /// Returns the URL.
    #[must_use]
    pub fn build(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for AuthorizeUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut url = String::with_capacity(128);
        url.push_str(AUTHORIZE_URL);
        url.push('?');
        if self.code {
            url.push_str("response_type=code&");
        }
        url.push_str("client_id=");
        // Writing to a `String` cannot fail.
        let _ = fmt::Write::write_fmt(&mut url, format_args!("{}", self.client_id));
        if !self.scopes.is_empty() {
            url.push_str("&scope=");
            for (index, scope) in self.scopes.iter().enumerate() {
                if index > 0 {
                    url.push_str("%20");
                }
                write_text(&mut url, scope.as_str());
            }
        }
        if let Some(state) = &self.state {
            url.push_str("&state=");
            write_text(&mut url, state);
        }
        if let Some(redirect_uri) = &self.redirect_uri {
            url.push_str("&redirect_uri=");
            write_text(&mut url, redirect_uri);
        }
        if let Some(prompt) = self.prompt {
            url.push_str("&prompt=");
            url.push_str(prompt.as_str());
        }
        if let Some(permissions) = self.permissions {
            let _ = fmt::Write::write_fmt(
                &mut url,
                format_args!("&permissions={}", permissions.bits()),
            );
        }
        if let Some(guild_id) = self.guild_id {
            let _ = fmt::Write::write_fmt(&mut url, format_args!("&guild_id={guild_id}"));
        }
        if self.disable_guild_select {
            url.push_str("&disable_guild_select=true");
        }
        if let Some(integration_type) = self.integration_type {
            let _ = fmt::Write::write_fmt(
                &mut url,
                format_args!("&integration_type={}", integration_type.value()),
            );
        }
        f.write_str(&url)
    }
}

/// An `application/x-www-form-urlencoded` body for the token endpoints, ending with the
/// client credentials.
pub(crate) struct TokenForm(String);

impl TokenForm {
    /// A form with the `grant_type` (or, for revocation, the first field) `name=value`.
    pub(crate) fn new(name: &str, value: &str) -> Self {
        let mut form = Self(String::with_capacity(160));
        form.0.push_str(name);
        form.0.push('=');
        write_text(&mut form.0, value);
        form
    }

    /// Adds a field.
    pub(crate) fn field(mut self, name: &str, value: &str) -> Self {
        self.0.push('&');
        self.0.push_str(name);
        self.0.push('=');
        write_text(&mut self.0, value);
        self
    }

    /// Adds the client credentials and returns the body.
    pub(crate) fn finish(self, credentials: &OAuth2Credentials) -> String {
        let client_id = credentials.client_id.to_string();
        self.field("client_id", &client_id)
            .field("client_secret", &credentials.client_secret)
            .0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_form_is_percent_encoded() {
        let credentials = OAuth2Credentials::new(ApplicationId::new(7), "s3cr&t=");
        let body = TokenForm::new("grant_type", "authorization_code")
            .field("code", "a b")
            .field("redirect_uri", "https://example.com/cb?x=1")
            .finish(&credentials);
        assert_eq!(
            body,
            "grant_type=authorization_code&code=a%20b\
             &redirect_uri=https%3A%2F%2Fexample.com%2Fcb%3Fx%3D1&client_id=7\
             &client_secret=s3cr%26t%3D"
        );
    }

    #[test]
    fn authorize_url_parameters() {
        let url = AuthorizeUrl::new(ApplicationId::new(1))
            .scopes([Scope::ApplicationsCommands])
            .integration_type(IntegrationType::UserInstall)
            .prompt(Prompt::None);
        assert_eq!(
            url.build(),
            "https://discord.com/oauth2/authorize?client_id=1&scope=applications.commands\
             &prompt=none&integration_type=1"
        );
        assert_eq!(
            AuthorizeUrl::new(ApplicationId::new(1)).build(),
            "https://discord.com/oauth2/authorize?client_id=1"
        );
    }
}
