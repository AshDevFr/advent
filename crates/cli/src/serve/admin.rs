//! The admin API under `/api/admin/`: signing in (with the master password,
//! or as a named admin with an email link), the first-admin setup, the
//! invite list, players, stats and exports.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use advent_server::admin::{self, ImportPreview, PlayerDetail, PlayerRow};
use advent_server::mail::Mailer;
use advent_server::players::{self, ImportReport, normalize_email};
use advent_server::stats::{self, EventStats};
use advent_server::{EventData, Player, ServerError, Store};
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::api::{ApiError, ApiResult, check_limit, client_ip, cookie_value};
use super::config::AdminAuth;
use super::limits::Kind;
use super::{AppState, OsEntropy, emails};

const ADMIN_COOKIE: &str = "advent_admin";

/// How long an admin stays signed in.
const ADMIN_SESSION: Duration = Duration::hours(12);

type Shared = State<Arc<AppState>>;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/admin/login", post(login))
        .route("/api/admin/logout", post(logout))
        .route("/api/admin/session", get(session))
        .route("/api/admin/mode", get(mode))
        .route("/api/admin/oidc/start", get(oidc_start))
        .route("/api/admin/oidc/callback", get(oidc_callback))
        .route("/api/admin/setup", post(setup))
        .route("/api/admin/signin/{link}", get(link_owner))
        .route("/api/admin/signin", post(use_link))
        .route("/api/admin/admins", get(list_admins).post(add_admin))
        .route("/api/admin/admins/{email}", delete(remove_admin))
        .route("/api/admin/events", get(events))
        .route("/api/admin/events/{event}/stats", get(event_stats))
        .route("/api/admin/events/{event}/players", get(players_table))
        .route("/api/admin/events/{event}/players/{email}", get(player))
        .route(
            "/api/admin/events/{event}/finishers.csv",
            get(finishers_csv),
        )
        .route("/api/admin/events/{event}/players.csv", get(players_csv))
        .route("/api/admin/import/preview", post(import_preview))
        .route("/api/admin/import", post(import))
        .route(
            "/api/admin/invitations",
            post(send_invitations).get(invitations),
        )
        .route("/api/admin/players/{email}/resend", post(resend))
        .route("/api/admin/players/{email}/reset", post(reset))
        .route("/api/admin/players/{email}", delete(remove))
}

/// Invitations being sent in the background, and how it went.
#[derive(Debug, Default, Clone, Serialize)]
pub struct InviteJob {
    pub running: bool,
    pub total: usize,
    pub sent: usize,
    pub failed: Vec<Failure>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub email: String,
    pub error: String,
}

pub type Jobs = Mutex<InviteJob>;

fn admin_cookie(state: &AppState, value: &str, expires: DateTime<Utc>) -> HeaderValue {
    let secure = if state.config.insecure_cookies {
        ""
    } else {
        "; Secure"
    };
    HeaderValue::from_str(&format!(
        "{ADMIN_COOKIE}={value}; Path=/; HttpOnly; SameSite=Strict{secure}; Expires={}",
        expires.format("%a, %d %b %Y %H:%M:%S GMT")
    ))
    .expect("cookie values are base64 and dates")
}

/// Only a signed-in admin gets past this. A session only counts in the
/// mode that opened it, so switching `admin.auth` signs everyone out.
async fn require_admin(state: &AppState, headers: &HeaderMap) -> ApiResult<Option<String>> {
    let token = cookie_value(headers, ADMIN_COOKIE).ok_or(ServerError::NoSession)?;
    let session = admin::admin_session(&state.store, &token, state.clock.now())
        .await?
        .ok_or(ServerError::NoSession)?;
    match (state.config.admin.auth, &session.email) {
        (AdminAuth::Password, None) | (AdminAuth::Accounts, Some(_)) => Ok(session.email),
        _ => Err(ServerError::NoSession.into()),
    }
}

/// Open an admin session and set its cookie on `response`.
async fn open_session(
    state: &AppState,
    email: Option<String>,
    mut response: Response,
) -> ApiResult<Response> {
    let expires = state.clock.now() + ADMIN_SESSION;
    let token = admin::admin_sign_in(&state.store, &OsEntropy, email, expires).await?;
    response
        .headers_mut()
        .insert(header::SET_COOKIE, admin_cookie(state, &token, expires));
    Ok(response)
}

