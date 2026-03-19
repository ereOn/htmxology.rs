---
name: release
description: Prepare and publish a new release to crates.io. Use when the user wants to release, publish, bump version, or cut a new version.
disable-model-invocation: true
argument-hint: [version]
---

# Release Workflow

Prepare and publish a new release for htmxology.

## Step 0: Check for upstream changes

```bash
git fetch origin
git log --oneline origin/main ^main -5
```

If there are upstream commits not yet in local main, **STOP** and ask the user how to proceed (rebase, merge, or continue).

## Step 1: Determine version

If a version was provided as `$ARGUMENTS`, use that. Otherwise:

1. Read the current version from `Cargo.toml` (`workspace.package.version`)
2. Ask the user what the next version should be (patch, minor, or major bump)

**Always confirm the version with the user before proceeding.**

## Step 2: Update version numbers

Update **both** version fields in the root `Cargo.toml`:

```toml
[workspace.package]
version = "<new-version>"

[workspace.dependencies]
htmxology-macros = { path = "./htmxology-macros", version = "<new-version>" }
```

## Step 3: Update CHANGELOG.md

Move all entries from `## [Unreleased]` to a new version section with today's date:

```markdown
## [Unreleased]

(empty - ready for next release)

## [<new-version>] - <today's date YYYY-MM-DD>

### Added
- (items moved from Unreleased)

### Changed
- (items moved from Unreleased)

### Fixed
- (items moved from Unreleased)
```

Remove empty subsections. Use today's date in ISO format.

## Step 4: Run all checks

```bash
just check
```

This runs build, license checks, format checks, and all tests. If anything fails, **stop and fix before continuing**.

## Step 5: Commit and tag

```bash
git add Cargo.toml CHANGELOG.md Cargo.lock
git commit -m "Release v<new-version>"
git tag -a v<new-version> -m "Release v<new-version>"
```

## Step 6: Publish to crates.io

Ask the user for confirmation before publishing. Then:

```bash
just publish
```

This publishes `htmxology-macros` first, then `htmxology`.

## Step 7: Push to GitHub

Ask the user for confirmation before pushing. Then:

```bash
git push origin main
git push origin v<new-version>
```
