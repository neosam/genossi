---
quick_id: 260603-hw7
date: 2026-06-03
status: complete
one_liner: module.nix bekommt applicationTitle/isProd/envShortDescription; config.json wird über builtins.toJSON erzeugt — Stable kann „Genossi DEV" jetzt abschalten oder umschreiben
---

# Quick Task 260603-hw7 — Summary

## Was geliefert wurde

- **module.nix** — drei neue Submodule-Options im `services.genossi.<name>`-Block:
  - `applicationTitle` (str, default `"Genossi"`) — Titel in Browser-Tab und Menü-Header
  - `isProd` (bool, default `false`) — wenn `true`, blendet das Menü das Env-Suffix aus (kein „DEV", „STAGING", …)
  - `envShortDescription` (str, default `"DEV"`) — Label neben „Genossi", solange `isProd = false`
- **module.nix** — `environment.etc."genossi-${name}/config.json".text` umgestellt von hardcodiertem Heredoc auf `builtins.toJSON { backend = …; application_title = …; is_prod = …; env_short_description = …; }`. Vorteile: korrekte JSON-Escapes, alle Frontend-`Config`-Felder synchron mitgeneriert, keine manuelle Quoting-Pflege mehr.
- **example-config.nix** — Production- und Staging-Beispiel zeigen die neuen Options (`isProd = true` bzw. `envShortDescription = "STAGING"`).

## Wie Stable das DEV-Label loswird

```nix
services.genossi.prod = {
  enable = true;
  domain = "genossi.example.org";
  isProd = true;                # → Menü zeigt nur noch „Genossi", kein Suffix
  # ODER, wenn weiter ein Label gewünscht ist:
  # envShortDescription = "PROD";
};
```

Default-Verhalten bleibt unverändert (`is_prod = false`, `env_short_description = "DEV"`) — niemand sieht durch das Upgrade ein anderes Label, ohne aktiv die Option zu setzen.

## Verifikation

1. `nix-instantiate --parse module.nix` → OK.
2. `nix-instantiate --parse example-config.nix` → OK.
3. Modul-Auswertung über `lib.evalModules` mit stub-`environment.etc` (Submodule mit `text: lines`) ausgeführt:
   - **Defaults** (nur `enable=true; domain="test.example.com"`) liefern
     `{"backend":"https://test.example.com/api","application_title":"Genossi","is_prod":false,"env_short_description":"DEV"}`.
   - **Overrides** (`isProd=true; envShortDescription="STAGING"; applicationTitle="Genossi Test"`) liefern
     `{"backend":"https://test.example.com/api","application_title":"Genossi Test","is_prod":true,"env_short_description":"STAGING"}`.
   - JSON ist `builtins.fromJSON`-parsebar (Quoting korrekt).
4. Frontend liest die Felder bereits heute aus `/config.json` ein:
   - `genossi-frontend/src/state/config.rs` — `Config` mit serde-Defaults für die drei Felder.
   - `genossi-frontend/src/app.rs:21-29` — Browser-Tab-Titel.
   - `genossi-frontend/src/component/top_bar.rs:142-146` — Menü-Header `"Genossi"` + bedingtes `env_short_description`-Span.

   Es war keine Frontend-Änderung nötig; die Bug-Quelle war ausschliesslich die statische `config.json` aus dem NixOS-Modul.

## Out of scope

- Keine Frontend-Änderungen — die Felder existieren mit serde-Defaults bereits.
- Keine Migration alter installierter `/etc/genossi-*/config.json`-Dateien — die werden bei jedem `nixos-rebuild switch` neu generiert.
- Keine generische `extraConfig`-Attrset-Option (kann später nachgereicht werden, wenn weitere Frontend-Felder konfigurierbar werden sollen).
- `example-config.nix` enthält noch andere alte/aspirational Felder (`features`, `ssl`, `forceSSL`), die nicht im aktuellen module.nix existieren. Bewusst nicht angefasst — out of scope.

## Behavior-Equivalence-Garantie

Bestehende NixOS-Konfigs, die nichts setzen, bekommen weiterhin genau die alte sichtbare Wirkung:
- Menü zeigt „Genossi DEV" (Default `is_prod=false`, `env_short_description="DEV"`).
- Nur der JSON-String in `/etc/genossi-${name}/config.json` enthält jetzt drei zusätzliche Felder — alle mit Werten, die identisch zu den Frontend-Serde-Defaults sind.
