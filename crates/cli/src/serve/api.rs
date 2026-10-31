//! HTTP routes: the player API under `/api/`, and the generated site for
//! everything else.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;

use advent_server::players;
use advent_server::stats::{self, Progress};
use advent_server::{Outcome, Player, ServerError, guess};
use axum::extract::{ConnectInfo, Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::limits::Kind;
use super::{AppState, OsEntropy, emails};
use crate::commands::preview;
use advent_server::mail::Mailer;

const SESSION_COOKIE: &str = "advent_session";

type Shared = State<Arc<AppState>>;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/api/session", get(session))
        .route("/api/signin", post(sign_in))
        .route("/api/signin/request", post(request_link))
        .route("/api/signin/{link}", get(link_owner))
        .route("/signin/{link}", get(app_page))
        .route("/admin", get(app_page))
        .route("/admin/", get(app_page))
        .route("/admin/{*rest}", get(app_page))
        .merge(super::admin::routes())
        .route("/api/signout", post(sign_out))
        .route("/api/name", put(set_name))
        .route("/api/events/{event}/progress", get(progress))
        .route("/api/events/{event}/days/{day}/guess", post(make_guess))
        .route("/api/{*rest}", get(api_not_found).post(api_not_found))
        .fallback(site)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            limit_requests,
        ))
        .with_state(state)
}

/// An API failure: a status and a message for the player.
pub struct ApiError(pub StatusCode, pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<ServerError> for ApiError {
    fn from(e: ServerError) -> Self {
        let status = match &e {
            ServerError::NoSession => StatusCode::UNAUTHORIZED,
            ServerError::InvalidLink | ServerError::UnknownPlayer | ServerError::UnknownDay(_) => {
                StatusCode::NOT_FOUND
            }
            ServerError::NotReleased(_) => StatusCode::FORBIDDEN,
            ServerError::InvalidName(_) => StatusCode::BAD_REQUEST,
            ServerError::Store(_) => {
                eprintln!("error: {e}");
                return ApiError(StatusCode::INTERNAL_SERVER_ERROR, "server error".into());
            }
        };
        ApiError(status, e.to_string())
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

/// The client's address: the connection's, or with `trust_proxy` the last
/// `X-Forwarded-For` entry (the one the proxy itself added).
pub(super) fn client_ip(state: &AppState, headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
    if state.config.trust_proxy
        && let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .and_then(|v| v.trim().parse().ok())
    {
        return ip;
    }
    peer.ip()
}

pub(super) fn check_limit(state: &AppState, kind: Kind, key: impl ToString) -> ApiResult<()> {
    state
        .limiter
        .check(kind, &key.to_string(), Instant::now())
        .map_err(|wait| {
            ApiError(
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "too many requests; try again in {} s",
                    wait.as_secs().max(1)
                ),
            )
        })
}

async fn limit_requests(
    State(state): Shared,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let ip = client_ip(&state, request.headers(), peer);
    match check_limit(&state, Kind::Request, ip) {
        Ok(()) => next.run(request).await,
        Err(e) => e.into_response(),
    }
}

/// A cookie's value from the request.
pub(super) fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(n, _)| *n == name)
        .map(|(_, value)| value.to_string())
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    cookie_value(headers, SESSION_COOKIE)
}

async fn signed_in(state: &AppState, headers: &HeaderMap) -> ApiResult<Player> {
    let token = session_token(headers).ok_or(ServerError::NoSession)?;
    Ok(players::session_player(&state.store, &token, state.clock.now()).await?)
}

fn session_cookie(state: &AppState, value: &str, expires: DateTime<Utc>) -> HeaderValue {
    let secure = if state.config.insecure_cookies {
        ""
    } else {
        "; Secure"
    };
    let cookie = format!(
        "{SESSION_COOKIE}={value}; Path=/; HttpOnly; SameSite=Lax{secure}; Expires={}",
        expires.format("%a, %d %b %Y %H:%M:%S GMT")
    );
    HeaderValue::from_str(&cookie).expect("cookie values are base64 and dates")
}

#[derive(Serialize)]
struct Me {
    email: String,
    name: Option<String>,
}

impl From<Player> for Me {
    fn from(p: Player) -> Self {
        Me {
            email: p.display_email,
            name: p.name,
        }
    }
}

async fn session(State(state): Shared, headers: HeaderMap) -> ApiResult<Json<Me>> {
    Ok(Json(signed_in(&state, &headers).await?.into()))
}

#[derive(Deserialize)]
struct SignIn {
    link: String,
}

async fn sign_in(
    State(state): Shared,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<SignIn>,
) -> ApiResult<Response> {
    check_limit(&state, Kind::SignIn, client_ip(&state, &headers, peer))?;
    let now = state.clock.now();
    // Sessions last until the calendar ends; a server still running an
    // ended calendar gives a month, for players coming back to their
    // certificate.
    let ends = state.content().ends_at;
    let expires = if ends > now {
        ends
    } else {
        now + chrono::Duration::days(30)
    };
    let (token, player) =
        players::sign_in(&state.store, &OsEntropy, &body.link, now, expires).await?;
    let mut response = Json(Me::from(player)).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, session_cookie(&state, &token, expires));
    Ok(response)
}

