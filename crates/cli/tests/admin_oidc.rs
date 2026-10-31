//! Admin single sign-on against a stand-in identity provider: discovery,
//! signing keys and a token endpoint, with ID tokens signed by a key made
//! for the test.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use common::sample_event;
use common::server::{Server, install_crypto, start};
use reqwest::StatusCode;
use reqwest::blocking::Client;
use rsa::RsaPrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::sha2::Sha256;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::traits::PublicKeyParts;
use serde_json::{Value, json};

const CLIENT_ID: &str = "advent-test";

/// What the provider says about the user, and what it was asked.
#[derive(Default)]
struct Provider {
    groups: Vec<String>,
    /// The PKCE verifiers sent to the token endpoint.
    verifiers: Vec<String>,
}

/// A minimal OpenID provider on a local port; returns its issuer URL.
fn provider(state: Arc<Mutex<Provider>>) -> String {
    let key = RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let issuer = format!("http://{}", listener.local_addr().unwrap());
    let base = issuer.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut out = stream.try_clone().unwrap();
            let mut reader = BufReader::new(stream);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                    length = v.trim().parse().unwrap();
                }
                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let path = request_line.split_whitespace().nth(1).unwrap_or("/");
            let reply = match path {
                "/.well-known/openid-configuration" => json!({
                    "issuer": base,
                    "authorization_endpoint": format!("{base}/authorize"),
                    "token_endpoint": format!("{base}/token"),
                    "jwks_uri": format!("{base}/jwks"),
                    "response_types_supported": ["code"],
                    "subject_types_supported": ["public"],
                    "id_token_signing_alg_values_supported": ["RS256"],
                }),
                "/jwks" => json!({ "keys": [{
                    "kty": "RSA", "use": "sig", "alg": "RS256", "kid": "test",
                    "n": URL_SAFE_NO_PAD.encode(key.n().to_bytes_be()),
                    "e": URL_SAFE_NO_PAD.encode(key.e().to_bytes_be()),
                }]}),
                "/token" => {
                    let form: Vec<(String, String)> = url_form(&String::from_utf8_lossy(&body));
                    let field = |name: &str| {
                        form.iter()
                            .find(|(k, _)| k == name)
                            .map(|(_, v)| v.clone())
                            .unwrap_or_default()
                    };
                    let mut provider = state.lock().unwrap();
                    provider.verifiers.push(field("code_verifier"));
                    // The test passes the nonce as the code.
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let claims = json!({
                        "iss": base, "sub": "user-1", "aud": CLIENT_ID,
                        "iat": now, "exp": now + 300, "nonce": field("code"),
                        "email": "ana@acme.com", "email_verified": true,
                        "groups": provider.groups,
                    });
                    json!({
                        "access_token": "access", "token_type": "Bearer", "expires_in": 300,
                        "id_token": sign(&key, &claims),
                    })
                }
                _ => json!({}),
            };
            let text = reply.to_string();
            let _ = write!(
                out,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                text.len()
            );
        }
    });
    issuer
}

fn url_form(text: &str) -> Vec<(String, String)> {
    text.split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (decode(k), decode(v)))
        .collect()
}

fn decode(text: &str) -> String {
    let bytes = text.replace('+', " ").into_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            out.push(
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap(), 16).unwrap(),
            );
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap()
}

