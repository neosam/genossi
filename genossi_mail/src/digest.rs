//! Digest-Worker (Phase 20) — täglicher Posteingangs-Benachrichtigungs-Worker.
//!
//! Baut exakt nach dem Vorbild `genossi_service_impl/src/timestamp_worker.rs`:
//! ein config-getriebener Poll-Loop (~60s), der Server-Lokalzeit + letztes
//! Versanddatum vergleicht (D-02, D-04). Zur konfigurierten Uhrzeit sammelt er
//! alle offenen (nicht-archivierten) Mails und verschickt pro Empfänger genau
//! eine Plain-Text-Digest-Mail pro Kalendertag (D-05). Ein verpasstes Fenster
//! wird beim nächsten Lauf nachgeholt (Catch-up, D-01).
//!
//! Die Versand-Logik liegt in reinen, unit-getesteten Helfern (`parse_recipients`,
//! `parse_send_time`, `is_due`, `build_digest_subject`, `build_digest_body`),
//! sodass alle Edge-Cases ohne laufenden Loop testbar sind.

use genossi_config::dao::ConfigEntry;
use std::sync::Arc;

/// Poll-Intervall des Worker-Loops (KEIN sleep-bis-Uhrzeit, D-04).
const POLL_INTERVAL_SECS: u64 = 60;

/// Config-Key: komma-getrennte Empfänger-Liste (muss identisch zum Frontend, Plan 03).
const CFG_RECIPIENTS: &str = "digest_recipients";
/// Config-Key: Versand-Uhrzeit im Format "HH:MM" (muss identisch zum Frontend, Plan 03).
const CFG_SEND_TIME: &str = "digest_send_time";

