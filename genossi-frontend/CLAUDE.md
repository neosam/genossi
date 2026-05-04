# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Genossi is an inventory management web application built with Dioxus (Rust-based web framework). It manages products, persons, and permissions with a clean REST API backend integration.

## Essential Commands

### Development
```bash
# Start Tailwind CSS compiler (required for styling)
npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch

# Run development server with hot reload
dx serve --hot-reload

# Build the application
dx build

# Clean build artifacts
dx clean
```

### Code Quality
```bash
# Format code
cargo fmt

# Run clippy linter
cargo clippy

# Type check
cargo check

# Run tests
cargo test
```

## Architecture Overview

### Frontend Stack
- **Dioxus**: React-like framework for Rust
- **Tailwind CSS**: Utility-first CSS with custom zoom classes (scale-down-50, scale-down-75, scale-down-100)
- **Multi-language i18n**: Located in `src/i18n/` with keys in `mod.rs` and translations in `en.rs` and `de.rs` (corrected 2026-05-04 — only two locales exist; the previous mention of `cs.rs` was stale documentation, the file does not exist in this codebase)

### Key Architectural Patterns

1. **Component-Service-State Pattern**:
   - **Components** (`src/component/`): Reusable UI components using Dioxus RSX syntax
   - **Services** (`src/service/`): Business logic and API communication with coroutines
   - **State** (`src/state/`): Data structures and domain models
   - **Pages** (`src/page/`): Full page components that compose smaller components

2. **Component-First Principle** (IMPORTANT):
   - **Always use reusable components instead of inline HTML/RSX.** When building UI, check `src/component/` for existing components first. If none fits, create a new reusable component.
   - **Never duplicate UI logic across pages.** If two pages need similar UI (e.g., a subject input, a body textarea, a template selector), extract it into a shared component under `src/component/`.
   - **Pages compose components, they don't contain raw HTML.** Pages should read like a high-level description of the UI, delegating rendering details to components.
   - **Why:** Consistent styling across the app, single source of truth for behavior, and easier maintenance. Without this, pages that should look and behave identically will silently diverge in styling and logic.

2. **API Communication**:
   - `src/api.rs`: REST API client functions
   - `src/loader.rs`: Data loading utilities
   - `rest-types/`: Shared types between frontend and backend
   - Backend proxy configuration in `Dioxus.toml`

3. **Routing**: Defined in `src/router.rs` using Dioxus Router

### Critical Components

**WeekView** (`src/component/week_view.rs`):
- Core shift planning view with sticky time column
- Uses CSS `zoom` property for zoom functionality (not `transform: scale`)
- Implements horizontal scrolling for weekdays while keeping time column fixed

**i18n System**:
- All translations must be added to **both locales (En, De)** — these are the only two locales defined in `src/i18n/mod.rs` (the `Locale` enum has only `En` and `De` variants, and the only translation files are `de.rs` and `en.rs`).
- German translations previously had a bug where they used `Locale::En` instead of `Locale::De` (now fixed)
- Translation keys are defined in `src/i18n/mod.rs` enum `Key`
- **Note (corrected 2026-05-04):** older versions of this section claimed three locales `(En, De, Cs)` and instructed contributors to add keys to a `cs.rs`. That was stale documentation — `cs.rs` does not exist in this codebase and the `Locale` enum has no `Cs` variant. If a Czech locale is ever needed, it must first be added to the enum and a `cs.rs` file created with all existing keys translated.

**Billing Period** (`src/page/billing_period_details.rs`):
- Displays sales person values with translations for BALANCE, EXPECTED_HOURS, OVERALL
- Formats dates using `i18n.format_date()` 
- Rounds monetary values to 2 decimal places

### Common Issues & Solutions

1. **Zoom gaps in WeekView**: Use CSS `zoom` property, not `transform: scale`
2. **German translations not working**: Ensure using `Locale::De` not `Locale::En`
3. **WASM validation errors**: Run `dx clean` before rebuilding
4. **Time column width issues**: Adjust `min-w-*` classes in TimeView component

### Backend Configuration

The application expects a backend server running on `http://localhost:3000` with endpoints defined in `Dioxus.toml` proxy configuration.

### Development Notes

- The application serves on `http://localhost:8080` by default
- Tailwind CSS must be running in watch mode during development
- Custom Tailwind colors are defined in `tailwind.config.js` (missingColor, blockedColor)
- Print-specific styles are configured for shift plan printing
