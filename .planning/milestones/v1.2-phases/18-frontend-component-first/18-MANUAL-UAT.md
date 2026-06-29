---
phase: 18
artifact: manual-uat
status: passed
created: 2026-06-07
signed_off: 2026-06-07
signed_off_by: Simon Goller (Vorstand)
---

# Phase 18 — Manual UAT (Browser Walk-Through)

> Browser-Test-Anleitung fuer die Phase-18-Integration. Auszufuehren als Vorstand
> (Admin-Privilege) nach Plan 07 Task 1 (Code-Integration komplett).

## Voraussetzungen

1. Backend laufen lassen: `cargo run --bin genossi`
2. Frontend Tailwind-Watch + dev-server: `cd genossi-frontend && npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch & dx serve --hot-reload`
3. Im Browser einloggen als **Vorstand** (Admin-Privilege).
4. Mindestens 2 aktive Members und 1 gekuendigter Member in der DB (`exit_date IS NULL` bzw. `NOT NULL`).

## UAT-Szenario 1: Kuendigung

1. Navigiere zu `/members/{id}` eines aktiven Members (`exit_date IS NULL`).
2. **Verifiziere**: Button „Mitgliedschaft anpassen" ist sichtbar.
3. Klick Button → Modal oeffnet mit Sub-Choice (4 Karten: Kuendigung, Teil-Rueckgabe, Uebertrag, Aufstocken).
4. Klick „Kuendigung" → Sub-View oeffnet mit Titel „Kuendigung" (rote Heading), Datum-Input **vorbelegt mit heute** (SC-2), Vorschau live nach Selektion.
5. Datum-Input setzen auf **2026-06-15** (im aktuellen GJ).
6. **Verifiziere Vorschau**: `"{Vorname Nachname}: {X} Anteile (unveraendert) · Stichtag: 31.12.2026 (H1) · Auszahlung in Phase FY2026"`.
7. Datum-Input auf **2026-08-15** → Vorschau-Stichtag wechselt auf `31.12.2027 (H2) · Phase FY2027`.
8. Datum-Input auf **2024-01-01** (out-of-range) → Input-Border rot + Error-Text „Datum liegt ausserhalb...", Submit disabled, Vorschau verschwindet.
9. Datum zurueck auf 2026-06-15. Klick roter Submit „Kuendigung ausloesen".
10. **Verifiziere**: Modal schliesst, gruener Toast „Kuendigung wurde ausgeloest." (oder generischer Success-Toast), Member-Detail rerendert mit neuem `exit_date`.

## UAT-Szenario 2: Teil-Rueckgabe (inkl. Auto-Anlegen-Phase)

1. Anderer aktiver Member mit `current_shares >= 3`. Button klicken.
2. „Teil-Rueckgabe" waehlen.
3. Datum 2026-06-15, Anteile **1** (von z.B. 5).
4. **Verifiziere Vorschau**: `"{Name}: 5 → 4 Anteile (nach Auszahlung) · Stichtag: 31.12.2026 · Phase FY2026"`.
5. Anteile auf **5** (= current_shares) → Inline-Fehler „Du kannst nicht mehr Anteile zurueckgeben als das Mitglied besitzt. Fuer Voll-Rueckgabe nutze „Kuendigung"." — Submit disabled, Vorschau leer.
6. Anteile zurueck auf 2. Submit klicken.
7. **Verifiziere**: Modal schliesst, gruener Toast „Teil-Rueckgabe wurde eingetragen." (oder Auto-Create-Variante).
8. **Optional**: Wechsel auf `/repayment-phases` → neue Phase FY2026 mit Status `Preparation` und Entry fuer dieses Mitglied muss existieren.

## UAT-Szenario 3: Uebertrag (Teil + Voll inkl. Voll-Warnung)

