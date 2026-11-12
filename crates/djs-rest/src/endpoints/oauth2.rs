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
    pub async fn get_current_bot_application_information(&self) -> Result<Application, Error> {
        self.request(Request::new(Route::GetCurrentBotApplicationInformation))
            .await
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
    pub async fn get_current_authorization_information(
        &self,
    ) -> Result<CurrentAuthorizationInformation, Error> {
        self.request(Request::new(Route::GetCurrentAuthorizationInformation))
            .await
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
    pub async fn exchange_code(
        &self,
        credentials: &OAuth2Credentials,
        code: &str,
        redirect_uri: &str,
    ) -> Result<AccessTokenResponse, Error> {
        let form = TokenForm::new("grant_type", "authorization_code")
            .field("code", code)
            .field("redirect_uri", redirect_uri)
            .finish(credentials);
        self.request(Request::new(Route::OAuth2TokenExchange).form_urlencoded(form))
            .await
    }

    /// [Refreshes](https://docs.discord.com/developers/topics/oauth2#authorization-code-grant-refresh-token-exchange-example)
    /// an access token: returns a new access token and a new refresh token.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `400` for an invalid or revoked refresh token.
    pub async fn refresh_access_token(
        &self,
        credentials: &OAuth2Credentials,
        refresh_token: &str,
    ) -> Result<AccessTokenResponse, Error> {
        let form = TokenForm::new("grant_type", "refresh_token")
            .field("refresh_token", refresh_token)
            .finish(credentials);
        self.request(Request::new(Route::OAuth2TokenExchange).form_urlencoded(form))
            .await
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
    pub async fn client_credentials_grant(
        &self,
        credentials: &OAuth2Credentials,
        scopes: &[Scope],
    ) -> Result<AccessTokenResponse, Error> {
        let scope = scopes
            .iter()
            .map(Scope::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        let form = TokenForm::new("grant_type", "client_credentials")
            .field("scope", &scope)
            .finish(credentials);
        self.request(Request::new(Route::OAuth2TokenExchange).form_urlencoded(form))
            .await
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
    pub async fn revoke_token(
        &self,
        credentials: &OAuth2Credentials,
        token: &str,
        hint: Option<TokenTypeHint>,
    ) -> Result<(), Error> {
        let mut form = TokenForm::new("token", token);
        if let Some(hint) = hint {
            form = form.field("token_type_hint", hint.as_str());
        }
        self.request_ignoring_body(
            Request::new(Route::OAuth2TokenRevocation).form_urlencoded(form.finish(credentials)),
        )
        .await
    }
}
