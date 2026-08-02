//! Best-effort outbound email for the invite and forgot-password flows.
//!
//! Sending email is a delivery *convenience*, not the source of truth for
//! either flow — the token itself already exists in `AppState` (`invites` /
//! `password_resets`) and is returned directly to an authenticated admin for
//! invites (`inviteUrl` in the `POST /api/org/invites` response), so a mail
//! relay outage must never fail the HTTP request that triggered the email.
//! See [`send_email`].
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// SMTP configuration, resolved once at startup from env vars. `None` means
/// no SMTP relay is configured — see [`send_email`] for what happens then.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}

impl SmtpConfig {
    /// Reads `LOREHUB_SMTP_HOST`, `LOREHUB_SMTP_PORT` (default 587),
    /// `LOREHUB_SMTP_USERNAME`, `LOREHUB_SMTP_PASSWORD`, `LOREHUB_SMTP_FROM`.
    /// Returns `None` iff `LOREHUB_SMTP_HOST` is unset — that's the signal
    /// this deployment has no mail relay configured (e.g. local dev),
    /// distinct from a *misconfigured* one (host set but port unparsable,
    /// which panics at startup exactly like `RouterConfig::from_env`'s
    /// existing env-var validation, for the same reason: a bad explicit
    /// value should fail loudly at boot, not silently at first send).
    pub fn from_env() -> Option<Self> {
        let host = std::env::var("LOREHUB_SMTP_HOST").ok()?;

        let port = match std::env::var("LOREHUB_SMTP_PORT") {
            Err(_) => 587,
            Ok(raw) => raw.parse::<u16>().unwrap_or_else(|err| {
                panic!(
                    "LOREHUB_SMTP_PORT is set to {raw:?}, which is not a valid port number: {err}"
                )
            }),
        };

        let username = std::env::var("LOREHUB_SMTP_USERNAME").unwrap_or_default();
        let password = std::env::var("LOREHUB_SMTP_PASSWORD").unwrap_or_default();
        let from = std::env::var("LOREHUB_SMTP_FROM")
            .unwrap_or_else(|_| format!("LoreHub <no-reply@{host}>"));

        Some(Self {
            host,
            port,
            username,
            password,
            from,
        })
    }
}

/// Sends `subject`/`body_text` to `to`. If `config` is `None` (no SMTP relay
/// configured), logs the would-be email at INFO level instead of sending —
/// this is the local-dev fallback so invite/reset flows are still fully
/// exercisable (the link appears in the server's own log output) without a
/// real mail server. A send failure (relay unreachable, auth rejected, etc.)
/// is logged at ERROR level and swallowed rather than propagated — a
/// transient mail-relay outage must not fail the HTTP request that triggered
/// the email (the caller already got a valid invite/reset token either way;
/// email is a delivery convenience, not the source of truth).
pub async fn send_email(config: Option<&SmtpConfig>, to: &str, subject: &str, body_text: &str) {
    let Some(config) = config else {
        tracing::info!(
            to,
            subject,
            body = body_text,
            "no SMTP relay configured (LOREHUB_SMTP_HOST unset) — logging email instead of sending"
        );
        return;
    };

    let from: Mailbox = match config.from.parse() {
        Ok(mailbox) => mailbox,
        Err(err) => {
            tracing::error!(
                error = %err,
                from = %config.from,
                "configured LOREHUB_SMTP_FROM is not a valid mailbox address; email not sent"
            );
            return;
        }
    };
    let to_mailbox: Mailbox = match to.parse() {
        Ok(mailbox) => mailbox,
        Err(err) => {
            tracing::error!(error = %err, %to, "recipient is not a valid mailbox address; email not sent");
            return;
        }
    };

    let message = match Message::builder()
        .from(from)
        .to(to_mailbox)
        .subject(subject)
        .body(body_text.to_string())
    {
        Ok(message) => message,
        Err(err) => {
            tracing::error!(error = %err, "failed to build outgoing email message; email not sent");
            return;
        }
    };

    let transport = match AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host) {
        Ok(builder) => builder
            .port(config.port)
            .credentials(Credentials::new(
                config.username.clone(),
                config.password.clone(),
            ))
            .build(),
        Err(err) => {
            tracing::error!(error = %err, host = %config.host, "failed to build SMTP transport; email not sent");
            return;
        }
    };

    if let Err(err) = transport.send(message).await {
        tracing::error!(error = %err, %to, "failed to send email via SMTP relay");
    }
}

#[cfg(test)]
mod tests {
    use super::SmtpConfig;

    /// The only thing worth unit-testing here without a live relay: the
    /// "no mail server configured" signal. `LOREHUB_SMTP_HOST` is never set
    /// anywhere in this repo's own env, including under `cargo test`, so
    /// this doesn't need to mutate process-global env state to be reliable.
    #[test]
    fn from_env_returns_none_when_host_unset() {
        assert!(std::env::var("LOREHUB_SMTP_HOST").is_err());
        assert!(SmtpConfig::from_env().is_none());
    }
}