/// Who a link signs in as, without signing in: the link page shows it
/// and asks for a click, because mail scanners open every link in an
/// email and a plain visit must not sign anyone in.
async fn link_owner(State(state): Shared, Path(link): Path<String>) -> ApiResult<Json<Me>> {
    Ok(Json(
        players::link_player(&state.store, &link).await?.into(),
    ))
}

/// The app's page, for routes the frontend handles itself.
async fn app_page(State(state): Shared) -> Response {
    serve_file(state, "/".into()).await
}

#[derive(Deserialize)]
struct LinkRequest {
    email: String,
}

/// Always the same answer, whether or not the address is invited, so the
/// form cannot be used to find out who is.
async fn request_link(
    State(state): Shared,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<LinkRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    check_limit(&state, Kind::SignIn, client_ip(&state, &headers, peer))?;
    if let Some(email) = players::normalize_email(&body.email)
        && check_limit(&state, Kind::LinkEmail, &email).is_ok()
    {
        // Looked up and sent after answering, so neither the answer nor its
        // timing tells whether the address is invited.
        let state = state.clone();
        tokio::spawn(async move {
            match advent_server::Store::player(&state.store, &email).await {
                Ok(Some(player)) => {
                    let content = state.content();
                    let url = emails::sign_in_url(&state.config.link_base(), &player.link);
                    let message = emails::lost_link(&content.brand, &player.display_email, &url);
                    if let Err(e) = state.mailer.send(&message).await {
                        eprintln!("error: sending the sign-in link to {email}: {e}");
                    }
                }
                Ok(None) => {}
                Err(e) => eprintln!("error: {e}"),
            }
        });
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "message": "If this address is invited, a sign-in link is on its way." })),
    ))
}

async fn sign_out(State(state): Shared, headers: HeaderMap) -> ApiResult<Response> {
    if let Some(token) = session_token(&headers) {
        players::sign_out(&state.store, &token).await?;
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        session_cookie(&state, "", DateTime::<Utc>::UNIX_EPOCH),
    );
    Ok(response)
}

#[derive(Deserialize)]
struct Name {
    name: String,
}

async fn set_name(
    State(state): Shared,
    headers: HeaderMap,
    Json(body): Json<Name>,
) -> ApiResult<Json<Me>> {
    let player = signed_in(&state, &headers).await?;
    Ok(Json(
        players::set_name(&state.store, &player.email, &body.name)
            .await?
            .into(),
    ))
}

fn event_data(state: &AppState, event: &str) -> ApiResult<Arc<advent_server::EventData>> {
    state
        .content()
        .events
        .get(event)
        .cloned()
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no event {event:?}")))
}

async fn progress(
    State(state): Shared,
    headers: HeaderMap,
    Path(event): Path<String>,
) -> ApiResult<Json<Progress>> {
    let player = signed_in(&state, &headers).await?;
    let data = event_data(&state, &event)?;
    Ok(Json(
        stats::progress(&state.store, &data, &player.email).await?,
    ))
}

#[derive(Deserialize)]
struct Guess {
    answer: String,
}

async fn make_guess(
    State(state): Shared,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path((event, day)): Path<(String, u32)>,
    Json(body): Json<Guess>,
) -> ApiResult<Response> {
    let player = signed_in(&state, &headers).await?;
    check_limit(&state, Kind::Guess, client_ip(&state, &headers, peer))?;
    let data = event_data(&state, &event)?;
    let outcome = guess(
        &state.store,
        &state.checker,
        &data,
        &player.email,
        day,
        &body.answer,
        state.clock.now(),
    )
    .await?;
    Ok(match outcome {
        Outcome::Solved { word } | Outcome::AlreadySolved { word } => {
            Json(json!({ "word": word })).into_response()
        }
        Outcome::Wrong { remaining } => {
            Json(json!({ "wrong": true, "remaining": remaining })).into_response()
        }
        Outcome::Limited { retry_at } => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "too many guesses",
                "retry_at": retry_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            })),
        )
            .into_response(),
    })
}

async fn api_not_found() -> ApiError {
    ApiError(StatusCode::NOT_FOUND, "no such API route".into())
}

/// The generated site. Event data changes at each release, so nothing is
/// cached by the browser.
async fn site(State(state): Shared, uri: Uri) -> Response {
    serve_file(state, uri.path().to_string()).await
}

async fn serve_file(state: Arc<AppState>, path: String) -> Response {
    let content = state.content();
    let site = content.site.clone();
    let file = tokio::task::spawn_blocking(move || {
        // Keep the folder alive while it is read.
        let _content = content;
        preview::respond(&site, &path)
    })
    .await;
    match file {
        Ok(file) => (
            StatusCode::from_u16(file.status).unwrap_or(StatusCode::NOT_FOUND),
            [
                (header::CONTENT_TYPE, file.content_type),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            file.body,
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