#[derive(Deserialize)]
struct Login {
    password: Option<String>,
    email: Option<String>,
}

/// Password mode: check the master password. Accounts mode: email a
/// sign-in link to the address if it is an admin's, answering the same
/// either way.
async fn login(
    State(state): Shared,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Login>,
) -> ApiResult<Response> {
    let ip = client_ip(&state, &headers, peer);
    check_limit(&state, Kind::AdminLogin, ip)?;
    if state.config.admin.auth == AdminAuth::Accounts {
        let email = body.email.as_deref().and_then(normalize_email);
        if let Some(email) = email
            && check_limit(&state, Kind::LinkEmail, format!("admin:{email}")).is_ok()
        {
            let state = state.clone();
            tokio::spawn(async move {
                if let Err(e) = send_admin_link(&state, &email).await {
                    eprintln!("error: admin sign-in link for {email}: {e}");
                }
            });
        }
        return Ok((
            StatusCode::ACCEPTED,
            Json(json!({ "message": "If this address belongs to an admin, a sign-in link is on its way." })),
        )
            .into_response());
    }
    let expected = state.config.admin.password.as_deref().unwrap_or_default();
    let given = body.password.unwrap_or_default();
    if expected.is_empty() || !admin::password_matches(&given, expected) {
        eprintln!("warning: failed admin sign-in from {ip}");
        return Err(ApiError(StatusCode::UNAUTHORIZED, "wrong password".into()));
    }
    eprintln!("admin signed in from {ip}");
    open_session(&state, None, StatusCode::NO_CONTENT.into_response()).await
}

/// Email an admin their single-use sign-in link (nothing if not an admin).
async fn send_admin_link(state: &AppState, email: &str) -> Result<(), String> {
    let Some(link) = admin::admin_link(&state.store, &OsEntropy, email, state.clock.now())
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    let url = format!("{}/admin/signin/{link}", state.config.link_base());
    let message = emails::admin_link(&state.content().brand, email, &url);
    state.mailer.send(&message).await.map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct Mode {
    auth: AdminAuth,
    setup_needed: bool,
    /// The single sign-on button's label, when it is configured.
    oidc: Option<String>,
}

/// How to sign in here, for the login page.
async fn mode(State(state): Shared) -> ApiResult<Json<Mode>> {
    let accounts = state.config.admin.auth == AdminAuth::Accounts;
    Ok(Json(Mode {
        auth: state.config.admin.auth,
        // With single sign-on, group members are admins already: the
        // setup code still works but the login page need not ask for it.
        setup_needed: accounts && state.oidc.is_none() && admin::setup_needed(&state.store).await?,
        oidc: state.oidc.as_ref().map(|o| o.label().to_string()),
    }))
}

fn redirect(to: &str) -> Response {
    (
        StatusCode::FOUND,
        [
            (header::LOCATION, to.to_string()),
            (header::CACHE_CONTROL, "no-store".into()),
        ],
    )
        .into_response()
}

/// Back to the admin page with a message for the login form.
fn admin_page_with_error(state: &AppState, message: &str) -> Response {
    let encoded: String = message
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    redirect(&format!(
        "{}/admin?error={encoded}",
        state.config.link_base()
    ))
}

/// Send the browser to the identity provider.
async fn oidc_start(
    State(state): Shared,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let Some(oidc) = &state.oidc else {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "single sign-on is not configured".into(),
        ));
    };
    check_limit(&state, Kind::AdminLogin, client_ip(&state, &headers, peer))?;
    Ok(match oidc.start().await {
        Ok(url) => redirect(&url),
        Err(e) => {
            eprintln!("error: {e}");
            admin_page_with_error(&state, &e.to_string())
        }
    })
}

#[derive(Deserialize)]
struct Callback {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// The provider sends the browser back here.
async fn oidc_callback(
    State(state): Shared,
    axum::extract::Query(query): axum::extract::Query<Callback>,
) -> ApiResult<Response> {
    let Some(oidc) = &state.oidc else {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "single sign-on is not configured".into(),
        ));
    };
    if let Some(error) = query.error {
        return Ok(admin_page_with_error(
            &state,
            &format!("the identity provider refused: {error}"),
        ));
    }
    let (Some(code), Some(returned)) = (query.code, query.state) else {
        return Ok(admin_page_with_error(
            &state,
            "the identity provider sent no code",
        ));
    };
    match oidc.finish(&code, &returned).await {
        Ok(email) => {
            eprintln!("admin {email} signed in with single sign-on");
            let landing = redirect(&format!("{}/admin", state.config.link_base()));
            open_session(&state, Some(email), landing).await
        }
        Err(e) => {
            eprintln!("warning: single sign-on refused: {e}");
            Ok(admin_page_with_error(&state, &e.to_string()))
        }
    }
}

