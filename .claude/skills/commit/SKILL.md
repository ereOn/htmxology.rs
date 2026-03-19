---
name: commit
description: Commit changes with proper formatting, changelog update, and checks. Use when the user wants to commit, save progress, or record changes.
disable-model-invocation: true
argument-hint: [commit message]
---

# Commit Workflow

Commit changes following the project's conventions.

## Step 1: Check for upstream changes

```bash
git fetch origin
git log --oneline origin/main ^main -5
```

If there are upstream commits not yet in local main, **STOP** and ask the user how to proceed.

## Step 2: Review changes

Run `git status` and `git diff` to understand what's being committed. Summarize the changes for the user.

## Step 3: Update CHANGELOG.md

Read `CHANGELOG.md` and add entries under `## [Unreleased]` describing the changes. Use the appropriate subsection:

- **Added** — new features or capabilities
- **Changed** — changes to existing functionality
- **Fixed** — bug fixes
- **Removed** — removed features or deprecated items

Keep entries concise (one line each). Do not add empty subsections.

## Step 4: Format code

```bash
cargo fmt --all
```

## Step 5: Run checks

```bash
just check
```

This runs build, license checks, format checks, and all tests. If anything fails, **stop and fix before continuing**.

## Step 6: Commit

Stage all relevant files (including `CHANGELOG.md`) and commit. If a message was provided as `$ARGUMENTS`, use that. Otherwise, draft a concise commit message based on the changes and confirm with the user.

Do not stage files that likely contain secrets (`.env`, credentials, etc.).
