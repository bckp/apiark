# ApiArk

[![CI](https://github.com/bckp/apiark/actions/workflows/ci.yml/badge.svg)](https://github.com/bckp/apiark/actions/workflows/ci.yml)

A maintenance fork of ApiArk for personal use on **macOS**.
The desktop application and CLI share the `apiark-core` Rust library,
collection format, and execution of HTTP requests, scripts, and tests.

## Download for macOS

Download the DMG from [GitHub Releases](https://github.com/bckp/apiark/releases/latest).
Version 0.4.7 provides one universal application for Apple Silicon and Intel Macs.
The release uses Developer ID signing and Apple notarization.
Open the DMG, then drag `ApiArk.app` to Applications.
Automatic updates are disabled. Download future releases from the same page.

## Features

- A macOS desktop application built with Tauri 2 and React.
- A macOS CLI with reports for automation.
- A shared Rust core in `crates/apiark-core` for collections, environments,
  variable interpolation, HTTP, cookies, redirects, scripts, tests, and Postman conversions.
- HTTP/REST requests, collections and folders, environments, authentication,
  response inspection, and collection runs.
- `{{variable}}` completion in URLs, parameter/header values, and the editor.
  Suggestions show names only, never values. [Scope and controls](docs/VARIABLE_COMPLETION.md).
- Post-response `image`, `text`, and `json` visualizations through `ark.visualize`.
- Postman Collection v2.1 import/export with preserved metadata and explicit
  compatibility warnings. The Postman `pm.*` runtime is not implemented.

The web application, AI integrations, MCP server, VS Code extension, plugin
system, embedded terminal, proxy capture, monitoring, mock servers, HTML
documentation generator, and specialized protocol backends have been removed.
Legacy GraphQL requests can run as ordinary HTTP requests with a JSON body,
without introspection or subscriptions.

## Build requirements

- macOS with Xcode Command Line Tools.
- Stable Rust, including `cargo`.
- Node.js 22 or 24 and pnpm **11.25.0**, as specified in `package.json`.
- Fish for the commands below.

If Xcode Command Line Tools are missing, start the installer and complete
the system dialog:

```fish
xcode-select --install
```

Run the remaining commands from the repository root. Install dependencies:

```fish
pnpm install --frozen-lockfile
```

## Build the desktop application

Create a local release build without a distribution signature:

```fish
pnpm --filter @apiark/desktop tauri build --bundles app --no-sign --ci
```

The application is created at `target/release/bundle/macos/ApiArk.app`.
Launch it with:

```fish
open 'target/release/bundle/macos/ApiArk.app'
```

Developer ID signing, notarization, and universal builds are covered in the
[build and release guide](docs/BUILD_AND_RELEASE.md).
A [manually triggered GitHub workflow](docs/BUILD_AND_RELEASE.md#podepsaný-build-na-githubu)
is available for signed Apple Silicon and Intel builds, with an optional
draft release. It produces notarized DMG images and application ZIP archives.
The hosted workflow still requires distribution secrets. Local releases use the macOS Keychain.

For development with automatic reloading:

```fish
pnpm --filter @apiark/desktop tauri dev
```

## Build and use the CLI

Use a separate target directory for the CLI: both the desktop executable
and the CLI are named `apiark`.

```fish
cargo build -p apiark-cli --release --locked --target-dir target/cli
target/cli/release/apiark --help
target/cli/release/apiark run ./my-collection --reporter json --output ./report.json
```

The executable is created at `target/cli/release/apiark`. Supported report
formats are `cli`, `json`, `junit`, and `html`. See the
[CLI guide](docs/BUILD_AND_RELEASE.md#cli) for import/export, environment,
and folder execution examples.

Both the desktop and CLI execute pre-request scripts, post-response scripts,
`ark.test`, and YAML assertions. Script errors and test failures produce a
nonzero CLI exit code. Unsupported Promise tests fail explicitly.

## Validate changes

Run the local checks for formatting, Rust tests, Clippy, frontend tests,
lint, the frontend build, and desktop compilation:

```fish
fish scripts/check.fish
```

Also test the application manually on a Mac, particularly after changes
to windows, tabs, editors, or save dialogs.

## Documentation

The detailed guides below are currently in Czech:

- [Build, CLI, data, CI, and signed releases](docs/BUILD_AND_RELEASE.md)
- [Postman compatibility and limitations](docs/POSTMAN.md)
- [Visualization API and limits](docs/VISUALIZATIONS.md)
- [Performance measurements](docs/PERFORMANCE.md)
- [Verified baseline and fork plan](docs/FORK_PLAN.md)
- [Handoff status and remaining verification](docs/HANDOFF.md)

The fork uses bundle ID `dev.kepak.apiark` and stores application
data in `~/.apiark-internal`. The original updater has been removed.
Collections retain the YAML format, extended with optional Postman metadata.

## Origin and license

This fork is based on [ApiArk](https://github.com/berbicanes/apiark), commit
`46ba45acdc921bcf828c61190d8143e73525fd40`. The MIT license and original
copyright notices are preserved in [LICENSE](LICENSE). Include this license
file when distributing the desktop application or CLI.
