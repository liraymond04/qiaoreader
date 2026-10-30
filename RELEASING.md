# Releasing QiaoReader

QiaoReader uses Semantic Versioning, a Keep a Changelog history, and tag-driven
GitHub releases.

## Branch and merge flow

`main` is the integration branch. Work on short-lived branches from `main`:

```sh
git switch -c feat/reference-panel
git config push.default current   # optional: plain `git push` follows the branch
git push -u origin HEAD
```

Open a PR (draft while it is WIP), let CI pass, and squash-merge once the task
is verified. **Merge when the work is done, not when bumping a version.**

### Do not keep a long-lived `feat` branch

A permanent `feat` (or `develop`) branch can be made to work, but it becomes a
second integration branch:

- every change lands there first and then has to be promoted to `main`;
- conflicts are deferred, not removed;
- `main` drifts from the code that is actually being tested;
- `feat` collides with the `feat/<topic>` naming convention.

For remote backup and coordination, push each topic branch to `origin` and use a
draft PR. Every commit is saved remotely without a shared catch-all branch. If
the project later needs a release train with multiple maintainers, add a
`develop` (not `feat`) branch deliberately, protect it, and define promotion to
`main`. Until then, keep `main` plus short-lived branches.

### `main` protection

`main` is protected with these rules:

- A pull request is required; direct pushes are blocked, including for admins.
- Both required status checks must pass: `Web (Svelte + TypeScript)` and
  `Rust (fmt, clippy, tests)`.
- The branch must be up to date with `main`; only linear history is allowed
  (squash or rebase), and conversations must be resolved.
- Force pushes and branch deletion are blocked.
- Tags matching `v*` cannot be deleted or moved.

Required approvals are currently **0**, so a solo maintainer can merge their own
PR. Raise this to 1 when a second regular maintainer joins. Release commits also
go through a `chore/release-vX.Y.Z` PR; tags are created only after that PR is
merged.

## Versioning

Versions follow [SemVer 2.0.0](https://semver.org/spec/v2.0.0.html), starting at
`0.1.0`:

- Before 1.0, bump **MINOR** for a feature or milestone and **PATCH** for fixes.
  Breaking changes may occur in a MINOR release but must be called out in the
  changelog.
- `1.0.0` is reserved for a stable reading loop and mobile support.
- Do not bump the version per commit. A version bump is a release event.
- Tags are annotated `vX.Y.Z` and always created from `main`.

The version lives in three files and must stay in sync:

- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`

```sh
bun run version:check             # verify the three files agree
bun run version:check --tag v0.1.0
bun run version:bump 0.2.0        # update all three and cut the changelog section
```

The bump script moves `CHANGELOG.md` entries from `[Unreleased]` into the new
version section, dated today. It does not commit or tag; review the diff first.
`Cargo.lock` also records the package version: run `cargo check` after the bump
so the lockfile is staged with the release commit.

## Changelog

Every user-visible change adds a line under `## [Unreleased]` in
[`CHANGELOG.md`](CHANGELOG.md), grouped as Added, Changed, Fixed, or Removed.
Internal-only changes (CI refactors, formatting) may skip it. The version bump
script moves the section into the release entry.

## Release checklist

1. `main` is green and `bun run verify` passes locally.
2. Create a release branch: `git switch -c chore/release-vX.Y.Z`.
3. Run `bun run version:bump X.Y.Z`.
4. Review the generated `CHANGELOG.md` section and the version diff; run
   `cargo check` to refresh `Cargo.lock`.
5. Commit `chore(release): vX.Y.Z`, push the branch, open a PR, and squash-merge
   it after CI passes.
6. After the merge, update `main` and tag the merge commit:

   ```sh
   git switch main
   git pull --ff-only
   git tag -a vX.Y.Z -m "QiaoReader vX.Y.Z"
   git push origin vX.Y.Z
   ```

7. The `Release` workflow validates that the tag is on `main`, that the tag
   matches all three version fields, and builds the artifacts.
8. The workflow publishes the release automatically. Verify the assets and the
   changelog-derived release notes; do not move the tag afterward.

Tags are the only release trigger. A normal push or merge to `main` runs `CI`
but does not publish a release. If you want builds on every merge, publish them
as workflow artifacts or an `edge` prerelease, not as stable releases.

## What the release workflow builds

Triggered by a `v*` tag (or manually with an existing tag):

- **Desktop** on Ubuntu, macOS (arm64 and x64), and Windows. Linux produces
  AppImage/deb/rpm; macOS produces `.app`/`.dmg`; Windows produces `.msi`/NSIS.
- **Android** when the signing secrets are configured: `tauri-action` runs
  `tauri android build` and uploads the `.apk`/`.aab` to the same release.
  The Android project is committed; without signing secrets the job is skipped
  so a desktop release still succeeds.

The workflow publishes the release automatically. The release body is the
matching `## [X.Y.Z]` section from `CHANGELOG.md`; write that section before
tagging. Desktop builds are unsigned unless signing secrets are configured;
macOS will show Gatekeeper warnings for unsigned builds.

## Android signing

Android release builds use the committed project under `src-tauri/gen/android`
and an upload keystore. The Gradle release signing block reads
`keystore.properties` from the Android project root, following
[Tauri's Android signing guide](https://v2.tauri.app/distribute/sign/android/).

1. Add these repository secrets:
   - `ANDROID_KEY_BASE64` — the keystore file, base64-encoded
   - `ANDROID_KEY_ALIAS` — the key alias
   - `ANDROID_KEY_PASSWORD` — the keystore/key password
2. The workflow writes `src-tauri/gen/android/keystore.properties` in the runner
   and decodes the keystore into `$RUNNER_TEMP`; nothing secret is committed.
3. The workflow installs compileSdk 37 and NDK 28.2.13676358. Keep those aligned
   with the generated Gradle project.

The release Android job is skipped unless the signing secrets are present, so a
desktop release still succeeds without them. Debug builds need no secrets:

```sh
bun tauri android build --debug --apk --target aarch64
```

The manual **Android debug build** workflow runs that command on CI and uploads
the arm64 APK as an artifact.

## Desktop signing

Optional for now. Add the usual Tauri secrets if distributing outside GitHub:
- macOS code signing/notarisation (`APPLE_CERTIFICATE`, `APPLE_SIGNING_IDENTITY`,
  `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`).
- Windows Authenticode signing.

Unsigned Linux builds are normal for AppImage/deb/rpm distribution.