/// Parst das komma-getrennte `digest_recipients`-Feld in eine getrimmte Liste
/// ohne leere Einträge. Fehlender Key ⇒ leere Liste (DIGEST-07).
pub(crate) fn parse_recipients(entries: &[ConfigEntry]) -> Vec<String> {
    entries
        .iter()
        .find(|e| e.key.as_ref() == CFG_RECIPIENTS)
        .map(|e| {
            e.value
                .as_ref()
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Parst `digest_send_time` ("HH:MM") in (Stunde, Minute). Ungültig/leer/out-of-range ⇒ None.
pub(crate) fn parse_send_time(entries: &[ConfigEntry]) -> Option<(u8, u8)> {
    let raw = entries.iter().find(|e| e.key.as_ref() == CFG_SEND_TIME)?;
    let mut parts = raw.value.as_ref().split(':');
    let h: u8 = parts.next()?.trim().parse().ok()?;
    let m: u8 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if h > 23 || m > 59 {
        return None;
    }
    Some((h, m))
}

/// D-01 Catch-up + D-03 ein-Versand-pro-Tag. `now_local` = Server-Lokalzeit.
///
/// - heute schon gesendet ⇒ false
/// - sonst fällig, sobald die konfigurierte Uhrzeit am heutigen Tag erreicht/
///   überschritten ist (das deckt sowohl den pünktlichen Lauf als auch den
///   Catch-up nach verpasstem Fenster ab).
pub(crate) fn is_due(
    now_local: time::OffsetDateTime,
    send_time: (u8, u8),
    last_sent_date: Option<time::Date>,
) -> bool {
    let today = now_local.date();
    if last_sent_date == Some(today) {
        return false; // heute schon gesendet
    }
    let (h, m) = send_time;
    let now_minutes = now_local.hour() as u32 * 60 + now_local.minute() as u32;
    let send_minutes = h as u32 * 60 + m as u32;
    now_minutes >= send_minutes
}

/// Betreff der Digest-Mail mit Anzahl offener Mails (D-09).
pub(crate) fn build_digest_subject(count: usize) -> String {
    let noun = if count == 1 {
        "offene Mail"
    } else {
        "offene Mails"
    };
    format!("Posteingang: {} {}", count, noun)
}

/// Plain-Text-Body (D-08): hardcodierter deutscher Text, je eine Zeile pro
/// offener Mail (Titel, Absender, Eingangszeit) in der übergebenen Reihenfolge
/// (neueste zuerst, D-10) plus {inbox_url}-Deep-Link (D-11).
pub(crate) fn build_digest_body(mails: &[&crate::dao::InboundMail], inbox_url: &str) -> String {
    let mut body = String::from("Guten Tag,\n\nim Posteingang liegen offene Nachrichten:\n\n");
    for m in mails {
        body.push_str(&format!(
            "- {} (von: {}, eingegangen: {})\n",
            m.subject, m.from_address, m.received_at
        ));
    }
    body.push_str(&format!("\nZum Posteingang: {}\n", inbox_url));
    body
}

/// Digest-Worker-Loop. Pollt periodisch (~60s), vergleicht Server-Lokalzeit +
/// letztes Versanddatum und verschickt bei Fälligkeit pro Empfänger eine
/// Digest-Mail. Spiegelt die Struktur von `start_timestamp_worker`.
pub async fn start_digest_worker<C, I, M, S>(
    config_service: Arc<C>,
    inbox_service: Arc<I>,
    mail_service: Arc<M>,
    digest_state_dao: Arc<S>,
) where
    C: genossi_config::service::ConfigService,
    I: crate::inbox::InboxService,
    M: crate::service::MailService,
    S: crate::dao::DigestStateDao,
{
    tracing::info!("Digest worker started");

    loop {
        let entries = match config_service.get_all().await {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("Digest worker: failed to read config: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
                continue;
            }
        };

        let recipients = parse_recipients(&entries);
        let send_time = parse_send_time(&entries);

        // DIGEST-07/D-14: keine Empfänger ODER keine gültige Uhrzeit ⇒ skip ohne Fehler
        if recipients.is_empty() || send_time.is_none() {
            tracing::debug!("Digest worker: no recipients or no send time, skipping");
            tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
            continue;
        }
        let send_time = send_time.unwrap();

        // Server-Lokalzeit (D-02). Fallback auf now_utc, falls now_local fehlschlägt.
        let now_local =
            time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());

        let last_sent = digest_state_dao.get_last_sent_date().await.unwrap_or(None);

        if is_due(now_local, send_time, last_sent) {
            // offene (nicht-archivierte) Mails laden (DIGEST-04/05).
            // InboxService::list() liefert bereits ORDER BY received_at DESC (D-10).
            match inbox_service.list().await {
                Ok(mails) => {
                    let offen: Vec<&crate::dao::InboundMail> =
                        mails.iter().filter(|m| !m.archived).collect();
                    if offen.is_empty() {
                        // KEIN set_last_sent_date — leerer Tag gilt nicht als erledigt (DIGEST-04)
                        tracing::debug!("Digest worker: inbox empty, not sending (DIGEST-04)");
                    } else {
                        let app_url = std::env::var("APP_URL")
                            .unwrap_or_else(|_| "http://localhost:3000/".to_string());
                        let inbox_url = format!("{}/inbox", app_url.trim_end_matches('/'));
                        let subject = build_digest_subject(offen.len());
                        let body = build_digest_body(&offen, &inbox_url);

                        // Einzelmail pro Empfänger (D-06), Fehler loggen+weiter (D-07).
                        for recipient in &recipients {
                            match mail_service
                                .send_test_mail_with_body(recipient, &subject, &body)
                                .await
                            {
                                Ok(()) => tracing::info!("Digest worker: sent to {}", recipient),
                                Err(e) => tracing::error!(
                                    "Digest worker: send to {} failed: {:?}",
                                    recipient,
                                    e
                                ),
                            }
                        }
                        // Versanddatum trotzdem setzen (D-07: Tag gilt als erledigt, kein Retry).
                        if let Err(e) = digest_state_dao.set_last_sent_date(now_local.date()).await
                        {
                            tracing::error!("Digest worker: failed to persist sent date: {:?}", e);
                        }
                    }
                }
                Err(e) => tracing::error!("Digest worker: failed to list inbox: {:?}", e),
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::InboundMail;
    use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};
    use uuid::Uuid;

    fn make_entry(key: &str, value: &str) -> ConfigEntry {
        ConfigEntry {
            key: Arc::from(key),
            value: Arc::from(value),
            value_type: Arc::from("string"),
        }
    }

    /// Konstruiert eine OffsetDateTime (UTC) ohne `time::macros` (im Workspace nicht aktiviert).
    fn dt(year: i32, month: Month, day: u8, hour: u8, minute: u8) -> OffsetDateTime {
        let date = Date::from_calendar_date(year, month, day).unwrap();
        let time = Time::from_hms(hour, minute, 0).unwrap();
        PrimitiveDateTime::new(date, time).assume_utc()
    }

    fn date(year: i32, month: Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).unwrap()
    }

    fn sample_mail(subject: &str, from: &str) -> InboundMail {
        let pdt = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::June, 26).unwrap(),
            Time::from_hms(9, 15, 0).unwrap(),
        );
        InboundMail {
            id: Uuid::new_v4(),
            created: pdt,
            version: Uuid::new_v4(),
            uid_validity: 1,
            imap_uid: 10,
            from_address: Arc::from(from),
            subject: Arc::from(subject),
            received_at: pdt,
            body_text: Arc::from("body"),
            has_attachments: false,
            has_html_body: false,
            raw_html_body: None,
            in_reply_to: None,
            message_id: None,
            replied: false,
            done: false,
            archived: false,
            assigned_member_id: None,
        }
    }

    // ── parse_recipients ──────────────────────────────────────────────────────

    #[test]
    fn parse_recipients_multiple() {
        let entries = vec![make_entry("digest_recipients", "a@x.de, b@y.de")];
        assert_eq!(parse_recipients(&entries), vec!["a@x.de", "b@y.de"]);
    }

    #[test]
    fn parse_recipients_empty_string() {
        let entries = vec![make_entry("digest_recipients", "")];
        assert!(parse_recipients(&entries).is_empty());
    }

    #[test]
    fn parse_recipients_whitespace_and_trailing_comma() {
        let entries = vec![make_entry("digest_recipients", "  a@x.de , , b@y.de ,")];
        assert_eq!(parse_recipients(&entries), vec!["a@x.de", "b@y.de"]);
    }

    #[test]
    fn parse_recipients_missing_key() {
        let entries = vec![make_entry("other_key", "x")];
        assert!(parse_recipients(&entries).is_empty());
    }

    // ── parse_send_time ───────────────────────────────────────────────────────

    #[test]
    fn parse_send_time_valid() {
        let entries = vec![make_entry("digest_send_time", "08:30")];
        assert_eq!(parse_send_time(&entries), Some((8, 30)));
    }

    #[test]
    fn parse_send_time_single_digit() {
        let entries = vec![make_entry("digest_send_time", "8:5")];
        assert_eq!(parse_send_time(&entries), Some((8, 5)));
    }

    #[test]
    fn parse_send_time_empty() {
        let entries = vec![make_entry("digest_send_time", "")];
        assert_eq!(parse_send_time(&entries), None);
    }

    #[test]
    fn parse_send_time_garbage() {
        let entries = vec![make_entry("digest_send_time", "abc")];
        assert_eq!(parse_send_time(&entries), None);
    }

    #[test]
    fn parse_send_time_hour_out_of_range() {
        let entries = vec![make_entry("digest_send_time", "25:00")];
        assert_eq!(parse_send_time(&entries), None);
    }

    #[test]
    fn parse_send_time_minute_out_of_range() {
        let entries = vec![make_entry("digest_send_time", "08:99")];
        assert_eq!(parse_send_time(&entries), None);
    }

    #[test]
    fn parse_send_time_missing_key() {
        let entries = vec![make_entry("other", "08:00")];
        assert_eq!(parse_send_time(&entries), None);
    }

    // ── is_due ────────────────────────────────────────────────────────────────

    #[test]
    fn is_due_already_sent_today_false() {
        let now = dt(2026, Month::June, 26, 10, 0);
        let today = date(2026, Month::June, 26);
        assert!(!is_due(now, (8, 0), Some(today)));
    }

    #[test]
    fn is_due_never_sent_before_time_false() {
        let now = dt(2026, Month::June, 26, 7, 30);
        assert!(!is_due(now, (8, 0), None));
    }

    #[test]
    fn is_due_never_sent_after_time_true_catchup() {
        // Catch-up D-01: nie gesendet, Uhrzeit überschritten ⇒ fällig.
        let now = dt(2026, Month::June, 26, 9, 0);
        assert!(is_due(now, (8, 0), None));
    }

    #[test]
    fn is_due_yesterday_sent_after_time_true() {
        let now = dt(2026, Month::June, 26, 8, 0);
        let yesterday = date(2026, Month::June, 25);
        assert!(is_due(now, (8, 0), Some(yesterday)));
    }

    #[test]
    fn is_due_yesterday_sent_before_time_false() {
        let now = dt(2026, Month::June, 26, 7, 59);
        let yesterday = date(2026, Month::June, 25);
        assert!(!is_due(now, (8, 0), Some(yesterday)));
    }

    #[test]
    fn is_due_exactly_at_send_time_true() {
        let now = dt(2026, Month::June, 26, 8, 0);
        assert!(is_due(now, (8, 0), None));
    }

    // ── build_digest_subject ──────────────────────────────────────────────────

    #[test]
    fn build_digest_subject_plural() {
        assert_eq!(build_digest_subject(5), "Posteingang: 5 offene Mails");
    }

    #[test]
    fn build_digest_subject_single_uses_singular_noun() {
        // WR-02: Singular muss grammatisch korrekt "offene Mail" lauten, nicht "offene Mails".
        assert_eq!(build_digest_subject(1), "Posteingang: 1 offene Mail");
    }

    #[test]
    fn build_digest_subject_zero_uses_plural_noun() {
        assert_eq!(build_digest_subject(0), "Posteingang: 0 offene Mails");
    }

    // ── build_digest_body ─────────────────────────────────────────────────────

    #[test]
    fn build_digest_body_contains_each_mail_and_link() {
        let m1 = sample_mail("Beitrittsantrag", "neu@example.de");
        let m2 = sample_mail("Rückfrage", "alt@example.de");
        let mails: Vec<&InboundMail> = vec![&m1, &m2];
        let body = build_digest_body(&mails, "https://app.example.de/inbox");

        assert!(body.contains("Beitrittsantrag"));
        assert!(body.contains("neu@example.de"));
        assert!(body.contains("Rückfrage"));
        assert!(body.contains("alt@example.de"));
        assert!(body.contains("https://app.example.de/inbox"));
        assert!(!body.is_empty());
    }

    #[test]
    fn build_digest_body_preserves_order_newest_first() {
        let m1 = sample_mail("Neueste", "a@x.de");
        let m2 = sample_mail("Aelteste", "b@y.de");
        let mails: Vec<&InboundMail> = vec![&m1, &m2];
        let body = build_digest_body(&mails, "http://localhost:3000/inbox");

        let pos_new = body.find("Neueste").unwrap();
        let pos_old = body.find("Aelteste").unwrap();
        assert!(pos_new < pos_old, "neueste Mail muss zuerst gelistet sein");
    }
}
