# ApiArk Internal

Malý interní fork [ApiArku](https://github.com/berbicanes/apiark), založený na
commitu `46ba45acdc921bcf828c61190d8143e73525fd40`. MIT licence a původní
copyright jsou zachované v [LICENSE](LICENSE).

- Desktop pouze pro macOS, postavený na Tauri 2 a Reactu.
- CLI pro macOS, Linux a Windows.
- Jediné Rust jádro `crates/apiark-core` pro kolekce, prostředí, interpolaci,
  HTTP, cookies, redirecty, skripty, testy a Postman převody.
- HTTP/REST, kolekce a složky, prostředí, autentizace, odpovědi a kolekční běh.
- Post-response vizualizace `image`, `text`, `json` přes `ark.visualize`.
- Postman Collection v2.1 import/export s uchováním metadat a upozorněním
  na omezení. Postman runtime `pm.*` se neimplementuje.

Web, AI, MCP server, VS Code rozšíření, pluginový systém, vestavěný terminál,
proxy capture, monitoring, mock servery, HTML dokumentace a specializované
protokolové backendy jsou odstraněné. Legacy GraphQL request lze používat
jako obyčejný HTTP request s JSON tělem, bez introspekce/subscriptions.

## Začátek

Rust stable, Node 22+, pnpm 11.25.0 a na Macu Xcode Command Line Tools:

```fish
pnpm install --frozen-lockfile
pnpm --filter @apiark/desktop tauri dev
```

```fish
cargo build -p apiark-cli --release --locked --target-dir target/cli
target/cli/release/apiark run ./my-collection --reporter json --output ./report.json
```

Desktop i CLI provádějí pre-request skripty, post-response skripty, `ark.test`
a YAML assertions. Chyba skriptu nebo testu vrací v CLI nenulový exit code.
Nepodporované Promise testy nemohou skončit jako úspěšný přeskočený test.

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

Při lokální kontrole nebyla dostupná platná podpisová identita. Distribuční
Developer ID podpis a notarizace se provádějí podle návodu po doplnění
identity do Keychainu. Nikdy nevkládej tajné údaje do repozitáře.
