## Context

Genossi nutzt eine geschichtete Architektur (DAO → Service → REST). Aktuell gibt es bereits Feature-Flags (`mock_auth`, `oidc`) die per `#[cfg(...)]` Teile des Codes bedingt kompilieren. Fuer den Testdaten-Endpunkt nutzen wir stattdessen `#[cfg(debug_assertions)]`, da es sich um eine reine Entwicklungshilfe handelt, die automatisch bei `cargo build` (ohne `--release`) verfuegbar ist.

## Goals / Non-Goals

**Goals:**
- Schnelle Verfuegbarkeit von Member-Testdaten beim lokalen Entwickeln
- Null Overhead im Release-Build (Code wird nicht kompiliert)
- Einfacher Aufruf per `curl` oder Browser-DevTools

**Non-Goals:**
- Kein generisches Test-Framework - nur Member-Testdaten
- Keine konfigurierbaren Parameter (Anzahl, Felder etc.)
- Kein Seeding beim Server-Start - nur on-demand per Endpunkt

## Decisions

### `cfg!(debug_assertions)` statt Cargo Feature

Der Endpunkt wird mit `#[cfg(debug_assertions)]` bedingt kompiliert. Das ist automatisch bei Debug-Builds aktiv und in Release-Builds entfernt, ohne dass ein Feature-Flag konfiguriert werden muss.

**Alternative**: Ein eigenes Cargo Feature `testdata`. Verworfen, weil es manuell aktiviert werden muss und versehentlich in Release-Builds landen koennte.

### Direkt im REST-Layer, kein eigener Service

Der Endpunkt nutzt den bestehenden `MemberService::create()` direkt. Die Testdaten-Definition lebt als statische Daten im REST-Modul `dev.rs`.

**Alternative**: Eigener TestDataService im Service-Layer. Verworfen, weil das Overengineering waere fuer einen Dev-only Endpunkt.

### Idempotenz per Existenz-Check

Vor dem Erzeugen wird `MemberService::get_all()` aufgerufen. Wenn bereits Members existieren, wird nichts erzeugt und 200 zurueckgegeben.

### Kein Auth-Check

Der Endpunkt umgeht die Auth-Middleware, da er nur in Debug-Builds existiert. Er wird ausserhalb der authentifizierten API-Routen registriert.

## Risks / Trade-offs

- **Risk**: Jemand baut versehentlich mit `debug_assertions` in Produktion → **Mitigation**: Standard-Rust-Toolchain setzt `debug_assertions` nur bei Debug-Builds. CI/CD sollte immer `--release` nutzen.
- **Trade-off**: Keine Konfigurierbarkeit der Testdaten → Akzeptabel, da feste Testdaten fuer den Anwendungsfall ausreichen.