#[derive(Deserialize)]
struct Setup {
    code: String,
    email: String,
}

/// Create the first admin with the setup code from the server log, and
/// sign them in straight away: a mistyped address can then be fixed by
/// adding the right one, rather than locking the server.
async fn setup(
    State(state): Shared,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Setup>,
) -> ApiResult<Response> {
    if state.config.admin.auth != AdminAuth::Accounts {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "no setup in password mode".into(),
        ));
    }
    let ip = client_ip(&state, &headers, peer);
    check_limit(&state, Kind::AdminLogin, ip)?;
    let first = admin::setup(
        &state.store,
        &body.code,
        &state.setup_code,
        &body.email,
        state.clock.now(),
    )
    .await
    .map_err(|e| {
        if e == ServerError::WrongSetupCode {
            eprintln!("warning: wrong setup code from {ip}");
        }
        ApiError::from(e)
    })?;
    eprintln!("first admin created: {}", first.email);
    open_session(
        &state,
        Some(first.email),
        StatusCode::NO_CONTENT.into_response(),
    )
    .await
}

/// Whose admin link it is, without using it.
async fn link_owner(
    State(state): Shared,
    Path(link): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let email = admin::admin_link_owner(&state.store, &link, state.clock.now()).await?;
    Ok(Json(json!({ "email": email })))
}

#[derive(Deserialize)]
struct UseLink {
    link: String,
}

async fn use_link(State(state): Shared, Json(body): Json<UseLink>) -> ApiResult<Response> {
    if state.config.admin.auth != AdminAuth::Accounts {
        return Err(ServerError::InvalidAdminLink.into());
    }
    let email = admin::use_admin_link(&state.store, &body.link, state.clock.now()).await?;
    eprintln!("admin {email} signed in");
    open_session(&state, Some(email), StatusCode::NO_CONTENT.into_response()).await
}

fn accounts_only(state: &AppState) -> ApiResult<()> {
    if state.config.admin.auth == AdminAuth::Accounts {
        Ok(())
    } else {
        Err(ApiError(
            StatusCode::NOT_FOUND,
            "admins exist only with `admin.auth: accounts`".into(),
        ))
    }
}

async fn list_admins(
    State(state): Shared,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<advent_server::Admin>>> {
    accounts_only(&state)?;
    require_admin(&state, &headers).await?;
    Ok(Json(state.store.admins().await.map_err(ServerError::from)?))
}

#[derive(Deserialize)]
struct NewAdmin {
    email: String,
}

/// Add an admin and email them a sign-in link.
async fn add_admin(
    State(state): Shared,
    headers: HeaderMap,
    Json(body): Json<NewAdmin>,
) -> ApiResult<(StatusCode, Json<advent_server::Admin>)> {
    accounts_only(&state)?;
    require_admin(&state, &headers).await?;
    if normalize_email(&body.email).is_none() {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "not an email address".into(),
        ));
    }
    let added = admin::add_admin(&state.store, &body.email, state.clock.now()).await?;
    send_admin_link(&state, &added.email).await.map_err(|e| {
        ApiError(
            StatusCode::BAD_GATEWAY,
            format!("added, but the email could not be sent: {e}"),
        )
    })?;
    Ok((StatusCode::CREATED, Json(added)))
}

