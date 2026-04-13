=== Genossi Member Count ===
Contributors: genossi
Tags: genossenschaft, mitglieder, member count, shortcode
Requires at least: 5.2
Tested up to: 6.5
Requires PHP: 7.4
Stable tag: 1.0.0
License: GPLv2 or later
License URI: https://www.gnu.org/licenses/gpl-2.0.html

Zeigt die Anzahl aktiver Genossenschafts-Mitglieder per Shortcode an.

== Description ==

Das Plugin stellt einen Shortcode `[genossi_member_count]` bereit, der die aktuelle Mitgliederzahl der Genossenschaft auf jeder WordPress-Seite oder jedem Beitrag anzeigt. Die Zahl wird ueber die oeffentliche Genossi-API abgerufen und gecached.

**Funktionen:**

* Shortcode `[genossi_member_count]` zur Inline-Anzeige der Mitgliederzahl
* WordPress Transient Caching mit konfigurierbarer Dauer (Standard: 15 Minuten)
* Automatische Uebernahme der API-URL vom Genossi-Beitritt-Plugin (falls installiert)
* Eigene Settings-Seite fuer Standalone-Betrieb
* Kein API-Key erforderlich (oeffentlicher Endpunkt)

**Beispiel:**

`Unsere Genossenschaft hat aktuell [genossi_member_count] Mitglieder.`

== Installation ==

1. Lade den Ordner `genossi-member-count` in das Verzeichnis `/wp-content/plugins/` hoch.
2. Aktiviere das Plugin ueber das WordPress-Admin-Menue unter "Plugins".
3. Falls das Genossi-Beitritt-Plugin installiert ist, wird die API-URL automatisch uebernommen. Andernfalls: Gehe zu "Einstellungen > Genossi Member Count" und trage die API-URL ein.
4. Fuege den Shortcode `[genossi_member_count]` auf einer Seite oder in einem Beitrag ein.

== Konfiguration ==

Unter "Einstellungen > Genossi Member Count":

* **Genossi API-URL**: Wird automatisch vom Genossi-Beitritt-Plugin uebernommen, falls installiert. Andernfalls: Die Basis-URL der Genossi-Instanz (z.B. `https://genossi.example.com`). Ohne abschliessenden Slash, ohne `/api/...`.
* **Cache-Dauer**: Wie lange die Mitgliederzahl gecached wird (in Sekunden). Standard: 900 (15 Minuten).

**Voraussetzung:** In der Genossi-Konfiguration muss `public_stats_enabled` auf `true` gesetzt sein, damit der API-Endpunkt aktiv ist.

== Changelog ==

= 1.0.0 =
* Erstveroeffentlichung
* Shortcode fuer Mitgliederzahl
* Transient Caching
* URL-Fallback auf Genossi-Beitritt-Plugin
* Settings-Seite
