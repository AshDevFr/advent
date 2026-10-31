//! Single sign-on for admins with OpenID Connect: the authorization code
//! flow with PKCE, a state and a nonce, and a group check on the verified
//! ID token. Members of `admin.oidc.admin_group` are admins; nobody else is,
//! whatever the provider says about them otherwise.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Context;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, HttpRequest, HttpResponse, IssuerUrl,
    Nonce, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
};

use super::config::OidcConfig;

/// How long a started sign-in may take at the provider.
const PENDING_LIFETIME: Duration = Duration::from_secs(600);

/// A sign-in started here, waiting for the provider to send the browser back.
struct Pending {
    verifier: PkceCodeVerifier,
    nonce: Nonce,
    started: Instant,
}

pub struct Oidc {
    config: OidcConfig,
    redirect: String,
    http: reqwest::Client,
    pending: Mutex<HashMap<String, Pending>>,
}

/// Why a sign-in was refused, in words for the admin page.
#[derive(Debug, thiserror::Error)]
pub enum OidcError {
    #[error("this sign-in has expired or was not started here; try again")]
    UnknownState,
    #[error("you are not in the admin group")]
    NotAdmin,
    #[error("the identity provider did not give an email address")]
    NoEmail,
    #[error("single sign-on failed: {0}")]
    Provider(String),
}

fn provider(e: impl std::fmt::Display) -> OidcError {
    OidcError::Provider(e.to_string())
}

/// A request to the provider through our HTTP client. It owns the client,
/// so the future borrows nothing and can be sent between threads.
async fn send(
    client: reqwest::Client,
    request: HttpRequest,
) -> Result<HttpResponse, reqwest::Error> {
    let (parts, body) = request.into_parts();
    let response = client
        .request(parts.method, parts.uri.to_string())
        .headers(parts.headers)
        .body(body)
        .send()
        .await?;
    let mut builder = http::Response::builder().status(response.status());
    for (name, value) in response.headers() {
        builder = builder.header(name, value);
    }
    let bytes = response.bytes().await?;
    Ok(builder
        .body(bytes.to_vec())
        .expect("a response copied from reqwest"))
}

/// The ID token's claims, read after its signature has been verified.
fn token_payload(jwt: &str) -> Option<serde_json::Value> {
    let payload = jwt.split('.').nth(1)?;
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()
}

impl Oidc {
    pub fn new(config: &OidcConfig, link_base: &str) -> anyhow::Result<Option<Self>> {
        if config.issuer.is_none() {
            return Ok(None);
        }
        // The provider's endpoints must not redirect our requests elsewhere.
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .context("creating the single sign-on HTTP client")?;
        Ok(Some(Self {
            config: config.clone(),
            redirect: format!("{link_base}/api/admin/oidc/callback"),
            http,
            pending: Mutex::new(HashMap::new()),
        }))
    }

    pub fn label(&self) -> &str {
        &self.config.label
    }

    /// The provider's metadata, fetched each sign-in: admins sign in rarely,
    /// and a provider that rotates its keys is then always current.
    async fn client(
        &self,
    ) -> Result<
        CoreClient<
            openidconnect::EndpointSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointMaybeSet,
            openidconnect::EndpointMaybeSet,
        >,
        OidcError,
    > {
        let owned = self.http.clone();
        let http = move |request| send(owned.clone(), request);
        let issuer =
            IssuerUrl::new(self.config.issuer.clone().unwrap_or_default()).map_err(provider)?;
        let metadata = CoreProviderMetadata::discover_async(issuer, &http)
            .await
            .map_err(provider)?;
        Ok(CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(self.config.client_id.clone().unwrap_or_default()),
            self.config.client_secret.clone().map(ClientSecret::new),
        )
        .set_redirect_uri(RedirectUrl::new(self.redirect.clone()).map_err(provider)?))
    }

    /// Start a sign-in: where to send the browser.
    pub async fn start(&self) -> Result<String, OidcError> {
        let client = self.client().await?;
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state, nonce) = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("email".into()))
            .add_scope(Scope::new("profile".into()))
            .set_pkce_challenge(challenge)
            .url();
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        pending.retain(|_, p| p.started.elapsed() < PENDING_LIFETIME);
        pending.insert(
            state.secret().clone(),
            Pending {
                verifier,
                nonce,
                started: Instant::now(),
            },
        );
        Ok(url.to_string())
    }

    /// Finish a sign-in: the admin's email if the provider vouches for them
    /// and they are in the admin group. Each state is used once.
    pub async fn finish(&self, code: &str, state: &str) -> Result<String, OidcError> {
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(state)
            .filter(|p| p.started.elapsed() < PENDING_LIFETIME)
            .ok_or(OidcError::UnknownState)?;
        let client = self.client().await?;
        let owned = self.http.clone();
        let http = move |request| send(owned.clone(), request);
        let token = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(provider)?
            .set_pkce_verifier(pending.verifier)
            .request_async(&http)
            .await
            .map_err(provider)?;
        let id_token = openidconnect::TokenResponse::id_token(&token)
            .ok_or_else(|| provider("no ID token in the response"))?;
        let claims = id_token
            .claims(&client.id_token_verifier(), &pending.nonce)
            .map_err(provider)?;
        let email = claims
            .email()
            .map(|e| e.to_string())
            .ok_or(OidcError::NoEmail)?;

        // The signature is verified above; the groups claim is read from the
        // same token, under whatever name the provider uses.
        let payload = token_payload(&id_token.to_string()).unwrap_or_default();
        let groups: Vec<&str> = match &payload[self.config.groups_claim.as_str()] {
            serde_json::Value::Array(items) => items.iter().filter_map(|g| g.as_str()).collect(),
            serde_json::Value::String(one) => vec![one.as_str()],
            _ => Vec::new(),
        };
        let wanted = self.config.admin_group.as_deref().unwrap_or_default();
        if !groups.contains(&wanted) {
            return Err(OidcError::NotAdmin);
        }
        Ok(email)
    }
}
