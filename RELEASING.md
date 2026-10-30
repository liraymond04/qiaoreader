# Releasing QiaoReader

QiaoReader uses Semantic Versioning, a Keep a Changelog history, and tag-driven
GitHub releases.

## Branch and merge flow

`main` is the trunk. Work on short-lived branches from `main`:

```sh
git switch -c feat/reference-panel
# ...commit...
git push -u origin feat/reference-panel
```

Open a PR, let `CI` pass, and squash-merge once the task is verified. **Merge
when the work is done, not when bumping a version.** A long-lived branch that
waits for a release makes integration harder and hides regressions. Direct
pushes to `main` are reserved for release commits and emergencies.

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

## Changelog

Every user-visible change adds a line under `## [Unreleased]` in
[`CHANGELOG.md`](CHANGELOG.md), grouped as Added, Changed, Fixed, or Removed.
Internal-only changes (CI refactors, formatting) may skip it. The version bump
script moves the section into the release entry.

## Release checklist

1. `main` is green and `bun run verify` passes locally.
2. Decide the version and run `bun run version:bump X.Y.Z`.
3. Review the generated `CHANGELOG.md` section and the version diff.
4. Commit `chore(release): vX.Y.Z`.
5. Create and push an annotated tag:

   ```sh
   git tag -a vX.Y.Z -m "QiaoReader vX.Y.Z"
   git push origin main --tags
   ```

6. The `Release` workflow validates that the tag is on `main`, that the tag
   matches all three version fields, and builds the artifacts.
7. Open the draft GitHub release, verify the assets, and publish it.

Tags are the only release trigger. A normal push or merge to `main` runs `CI`
but does not publish a release. If you want builds on every merge, publish them
as workflow artifacts or an `edge` prerelease, not as stable releases.

## What the release workflow builds

Triggered by a `v*` tag (or manually with an existing tag):

- **Desktop** on Ubuntu, macOS (arm64 and x64), and Windows. Linux produces
  AppImage/deb/rpm; macOS produces `.app`/`.dmg`; Windows produces `.msi`/NSIS.
- **Android** when `src-tauri/gen/android` exists: `tauri-action` runs
  `tauri android build` and uploads the `.apk`/`.aab` to the same draft release.
  Until the Android project is initialised in Milestone 4, this job is skipped.

The workflow creates a **draft** release so assets can be checked before
publishing. Desktop builds are unsigned unless signing secrets are configured;
macOS will show Gatekeeper warnings for unsigned builds.

## Android signing

Android release builds need the generated project and an upload keystore:

1. Initialise and commit the Android project (`bun tauri android init`), then
   configure the release signing block in
   `src-tauri/gen/android/app/build.gradle.kts` as described in
   [Tauri's Android signing guide](https://v2.tauri.app/distribute/sign/android/).
2. Add these repository secrets:
   - `ANDROID_KEY_BASE64` — the keystore file, base64-encoded
   - `ANDROID_KEY_ALIAS` — the key alias
   - `ANDROID_KEY_PASSWORD` — the keystore/key password
3. The workflow writes `src-tauri/gen/android/keystore.properties` in the runner
   and decodes the keystore into `$RUNNER_TEMP`; nothing secret is committed.
4. Keep the NDK version in the workflow aligned with the `ndkVersion` in the
   generated Gradle project.

The Android job fails with a clear message if the signing secrets are missing.
That is intentional: a release APK must be signed.

## Desktop signing

Optional for now. Add the usual Tauri secrets if distributing outside GitHub:
- macOS code signing/notarisation (`APPLE_CERTIFICATE`, `APPLE_SIGNING_IDENTITY`,
  `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`).
- Windows Authenticode signing.

Unsigned Linux builds are normal for AppImage/deb/rpm distribution.