1. Aktive Members mit z.B. 5 Anteilen (Source-Member A). Button klicken → „Uebertrag".
2. Datum 2026-06-15, Anteile 2.
3. **Verifiziere**: Empfaenger-Search erscheint nach kurz „Laedt..." Spinner. Gekuendigte Members NICHT in Liste.
4. Empfaenger B (3 Anteile) waehlen.
5. **Verifiziere Vorschau**: `"A {Name}: 5 → 3 Anteile · B {Name}: 3 → 5 Anteile · Datum: 15.06.2026"`. KEINE Voll-Uebertrag-Warnung.
6. Anteile auf **5** (= A.current_shares) → Vorschau: `"A: 5 → 0 Anteile · ..."` + orange-fette Warnung `"⚠ Voll-Uebertrag — {A Name} tritt am 15.06.2026 aus"`.
7. Anteile zurueck auf 2. Empfaenger auf den Source-Member selbst → Inline-Fehler „Empfaenger muss ein anderes Mitglied sein.", Submit disabled.
8. Empfaenger zurueck auf B. Submit „Uebertrag ausfuehren".
9. **Verifiziere**: Modal schliesst, gruener Toast „Uebertrag wurde ausgefuehrt.", A.current_shares=3, B.current_shares=5 (beim naechsten Page-Besuch).

## UAT-Szenario 4: Aufstockung

1. Aktiver Member, Button → „Aufstocken".
2. Datum 2026-06-15, Anteile 3.
3. **Verifiziere Vorschau**: `"{Name}: 5 → 8 Anteile · Datum: 15.06.2026"`.
4. Submit „Anteile aufstocken".
5. **Verifiziere**: Modal schliesst, gruener Toast „Aufstockung wurde eingetragen.", current_shares=8.

## UAT-Szenario 5: Negative-Pfade

1. **Submit ohne Datum**: Datum leer setzen → Submit MUSS disabled bleiben in jeder Sub-View.
2. **Out-of-range Datum**: 2024-01-01 in Cancel-Sub-View → Border rot, Submit disabled.
3. **Back-Navigation**: Sub-View → „← Zurueck zur Auswahl" → Sub-Choice mit reseted Feldern.
4. **Abbrechen**: „Abbrechen"-Button schliesst Modal ohne Submit (kein Toast, kein Refresh).
5. **Server-Error** (optional): Falls 409 (z.B. bereits-cancelled): ErrorAlert INNERHALB Modal (NICHT Toast), Modal bleibt offen, Submit enabled fuer Retry.

## UAT-Szenario 6: i18n DE/EN Switch (falls Locale-Switcher verfuegbar)

1. Locale auf EN → Button „Adjust membership", Modal-Title „Adjust membership", Sub-Choice „Cancellation / Partial repayment / Transfer / Increase shares".
2. Vorschau-Text in EN: `"{name}: {X} shares (unchanged) · Effective: 2026-12-31 (H1) · Payout in phase FY2026"`.

## Sign-Off

- [ ] Szenario 1 (Kuendigung) PASS
- [ ] Szenario 2 (Teil-Rueckgabe) PASS
- [ ] Szenario 3 (Uebertrag inkl. Voll-Warnung) PASS
- [ ] Szenario 4 (Aufstockung) PASS
- [ ] Szenario 5 (Negative-Pfade — alle 5) PASS
- [ ] Szenario 6 (i18n DE/EN) PASS (sofern Switcher verfuegbar)
- [ ] Browser-Console: keine JavaScript-Errors / Dioxus-Warnings
- [ ] Network-Tab: alle POST/GET liefern 2xx (oder erwartete 4xx fuer Negative-Pfade)

**Tester:** _______________________
**Datum:** _______________________
**Ergebnis:** ☐ PASS ☐ FAIL (Issues unten dokumentieren)

### Gefundene Issues (falls FAIL)

| # | Szenario | Erwartet | Beobachtet | Hotfix-Plan |
|---|----------|----------|------------|-------------|
|   |          |          |            |             |
