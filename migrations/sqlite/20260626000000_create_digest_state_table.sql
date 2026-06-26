-- Phase 20 (D-03): Dedizierte Singleton-State-Tabelle für den Inbox-Digest-Worker.
-- Bewusst NICHT der Config-KV-Store (config_entries), sondern eine eigene Tabelle:
-- sie hält den internen Worker-Zustand (letztes Versanddatum), nicht User-Config.
-- Einziger Key: 'last_sent_date', Value: ISO-Datum 'YYYY-MM-DD' (max. 1 Row).
CREATE TABLE IF NOT EXISTS digest_state (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
