//! The admin API under `/api/admin/`: signing in with the master password,
//! the invite list, players, stats and exports.

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

/// Only a signed-in admin gets past this.
async fn require_admin(state: &AppState, headers: &HeaderMap) -> ApiResult<()> {
    let token = cookie_value(headers, ADMIN_COOKIE).ok_or(ServerError::NoSession)?;
    if admin::admin_signed_in(&state.store, &token, state.clock.now()).await? {
        Ok(())
    } else {
        Err(ServerError::NoSession.into())
    }
}

#[derive(Deserialize)]
struct Login {
    password: String,
}

async fn login(
    State(state): Shared,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Login>,
) -> ApiResult<Response> {
    let ip = client_ip(&state, &headers, peer);
    check_limit(&state, Kind::AdminLogin, ip)?;
    let expected = state.config.admin.password.as_deref().unwrap_or_default();
    if expected.is_empty() || !admin::password_matches(&body.password, expected) {
        eprintln!("warning: failed admin sign-in from {ip}");
        return Err(ApiError(StatusCode::UNAUTHORIZED, "wrong password".into()));
    }
    let expires = state.clock.now() + ADMIN_SESSION;
    let token = admin::admin_sign_in(&state.store, &OsEntropy, expires).await?;
    eprintln!("admin signed in from {ip}");
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, admin_cookie(&state, &token, expires));
    Ok(response)
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