fn sign(key: &RsaPrivateKey, claims: &Value) -> String {
    let header =
        URL_SAFE_NO_PAD.encode(json!({ "alg": "RS256", "kid": "test", "typ": "JWT" }).to_string());
    let payload = URL_SAFE_NO_PAD.encode(claims.to_string());
    let signing = SigningKey::<Sha256>::new(key.clone());
    let signature = signing.sign(format!("{header}.{payload}").as_bytes());
    format!(
        "{header}.{payload}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    )
}

fn server_with_sso(issuer: &str) -> (Server, common::Repo) {
    let repo = sample_event();
    let server = start(
        &repo,
        "2026-11-20T09:00:00Z",
        &[
            ("ADVENT_ADMIN__AUTH", "accounts"),
            ("ADVENT_ADMIN__SETUP_CODE", "UNUSED"),
            ("ADVENT_ADMIN__OIDC__ISSUER", issuer),
            ("ADVENT_ADMIN__OIDC__CLIENT_ID", CLIENT_ID),
            ("ADVENT_ADMIN__OIDC__CLIENT_SECRET", "secret"),
            ("ADVENT_ADMIN__OIDC__ADMIN_GROUP", "advent-admins"),
            ("ADVENT_ADMIN__OIDC__LABEL", "Acme SSO"),
            ("ADVENT_PUBLIC_URL", "https://advent.acme.com"),
        ],
    );
    // The repo's temporary folder must outlive the server.
    (server, repo)
}

/// A browser that keeps cookies but does not follow redirects, so each hop
/// can be looked at.
fn browser() -> Client {
    install_crypto();
    Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

/// Start a sign-in; returns the state and nonce from the provider URL.
fn begin(server: &Server, c: &Client) -> (String, String) {
    let response = server.get(c, "/api/admin/oidc/start");
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response.headers()["location"].to_str().unwrap().to_string();
    let query: Vec<(String, String)> = url_form(location.split_once('?').unwrap().1);
    let get = |name: &str| query.iter().find(|(k, _)| k == name).unwrap().1.clone();
    assert!(location.contains("/authorize?"), "{location}");
    assert_eq!(get("code_challenge_method"), "S256");
    (get("state"), get("nonce"))
}

fn callback(server: &Server, c: &Client, code: &str, state: &str) -> reqwest::blocking::Response {
    server.get(
        c,
        &format!("/api/admin/oidc/callback?code={code}&state={state}"),
    )
}

#[test]
fn admin_oidc_signs_in_members_of_the_admin_group() {
    let provider_state = Arc::new(Mutex::new(Provider {
        groups: vec!["staff".into(), "advent-admins".into()],
        ..Default::default()
    }));
    let issuer = provider(provider_state.clone());
    let (server, _keep) = server_with_sso(&issuer);
    let c = browser();

    let mode: Value = server.get(&c, "/api/admin/mode").json().unwrap();
    assert_eq!(mode["oidc"], "Acme SSO");

    let (state, nonce) = begin(&server, &c);
    let done = callback(&server, &c, &nonce, &state);
    assert_eq!(done.status(), StatusCode::FOUND);
    assert_eq!(done.headers()["location"], "https://advent.acme.com/admin");
    assert!(
        done.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .starts_with("advent_admin=")
    );
    assert_eq!(
        server.get(&c, "/api/admin/session").status(),
        StatusCode::NO_CONTENT
    );
    let verifiers = &provider_state.lock().unwrap().verifiers;
    assert!(
        verifiers[0].len() >= 43,
        "PKCE verifier sent: {verifiers:?}"
    );
}

#[test]
fn admin_oidc_refuses_users_outside_the_group() {
    let provider_state = Arc::new(Mutex::new(Provider {
        groups: vec!["staff".into()],
        ..Default::default()
    }));
    let issuer = provider(provider_state);
    let (server, _keep) = server_with_sso(&issuer);
    let c = browser();
    let (state, nonce) = begin(&server, &c);
    let refused = callback(&server, &c, &nonce, &state);
    assert_eq!(refused.status(), StatusCode::FOUND);
    let location = refused.headers()["location"].to_str().unwrap().to_string();
    assert!(
        location.contains("/admin?error=you%20are%20not%20in%20the%20admin%20group"),
        "{location}"
    );
    assert!(refused.headers().get("set-cookie").is_none());
    assert_eq!(
        server.get(&c, "/api/admin/session").status(),
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn admin_oidc_refuses_unknown_replayed_or_tampered_requests() {
    let provider_state = Arc::new(Mutex::new(Provider {
        groups: vec!["advent-admins".into()],
        ..Default::default()
    }));
    let issuer = provider(provider_state);
    let (server, _keep) = server_with_sso(&issuer);
    let c = browser();

    // A state this server never issued.
    let forged = callback(&server, &c, "nonce", "made-up-state");
    assert!(
        forged.headers()["location"]
            .to_str()
            .unwrap()
            .contains("error=this%20sign-in%20has%20expired")
    );

    // The right state with a wrong nonce: the ID token does not match.
    let (state, _nonce) = begin(&server, &c);
    let mismatched = callback(&server, &c, "another-nonce", &state);
    assert!(
        mismatched.headers()["location"]
            .to_str()
            .unwrap()
            .contains("error=single%20sign-on%20failed")
    );
    assert!(mismatched.headers().get("set-cookie").is_none());

    // A state works once.
    let (state, nonce) = begin(&server, &c);
    assert!(
        callback(&server, &c, &nonce, &state)
            .headers()
            .get("set-cookie")
            .is_some()
    );
    let again = callback(&server, &browser(), &nonce, &state);
    assert!(again.headers().get("set-cookie").is_none());
}
