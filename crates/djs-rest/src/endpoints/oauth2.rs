//! [OAuth2 endpoints](https://docs.discord.com/developers/topics/oauth2): the application and
//! authorization behind a token, and the token endpoints (D-131, D-132).
//!
//! The token endpoints take the application's [`OAuth2Credentials`] in an
//! `application/x-www-form-urlencoded` body and are sent without the client's own token, so
//! any [`Rest`] works for them — also one built without a token.

use djs_model::{
    application::Application,
    oauth2::{AccessTokenResponse, CurrentAuthorizationInformation, Scope},
};

use crate::client::boxed;
use crate::{
    Error, Request, Rest, Route,
    oauth2::{OAuth2Credentials, TokenForm, TokenTypeHint},
};

impl Rest {
    /// [Get Current Bot Application Information](https://docs.discord.com/developers/topics/oauth2#get-current-bot-application-information):
    /// returns the bot's application, like
    /// [`get_current_application`](Self::get_current_application).
    ///
    /// discord.js counterpart: `ClientApplication#fetch`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn get_current_bot_application_information(
        &self,
    ) -> impl Future<Output = Result<Application, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetCurrentBotApplicationInformation))
                .await
        })
    }

    /// [Get Current Authorization Information](https://docs.discord.com/developers/topics/oauth2#get-current-authorization-information):
    /// returns what the client's access token is authorized for, when it expires and — with
    /// the `identify` scope — who authorized it. Needs a client built with
    /// [`bearer_token`](crate::RestBuilder::bearer_token).
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `401` for a bot token or an invalid access token.
    pub fn get_current_authorization_information(
        &self,
    ) -> impl Future<Output = Result<CurrentAuthorizationInformation, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetCurrentAuthorizationInformation))
                .await
        })
    }

    /// Exchanges the `code` of an [authorization code grant](https://docs.discord.com/developers/topics/oauth2#authorization-code-grant)
    /// redirect for the user's access token. `redirect_uri` must be the one of the
    /// authorization URL.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Examples
    ///
    /// See the [`oauth2`](crate::oauth2) module.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `400` for an invalid, expired or already used code or a
    /// mismatched redirect URI, `401` for wrong credentials. Discord's `OAuth2` error bodies
    /// (`{"error": "invalid_grant", …}`) are not parsed into an
    /// [`ApiError`](crate::ApiError).
    pub fn exchange_code<'a>(
        &'a self,
        credentials: &'a OAuth2Credentials,
        code: &'a str,
        redirect_uri: &'a str,
    ) -> impl Future<Output = Result<AccessTokenResponse, Error>> + Send + 'a {
        boxed(async move {
            let form = TokenForm::new("grant_type", "authorization_code")
                .field("code", code)
                .field("redirect_uri", redirect_uri)
                .finish(credentials);
            self.call(Request::new(Route::OAuth2TokenExchange).form_urlencoded(form))
                .await
        })
    }

    /// [Refreshes](https://docs.discord.com/developers/topics/oauth2#authorization-code-grant-refresh-token-exchange-example)
    /// an access token: returns a new access token and a new refresh token.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `400` for an invalid or revoked refresh token.
    pub fn refresh_access_token<'a>(
        &'a self,
        credentials: &'a OAuth2Credentials,
        refresh_token: &'a str,
    ) -> impl Future<Output = Result<AccessTokenResponse, Error>> + Send + 'a {
        boxed(async move {
            let form = TokenForm::new("grant_type", "refresh_token")
                .field("refresh_token", refresh_token)
                .finish(credentials);
            self.call(Request::new(Route::OAuth2TokenExchange).form_urlencoded(form))
                .await
        })
    }

    /// The [client credentials grant](https://docs.discord.com/developers/topics/oauth2#client-credentials-grant):
    /// an access token of the application's owner for `scopes`, without a refresh token —
    /// mostly for testing. Team applications may only request `identify` and
    /// `applications.commands.update`.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `400` for a scope that is not allowed.
    pub fn client_credentials_grant<'a>(
        &'a self,
        credentials: &'a OAuth2Credentials,
        scopes: &'a [Scope],
    ) -> impl Future<Output = Result<AccessTokenResponse, Error>> + Send + 'a {
        boxed(async move {
            let scope = scopes
                .iter()
                .map(Scope::as_str)
                .collect::<Vec<_>>()
                .join(" ");
            let form = TokenForm::new("grant_type", "client_credentials")
                .field("scope", &scope)
                .finish(credentials);
            self.call(Request::new(Route::OAuth2TokenExchange).form_urlencoded(form))
                .await
        })
    }

    /// [Revokes](https://docs.discord.com/developers/topics/oauth2#authorization-code-grant-token-revocation-example)
    /// an access or refresh token. Discord revokes every token of the authorization, whichever
    /// is passed.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn revoke_token<'a>(
        &'a self,
        credentials: &'a OAuth2Credentials,
        token: &'a str,
        hint: Option<TokenTypeHint>,
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        boxed(async move {
            let mut form = TokenForm::new("token", token);
            if let Some(hint) = hint {
                form = form.field("token_type_hint", hint.as_str());
            }
            self.request_ignoring_body(
                Request::new(Route::OAuth2TokenRevocation)
                    .form_urlencoded(form.finish(credentials)),
            )
            .await
        })
    }
}