async fn remove_admin(
    State(state): Shared,
    headers: HeaderMap,
    Path(email): Path<String>,
) -> ApiResult<StatusCode> {
    accounts_only(&state)?;
    require_admin(&state, &headers).await?;
    admin::remove_admin(&state.store, &email).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn logout(State(state): Shared, headers: HeaderMap) -> ApiResult<Response> {
    if let Some(token) = cookie_value(&headers, ADMIN_COOKIE) {
        admin::admin_sign_out(&state.store, &token).await?;
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        admin_cookie(&state, "", DateTime::<Utc>::UNIX_EPOCH),
    );
    Ok(response)
}

async fn session(State(state): Shared, headers: HeaderMap) -> ApiResult<StatusCode> {
    require_admin(&state, &headers).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// An event as the admin section lists it: its days by number and date,
/// with titles only for released days (unreleased puzzles stay sealed).
#[derive(Serialize)]
struct AdminEvent {
    id: String,
    title: String,
    days: Vec<AdminDay>,
}

#[derive(Serialize)]
struct AdminDay {
    day: u32,
    date: String,
    title: Option<String>,
}

async fn events(State(state): Shared, headers: HeaderMap) -> ApiResult<Json<Vec<AdminEvent>>> {
    require_admin(&state, &headers).await?;
    let content = state.content();
    let list = content
        .events
        .values()
        .map(|data| {
            let event = &data.event;
            AdminEvent {
                id: data.id.clone(),
                title: event.title.clone(),
                days: (1..=event.days)
                    .map(|day| AdminDay {
                        day,
                        date: event.day_date(day).to_string(),
                        title: released_title(&content.site, &data.id, day),
                    })
                    .collect(),
            }
        })
        .collect();
    Ok(Json(list))
}

/// A released day's title, from the page the site publishes for it.
fn released_title(site: &std::path::Path, event: &str, day: u32) -> Option<String> {
    let page = std::fs::read(site.join(event).join(format!("api/day/{day}.json"))).ok()?;
    let page: serde_json::Value = serde_json::from_slice(&page).ok()?;
    page["title"].as_str().map(str::to_string)
}

fn event_data(state: &AppState, event: &str) -> ApiResult<Arc<EventData>> {
    state
        .content()
        .events
        .get(event)
        .cloned()
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no event {event:?}")))
}

async fn event_stats(
    State(state): Shared,
    headers: HeaderMap,
    Path(event): Path<String>,
) -> ApiResult<Json<EventStats>> {
    require_admin(&state, &headers).await?;
    let data = event_data(&state, &event)?;
    Ok(Json(stats::stats(&state.store, &data).await?))
}

async fn players_table(
    State(state): Shared,
    headers: HeaderMap,
    Path(event): Path<String>,
) -> ApiResult<Json<Vec<PlayerRow>>> {
    require_admin(&state, &headers).await?;
    let data = event_data(&state, &event)?;
    Ok(Json(admin::player_rows(&state.store, &data).await?))
}

async fn player(
    State(state): Shared,
    headers: HeaderMap,
    Path((event, email)): Path<(String, String)>,
) -> ApiResult<Json<PlayerDetail>> {
    require_admin(&state, &headers).await?;
    let data = event_data(&state, &event)?;
    Ok(Json(
        admin::player_detail(&state.store, &data, &email).await?,
    ))
}

fn csv(name: &str, body: String) -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        body,
    )
        .into_response()
}

async fn finishers_csv(
    State(state): Shared,
    headers: HeaderMap,
    Path(event): Path<String>,
) -> ApiResult<Response> {
    require_admin(&state, &headers).await?;
    let data = event_data(&state, &event)?;
    Ok(csv(
        &format!("{event}-finishers.csv"),
        admin::finishers_csv(&state.store, &data).await?,
    ))
}

async fn players_csv(
    State(state): Shared,
    headers: HeaderMap,
    Path(event): Path<String>,
) -> ApiResult<Response> {
    require_admin(&state, &headers).await?;
    let data = event_data(&state, &event)?;
    Ok(csv(
        &format!("{event}-players.csv"),
        admin::players_csv(&state.store, &data).await?,
    ))
}

#[derive(Deserialize)]
struct ImportText {
    text: String,
}

async fn import_preview(
    State(state): Shared,
    headers: HeaderMap,
    Json(body): Json<ImportText>,
) -> ApiResult<Json<ImportPreview>> {
    require_admin(&state, &headers).await?;
    Ok(Json(admin::preview(&state.store, &body.text).await?))
}

async fn import(
    State(state): Shared,
    headers: HeaderMap,
    Json(body): Json<ImportText>,
) -> ApiResult<Json<ImportReport>> {
    require_admin(&state, &headers).await?;
    let (invites, invalid) = admin::parse_invites(&body.text);
    let mut report = players::import(&state.store, &OsEntropy, &invites, state.clock.now()).await?;
    report.invalid.extend(invalid);
    Ok(Json(report))
}

