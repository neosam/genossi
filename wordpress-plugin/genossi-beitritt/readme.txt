=== Genossi Beitritt ===
Contributors: genossi
Tags: genossenschaft, beitritt, formular, membership
Requires at least: 5.2
Tested up to: 6.5
Requires PHP: 7.4
Stable tag: 1.0.0
License: GPLv2 or later
License URI: https://www.gnu.org/licenses/gpl-2.0.html

Beitrittsformular fuer Genossenschaften. Rendert ein Formular per Shortcode und sendet die Daten serverseitig an die Genossi-API.

== Description ==

Das Plugin stellt einen Shortcode `[genossi_beitritt]` bereit, der ein Beitrittsformular auf jeder WordPress-Seite oder jedem Beitrag rendert. Bei Absenden des Formulars werden die Daten serverseitig (PHP) an die Genossi-API gesendet. Weder die API-URL noch der API-Key werden dem Browser exponiert.

**Funktionen:**

* Beitrittsformular mit allen relevanten Feldern (Name, Adresse, E-Mail, Geschaeftsanteile)
* Serverseitiger API-Call (WordPress-Server -> Genossi-API)
* Pflichtfeld-Validierung (client- und serverseitig)
* Benutzerfreundliche Fehleranzeige
* CSRF-Schutz via WordPress-Nonce
* Minimales CSS, das sich in jedes Theme einfuegt

== Installation ==

1. Lade den Ordner `genossi-beitritt` in das Verzeichnis `/wp-content/plugins/` hoch.
2. Aktiviere das Plugin ueber das WordPress-Admin-Menue unter "Plugins".
3. Gehe zu "Einstellungen > Genossi Beitritt" und trage die API-URL und den API-Key ein.
4. Fuege den Shortcode `[genossi_beitritt]` auf einer Seite oder in einem Beitrag ein.

== Konfiguration ==

Unter "Einstellungen > Genossi Beitritt" muessen zwei Felder konfiguriert werden:

* **Genossi API-URL**: Die Basis-URL der Genossi-Instanz (z.B. `https://genossi.example.com`). Ohne abschliessenden Slash, ohne `/api/...`.
* **API-Key**: Der API-Key fuer den Zugriff auf den oeffentlichen Beitritts-Endpunkt. Wird im Genossi-Admin-Bereich generiert.

== Captcha-Integration ==

Das Plugin selbst implementiert kein Captcha. Es ist jedoch kompatibel mit gaengigen WordPress-Captcha-Plugins:

**Empfohlene Plugins:**

* **reCaptcha by BestWebSoft** - Fuegt Google reCAPTCHA zu WordPress-Formularen hinzu. Unterstuetzt Shortcode-basierte Formulare ueber Custom-Hooks.
* **hCaptcha for WordPress** - Datenschutzfreundliche Alternative zu reCAPTCHA.
* **Honeypot for Contact Form 7 / WPForms** - Unsichtbare Spam-Erkennung ohne Benutzerinteraktion.

**Hinweis:** Da das Plugin ein Standard-HTML-Formular mit POST verwendet, funktionieren Captcha-Plugins am besten, die sich in das WordPress `init`- oder `template_redirect`-Hook einklinken, oder die allgemeine Formular-Felder via JavaScript hinzufuegen.

== Changelog ==

= 1.0.0 =
* Erstveroeffentlichung
* Beitrittsformular mit Shortcode
* Settings-Seite fuer API-Konfiguration
* Serverseitiger API-Call an Genossi
