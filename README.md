# ApiArk Internal

Interní udržovací fork ApiArku pro vlastní používání na **macOS**.
Desktopová aplikace a CLI sdílejí Rust jádro `apiark-core`, stejné kolekce
a stejné provádění HTTP požadavků, skriptů a testů.

## Funkce

- Desktop pouze pro macOS, postavený na Tauri 2 a Reactu.
- CLI pro macOS s reporty pro automatizaci.
- Jediné Rust jádro `crates/apiark-core` pro kolekce, prostředí, interpolaci,
  HTTP, cookies, redirecty, skripty, testy a Postman převody.
- HTTP/REST, kolekce a složky, prostředí, autentizace, odpovědi a kolekční běh.
- Našeptávání `{{proměnných}}` v URL, hodnotách parametrů/hlaviček a editoru;
  návrhy obsahují pouze názvy, nikdy hodnoty. [Ovládání a rozsah](docs/VARIABLE_COMPLETION.md).
- Post-response vizualizace `image`, `text`, `json` přes `ark.visualize`.
- Postman Collection v2.1 import/export s uchováním metadat a upozorněním
  na omezení. Postman runtime `pm.*` se neimplementuje.

Web, AI, MCP server, VS Code rozšíření, pluginový systém, vestavěný terminál,
proxy capture, monitoring, mock servery, HTML dokumentace a specializované
protokolové backendy jsou odstraněné. Legacy GraphQL request lze používat
jako obyčejný HTTP request s JSON tělem, bez introspekce/subscriptions.

## Požadavky pro build na Macu

- macOS a Xcode Command Line Tools.
- Rust stable včetně `cargo`.
- Node.js 22 nebo 24 a pnpm **11.25.0** podle `package.json`.
- Fish pro níže uvedené příkazy.

Pokud Command Line Tools ještě nemáš, spusť jejich instalaci a dokonči ji
v systémovém dialogu:

```fish
xcode-select --install
```

Všechny další příkazy spouštěj z kořene repozitáře. Nainstaluj závislosti:

```fish
pnpm install --frozen-lockfile
```

## Sestavení aplikace

Lokální release build bez distribučního podpisu:

```fish
pnpm --filter @apiark/desktop tauri build --bundles app --no-sign --ci
```

Výsledkem je `target/release/bundle/macos/ApiArk Internal.app`. Spuštění:

```fish
open 'target/release/bundle/macos/ApiArk Internal.app'
```

Build byl ověřený na Macu s Apple Silicon. Podpis vlastní Developer ID
identitou, notarizace a další macOS build varianty jsou popsané v
[návodu pro build a vydání](docs/BUILD_AND_RELEASE.md).

Pro vývoj s automatickým načítáním změn:

```fish
pnpm --filter @apiark/desktop tauri dev
```

## Sestavení a použití CLI

CLI sestavuj do samostatného adresáře: desktopový i CLI binární soubor se
jmenují `apiark`.

```fish
cargo build -p apiark-cli --release --locked --target-dir target/cli
target/cli/release/apiark --help
target/cli/release/apiark run ./my-collection --reporter json --output ./report.json
```

Výsledný binární soubor je `target/cli/release/apiark`. Podporované reporty
jsou `cli`, `json`, `junit` a `html`. Příklady importu, exportu, prostředí
a běhu složek najdeš v [návodu CLI](docs/BUILD_AND_RELEASE.md#cli).

Desktop i CLI provádějí pre-request skripty, post-response skripty, `ark.test`
a YAML assertions. Chyba skriptu nebo testu vrací v CLI nenulový exit code.
Nepodporované Promise testy nemohou skončit jako úspěšný přeskočený test.

## Ověření změn

Kompletní lokální kontrola formátování, Rust testů, Clippy, frontendových
testů, lintu, frontend buildu a desktopové kompilace:

```fish
fish scripts/check.fish
```

Automatické kontroly doplň ručním vyzkoušením aplikace na Macu, zvlášť při
změnách oken, záložek, editorů a ukládacích dialogů.

## Dokumentace

- [Build, CLI, data, CI a podepsané vydání](docs/BUILD_AND_RELEASE.md)
- [Postman kompatibilita a omezení](docs/POSTMAN.md)
- [Přesné API a limity vizualizací](docs/VISUALIZATIONS.md)
- [Měření výkonu](docs/PERFORMANCE.md)
- [Ověřený výchozí stav a plán](docs/FORK_PLAN.md)
- [Stav předání a neověřené kroky](docs/HANDOFF.md)

Fork používá bundle ID `cz.kepakovi.apiark.internal` a data v
`~/.apiark-internal`. Původní updater je odstraněný. Kolekce zůstávají ve
stejném YAML formátu, rozšířeném o volitelná Postman metadata.

## Původ a licence

Fork vychází z [ApiArku](https://github.com/berbicanes/apiark), z commitu
`46ba45acdc921bcf828c61190d8143e73525fd40`. MIT licence a původní copyright
jsou zachované v [LICENSE](LICENSE). Při distribuci aplikace nebo CLI přibal
také tento licenční soubor.
