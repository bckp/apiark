# Signed macOS releases

A distribution signature identifies the publisher to macOS.
Apple notarization scans a signed build and issues a ticket.
Stapling attaches that ticket to the application or disk image.
The release requires a valid Developer ID Application identity and access to Apple notarization.
Keep private keys and passwords in Keychain or GitHub Secrets.

## Prepare a version

Use the next unused patch version after the highest release tag.
Update the desktop version in `tauri.conf.json`, its `Cargo.toml`, `Cargo.lock`, and both private `package.json` files.
Leave the independently versioned core and CLI packages unchanged.
Add English release notes under `docs/releases/`.
Run `fish scripts/check.fish` from the repository root.
Commit the changes on a `codex/` branch.
Wait for `CI required` to pass for that exact commit before public publication.
Never replace an existing release tag or overwrite a published download.

## Build on a Mac

Run these Fish commands from the repository root.
The universal build needs both Apple Rust targets and Xcode Command Line Tools.
Use the exact Developer ID Application name from Keychain.

```fish
security find-identity -v -p codesigning
rustup target add aarch64-apple-darwin x86_64-apple-darwin
set -lx APPLE_SIGNING_IDENTITY 'Developer ID Application: YOUR NAME (TEAMID)'
pnpm --filter @apiark/desktop tauri build --target universal-apple-darwin --bundles app --ci
set -l release_app target/universal-apple-darwin/release/bundle/macos/ApiArk.app
codesign --verify --deep --strict --verbose=2 "$release_app"
codesign -dv --verbose=4 "$release_app"
lipo -archs "$release_app/Contents/MacOS/apiark"
```

Make sure that the signature contains the expected Developer ID Application authority, Team ID, timestamp, and hardened runtime flag.
Make sure that the executable contains both `arm64` and `x86_64`.
Make sure that `Contents/Info.plist` contains the intended version and identifier `dev.kepak.apiark`.
Make sure that `Contents/Resources/LICENSE` matches the repository license.

## Notarize the application

Use an existing Keychain profile for `notarytool`.
If no profile exists, create one with this interactive command.
Enter an app-specific password at the secure prompt.
Do not put the password in the command or repository.

```fish
xcrun notarytool store-credentials apiark-notary --apple-id YOUR_APPLE_ID --team-id YOUR_TEAM_ID
```

The commands below use that profile and the `release_app` variable from the build step.
Only the application archive goes to Apple. User collections and application data stay on the Mac.

```fish
ditto -c -k --keepParent "$release_app" /tmp/ApiArk-notarization.zip
xcrun notarytool submit /tmp/ApiArk-notarization.zip --keychain-profile apiark-notary --wait --timeout 30m --output-format json > /tmp/ApiArk-app-notarization.json
python3 -c 'import json; assert json.load(open("/tmp/ApiArk-app-notarization.json"))["status"] == "Accepted"'
xcrun stapler staple "$release_app"
xcrun stapler validate "$release_app"
codesign --verify --deep --strict --verbose=2 "$release_app"
spctl --assess --type execute --verbose=2 "$release_app"
```

If Apple does not return `Accepted`, stop the release.
Read the submission log with `xcrun notarytool log SUBMISSION_ID --keychain-profile apiark-notary`.

## Package and notarize the DMG

A DMG is a disk image that contains the application.
Package the stapled application so that its ticket remains inside the download.
Replace `VERSION` with the version from `tauri.conf.json`.
The commands use the signature identity and application path from the earlier steps.

```fish
set -l release_stage (mktemp -d /tmp/apiark-dmg.XXXXXX)
set -l release_dmg /tmp/ApiArk-VERSION-macos-universal.dmg
ditto "$release_app" "$release_stage/ApiArk.app"
ln -s /Applications "$release_stage/Applications"
hdiutil create -volname ApiArk -srcfolder "$release_stage" -format UDZO "$release_dmg"
codesign --force --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$release_dmg"
xcrun notarytool submit "$release_dmg" --keychain-profile apiark-notary --wait --timeout 30m --output-format json > /tmp/ApiArk-dmg-notarization.json
python3 -c 'import json; assert json.load(open("/tmp/ApiArk-dmg-notarization.json"))["status"] == "Accepted"'
xcrun stapler staple "$release_dmg"
xcrun stapler validate "$release_dmg"
codesign --verify --strict --verbose=2 "$release_dmg"
spctl --assess --type open --context context:primary-signature --verbose=2 "$release_dmg"
hdiutil verify "$release_dmg"
shasum -a 256 "$release_dmg"
```

Mount the final image read-only before publication.
Make sure that its application retains the intended version, architectures, signature, ticket, and license.
Test the application on a Mac. Record any untested GUI behavior or hardware.
Publish the DMG, SHA-256 checksum, release notes, and build record on GitHub.
The build record must identify the source commit, test results, and accepted notarization submissions.
Tag the exact source commit as `vVERSION`.

## Build on GitHub

The manual `Signed macOS release` workflow accepts only the default branch.
The `macos-release` environment restricts signing to `main`.
The workflow requires a successful `CI required` result for the same commit.
It repeats the local tests before signing.
It builds Apple Silicon and Intel applications on separate native runners.

Set these GitHub Secrets in that environment:

- `APPLE_CERTIFICATE`: Base64 text of a password-protected `.p12` export that includes the Developer ID Application private key.
- `APPLE_CERTIFICATE_PASSWORD`: The password for that export.
- `APPLE_ID`: The Apple account for notarization.
- `APPLE_PASSWORD`: An app-specific password for that account.

Set `APPLE_SIGNING_IDENTITY` and `APPLE_TEAM_ID` as environment variables in GitHub.
Base64 does not encrypt the certificate export.
Local releases do not require a private key export or GitHub Secrets.
The first hosted signed build still needs these Secrets.

Tauri signs and notarizes each application, then signs its DMG.
The workflow submits each DMG separately and attaches its ticket.
The workflow requires Developer ID signatures, the expected Team ID, valid tickets, and successful Gatekeeper assessments.
It also compares the application identity, version, architecture, and bundled license with the source.
The output contains two application ZIP archives, two DMGs, notarization records, and SHA-256 checksum files.

The default `create_draft = false` produces workflow artifacts only.
With `create_draft = true`, the workflow creates a draft release after both architectures pass.
Public publication remains a separate action.
The workflow rejects a tag that points to another commit.
It does not replace an existing release.

## Updater signatures

Apple code signing and Tauri updater signing use different keys.
This fork removes the updater plugin, original endpoint, public key, and permissions.
No updater key is necessary for these releases.
Download later versions from GitHub Releases.

The procedure follows the official [Tauri signing documentation](https://v2.tauri.app/distribute/sign/macos/)
and [DMG documentation](https://v2.tauri.app/distribute/dmg/).
