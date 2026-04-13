## Context

Genossi hat zwei bestehende Systeme, die bisher unverbunden sind:

1. **PDF-Generierung**: Typst-Templates werden mit Member-Daten gerendert und als Download zurückgegeben (`POST /api/templates/render/{*path}/{member_id}`)
2. **MemberDocuments**: Dokumente werden per Multipart-Upload hochgeladen und auf dem Filesystem gespeichert (`POST /members/{member_id}/documents`)

Aktuell muss man ein Dokument erst per Browser herunterladen und dann manuell wieder hochladen, um es beim Mitglied zu hinterlegen. Außerdem ersetzt das Singleton-Verhalten bei Upload vorhandene Dokumente automatisch per soft-delete.

## Goals / Non-Goals

**Goals:**
- Neuer Endpoint der ein Template rendert und das Ergebnis direkt als MemberDocument ablegt
- Feste Zuordnung Template-Name → DocumentType
- Singleton-Dokumente blockieren statt ersetzen (sowohl bei Upload als auch bei Generierung)
- Frontend-Button auf der Member-Detail-Seite

**Non-Goals:**
- Automatische Generierung bei bestimmten Events (z.B. bei Beitritt)
- Batch-Generierung für mehrere Mitglieder gleichzeitig
- Neue DocumentTypes hinzufügen (nur bestehendes Mapping nutzen)

## Decisions

### 1. Neuer Endpoint statt Erweiterung des bestehenden Render-Endpoints

**Entscheidung**: Neuer Endpoint `POST /api/members/{member_id}/documents/generate/{document_type}`

**Alternativen**:
- Erweiterung des bestehenden Render-Endpoints mit Query-Parameter `?store=true` — vermischt zwei Concerns (Preview vs. persistente Ablage)
- Endpoint auf Template-Seite (`POST /api/templates/render/.../store`) — semantisch gehört das Dokument zum Member, nicht zum Template

**Begründung**: Der Endpoint gehört zur Member-Document-Domäne, nicht zur Template-Domäne. Die Generierung ist ein Seiteneffekt der Dokumentablage.

### 2. Template-zu-DocumentType-Mapping als Lookup-Map

**Entscheidung**: Statische Map im Code:
```
join_confirmation.typ → JoinConfirmation
join_declaration.typ  → JoinDeclaration
```

**Begründung**: Einfach, erweiterbar durch Code-Änderung. Keine Datenbank oder Konfiguration nötig. Neue Mappings kommen mit neuen Templates.

### 3. Singleton-Blockierung im Service-Layer

**Entscheidung**: Die Prüfung "existiert bereits ein Dokument dieses Typs?" wird im MemberDocumentService durchgeführt, nicht im neuen Generierungs-Endpoint. Das betrifft sowohl den bestehenden Upload als auch die neue Generierung.

**Begründung**: Konsistentes Verhalten — egal wie das Dokument angelegt wird, die Regel ist dieselbe. Single Responsibility: der Service kennt die Business-Regeln.

### 4. Generierungs-Logik im MemberDocumentService

**Entscheidung**: Neue Methode `generate` im `MemberDocumentService`-Trait, die den `PdfGenerator` und `TemplateStorage` nutzt.

**Alternative**: Separater Service — würde aber nur MemberDocumentService und PdfGenerator zusammenkleben, ohne eigene Logik.

**Begründung**: Die Generierung ist konzeptionell eine spezielle Form des Uploads. Der MemberDocumentService hat bereits Zugriff auf die Document-Storage-Infrastruktur.

## Risks / Trade-offs

- **[Breaking Change]** Bestehende Singleton-Uploads schlagen fehl statt zu ersetzen → Nutzer müssen erst löschen, dann neu hochladen. **Mitigation**: Frontend zeigt klare Fehlermeldung und der Löschen-Button ist direkt daneben.
- **[Kopplung]** MemberDocumentService bekommt Abhängigkeit auf PdfGenerator und TemplateStorage → erhöht die Komplexität des Services. **Mitigation**: Die Methode ist dünn — sie delegiert nur an bestehende Komponenten.