async fn invitation_for(state: &AppState, player: &Player) -> advent_server::mail::Email {
    let url = emails::sign_in_url(&state.config.link_base(), &player.link);
    emails::invitation(&state.content().brand, &player.display_email, &url)
}

#[derive(Deserialize)]
struct InviteRequest {
    /// Only these players; without it, everyone not invited yet.
    emails: Option<Vec<String>>,
}

/// Start sending invitations in the background; progress is read with
/// `GET /api/admin/invitations`. One at a time, so a company relay is not
/// flooded and every failure can be retried.
async fn send_invitations(
    State(state): Shared,
    headers: HeaderMap,
    Json(body): Json<InviteRequest>,
) -> ApiResult<(StatusCode, Json<InviteJob>)> {
    require_admin(&state, &headers).await?;
    let all = state.store.players().await.map_err(ServerError::from)?;
    let targets: Vec<Player> = match &body.emails {
        Some(list) => {
            let wanted: Vec<String> = list.iter().filter_map(|e| normalize_email(e)).collect();
            all.into_iter()
                .filter(|p| wanted.contains(&p.email))
                .collect()
        }
        None => all.into_iter().filter(|p| p.invited_at.is_none()).collect(),
    };
    {
        let mut job = state.jobs.lock().unwrap_or_else(|e| e.into_inner());
        if job.running {
            return Err(ApiError(
                StatusCode::CONFLICT,
                "invitations are already being sent".into(),
            ));
        }
        *job = InviteJob {
            running: !targets.is_empty(),
            total: targets.len(),
            ..Default::default()
        };
    }
    let worker = state.clone();
    tokio::spawn(async move {
        for player in targets {
            let email = invitation_for(&worker, &player).await;
            let outcome = match worker.mailer.send(&email).await {
                Ok(()) => players::mark_invited(&worker.store, &player.email, worker.clock.now())
                    .await
                    .map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            };
            let mut job = worker.jobs.lock().unwrap_or_else(|e| e.into_inner());
            match outcome {
                Ok(()) => job.sent += 1,
                Err(error) => {
                    eprintln!("error: invitation to {}: {error}", player.email);
                    job.failed.push(Failure {
                        email: player.display_email.clone(),
                        error,
                    });
                }
            }
        }
        worker
            .jobs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .running = false;
    });
    let job = state.jobs.lock().unwrap_or_else(|e| e.into_inner()).clone();
    Ok((StatusCode::ACCEPTED, Json(job)))
}

async fn invitations(State(state): Shared, headers: HeaderMap) -> ApiResult<Json<InviteJob>> {
    require_admin(&state, &headers).await?;
    Ok(Json(
        state.jobs.lock().unwrap_or_else(|e| e.into_inner()).clone(),
    ))
}

async fn known_player(state: &AppState, email: &str) -> ApiResult<Player> {
    let email = normalize_email(email).ok_or(ServerError::UnknownPlayer)?;
    Ok(state
        .store
        .player(&email)
        .await
        .map_err(ServerError::from)?
        .ok_or(ServerError::UnknownPlayer)?)
}

/// Send one player their invitation (again).
async fn resend(
    State(state): Shared,
    headers: HeaderMap,
    Path(email): Path<String>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers).await?;
    let player = known_player(&state, &email).await?;
    let message = invitation_for(&state, &player).await;
    state.mailer.send(&message).await.map_err(|e| {
        ApiError(
            StatusCode::BAD_GATEWAY,
            format!("the email could not be sent: {e}"),
        )
    })?;
    players::mark_invited(&state.store, &player.email, state.clock.now()).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// A new link for a player whose link went astray: the old one and every
/// session it opened stop working. The new link is not sent automatically.
async fn reset(
    State(state): Shared,
    headers: HeaderMap,
    Path(email): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&state, &headers).await?;
    let player = known_player(&state, &email).await?;
    players::reset_link(&state.store, &OsEntropy, &player.email).await?;
    Ok(Json(json!({ "reset": true })))
}

async fn remove(
    State(state): Shared,
    headers: HeaderMap,
    Path(email): Path<String>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers).await?;
    let player = known_player(&state, &email).await?;
    players::remove(&state.store, &player.email).await?;
    Ok(StatusCode::NO_CONTENT)
}
