//! What the server writes to players: the invitation and the lost-link
//! email. Plain text with a simple HTML version in the brand's colour; no
//! remote images, which mail clients block anyway.

use advent_server::mail::Email;

/// The brand as emails show it.
#[derive(Debug, Clone)]
pub struct Branding {
    pub name: String,
    /// The light palette's primary colour, a CSS colour.
    pub primary: String,
}

pub fn sign_in_url(link_base: &str, token: &str) -> String {
    format!("{link_base}/signin/{token}")
}

// Sent from the admin section, when invitations go out.
#[allow(dead_code)]
pub fn invitation(brand: &Branding, to: &str, url: &str) -> Email {
    compose(
        brand,
        to,
        &format!("Your invitation to {}", brand.name),
        &format!("You are invited to play {}.", brand.name),
        "Open the calendar",
        url,
    )
}

pub fn lost_link(brand: &Branding, to: &str, url: &str) -> Email {
    compose(
        brand,
        to,
        &format!("Your link to {}", brand.name),
        &format!(
            "Here is your personal link to {}, as you asked.",
            brand.name
        ),
        "Open the calendar",
        url,
    )
}

fn compose(
    brand: &Branding,
    to: &str,
    subject: &str,
    lead: &str,
    action: &str,
    url: &str,
) -> Email {
    let note = "This link is personal: it signs you in as you, on any device. Keep it to yourself.";
    let text = format!("{lead}\n\n{action}: {url}\n\n{note}\n");
    let html = format!(
        r#"<!doctype html><html><body style="margin:0;padding:24px;font-family:Georgia,serif;background:#f6f3ec;color:#2a2620">
<div style="max-width:520px;margin:0 auto;background:#ffffff;border-radius:12px;padding:32px">
<h1 style="margin:0 0 16px;font-size:22px">{name}</h1>
<p style="font-size:16px;line-height:1.5">{lead}</p>
<p style="margin:28px 0"><a href="{url}" style="background:{primary};color:#ffffff;text-decoration:none;padding:12px 22px;border-radius:8px;font-family:sans-serif;font-size:15px">{action}</a></p>
<p style="font-size:13px;color:#6b6358;line-height:1.5">{note}<br>{url}</p>
</div></body></html>"#,
        name = escape(&brand.name),
        lead = escape(lead),
        action = escape(action),
        url = escape(url),
        note = escape(note),
        primary = escape(&brand.primary),
    );
    Email {
        to: to.to_string(),
        subject: subject.to_string(),
        text,
        html: Some(html),
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serve_emails_carry_the_link_and_escape_the_brand() {
        let brand = Branding {
            name: "Acme <Advent>".into(),
            primary: "#112233".into(),
        };
        let url = sign_in_url("https://advent.acme.com", "tok");
        let email = invitation(&brand, "alice@acme.com", &url);
        assert_eq!(email.subject, "Your invitation to Acme <Advent>");
        assert!(email.text.contains("https://advent.acme.com/signin/tok"));
        let html = email.html.unwrap();
        assert!(html.contains("Acme &lt;Advent&gt;"));
        assert!(html.contains("href=\"https://advent.acme.com/signin/tok\""));
        assert!(html.contains("#112233"));
    }
}
