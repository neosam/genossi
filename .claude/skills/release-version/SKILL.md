---
name: release-version
description: >
  Release a new version of genossi. Generates release notes from changes since the last tag,
  runs cli-update-version.sh with the release notes as tag message, and reports the
  new version number. Use when the user says "release", "neue Version", "Version releasen",
  or "/release-version".
---

# Release Version Skill

Release a new genossi version with release notes as tag message.

## Steps

### 1. Find the Last Tag and Get Changes

Use jj to find the latest tag and list changes since then:

```bash
LAST_TAG=$(jj tag list | tail -1 | awk '{print $1}' | tr -d ':')
jj log -r "tags(exact:\"$LAST_TAG\")..@" --no-graph -T 'description.first_line() ++ "\n"'
```

### 2. Generate Release Notes

From the commit messages, create structured release notes. Categorize changes into sections
like Features, Bug Fixes, Improvements, etc. Only include sections that have entries.
Use bullet points for each change. Example format:

```
Features:
- Session management UI
- User-friendly error display

Bug Fixes:
- Fix upload path traversal

Improvements:
- CORS allowlist and security headers
```

### 3. Run the Release Script

Run the release script from the project root with the release notes as tag message:

```bash
./cli-update-version.sh -m "<release notes>"
```

IMPORTANT: Wait for the script to complete. The release notes must always be provided via
the `-m` flag to avoid an interactive editor opening.

### 4. Report the Result

Tell the user the new version number. The script outputs `Released version: X.Y.Z` at the end.
Report this version to the user.
