# Build a vydání pro macOS

Tento fork udržujeme pro macOS, včetně desktopu i CLI. CLI a `apiark-core`
nemají závislost na Tauri. Níže jsou postupy pro lokální build a vydání na Macu.

## Lokální build

Potřebuješ Rust stable s C toolchainem, Node 22/24 a pnpm 11.25.0.
Na Macu jsou potřeba Xcode Command Line Tools. Příkazy jsou ve Fish syntaxi.

```fish
pnpm install --frozen-lockfile
cargo test -p apiark-core -p apiark-cli --locked
cargo test -p apiark --lib --locked
pnpm --filter @apiark/desktop test
pnpm --filter @apiark/desktop lint
pnpm --filter @apiark/desktop build
```

Vývojové okno:

```fish
pnpm --filter @apiark/desktop tauri dev
```

Samostatný macOS bundle bez distribučního podpisu:

```fish
pnpm --filter @apiark/desktop tauri build --bundles app --no-sign --ci
open 'target/release/bundle/macos/ApiArk.app'
```

CLI sestavuj do samostatného adresáře, protože desktopový i CLI binární
soubor se jmenují `apiark`:

```fish
cargo build -p apiark-cli --release --locked --target-dir target/cli
target/cli/release/apiark --help
```

Variantu pro macOS Intel lze sestavit na vhodném hostu s targetem
`x86_64-apple-darwin`. Univerzální desktop potřebuje oba Apple Rust targety
a volbu Tauri `--target universal-apple-darwin`. Postup podpisu univerzálního
vydání je v [release návodu](MACOS_RELEASE.md).

## CLI

```fish
target/cli/release/apiark import synthetic.postman_collection.json --output ./collections
target/cli/release/apiark run ./collections/synthetic --env Development
target/cli/release/apiark run ./collections/synthetic --folder smoke --reporter json
target/cli/release/apiark run ./collections/synthetic --reporter json --output ./report.json
target/cli/release/apiark run ./collections/synthetic --reporter junit --output ./report.xml
target/cli/release/apiark run ./collections/synthetic --data ./synthetic-data.csv --iterations 3
target/cli/release/apiark export ./collections/synthetic --format postman --output ./export.json
```

Reporty: `cli`, `json`, `junit`, `html`. Diagnostika jde na stderr, strojový
report na stdout nebo do `--output`. Exit code 0 znamená, že všechny provedené
testy/assertions prošly; 1 znamená chybu běhu nebo selhání; 2 chybu argumentů.
Samotný HTTP status není test. Pro požadovaný status přidej `assert.status`
nebo `ark.test`. Chyby skriptů, nepodporované Promise testy a neplatný
neprázdný blok assertions nemohou být úspěšný test.

CLI validuje vizualizace stejně jako desktop. Neotevírá GUI a nic automaticky
nezapisuje jako obrázek; JSON report uvádí `visualizationCount` u requestu.
Veřejný npm instalátor byl odstraněn, distribuují se samostatné binární soubory.

JUnit a HTML report mají jeden výsledek na request. JSON navíc uvádí počty
skriptových testů a assertions; selhání obsahuje diagnostiku. Report může
obsahovat URL requestu a chyby testů, proto jeho uložení řiď stejně jako
ostatní výstupy své automatizace. Maskování historie nefiltruje explicitní CLI report.

Příklad nativního `health.yaml` v kolekci s `.apiark/apiark.yaml`:

```yaml
name: Health
method: GET
url: "{{baseUrl}}/health"
preRequestScript: |
  ark.request.setHeader('X-Check', 'internal');
postResponseScript: |
  ark.visualize('json', {name: 'Health', data: ark.response.json()});
tests: |
  ark.test('healthy', () => ark.expect(ark.response.status).to.equal(200));
assert:
  status: 200
```

Interpolace probíhá po pre-request skriptu, takže `ark.env.set` ovlivní
URL, auth, hlavičky i tělo daného běhu. `ark.env` změny se předají dalším
requestům stejné iterace. `ark.globals` a `ark.variables` nejsou trvalý
mezirequestový store; pro návazné requesty používej `ark.env`.

## Identita a data

Bundle ID: `dev.kepak.apiark`. Název: `ApiArk`.
Globální nastavení, historie, logy, cookie persistence a koš jsou v
`~/.apiark-internal`. Původní `~/.apiark` se nemigruje ani nečte.
Kolekce dál používají `.apiark/apiark.yaml` a YAML requesty; jejich umístění
volí uživatel. Otevření stejné složky v obou aplikacích přirozeně sdílí soubory.
Formát kolekce je verze 1 (chybějící `version` znamená 1). Jiné verze obě
aplikace odmítnou bez migrace souborů. Specializované staré protokoly se
nespouštějí jako HTTP; otevření/běh vrátí srozumitelnou chybu.

Od 17. 10. 2026 je bundle ID změněné z `cz.kepakovi.apiark.internal` na
`dev.kepak.apiark`. macOS a WebKit aplikaci považují za novou identitu.
Vlastní datová složka ani cesty ke kolekcím se nemění; migrace původního
WebView úložiště není implementovaná. Jazyk a stav nápověd v localStorage
se proto mohou vrátit na výchozí hodnoty. CI ověřuje skutečné bundle ID
a název v sestaveném Info.plist proti konfiguraci aplikace. Aplikace se nyní
jmenuje `ApiArk`, výstup je `ApiArk.app`. Výchozí složka pro nové importy je
`~/ApiArk`; již uložené kolekce se nepřesouvají.

Binární odpověď lze v desktopu uložit přes Save to file. Přijímání má limit
10 MiB i bez Content-Length; soubory větší než limit se odmítnou. Bajty
zůstávají u odpovědi v paměti, bez dočasných souborů. Přepnutí záložky nebo
nový request během dialogu neovlivní už zvolenou odpověď. Automatický
download velkých streamů není implementovaný.

## CI

V `.github/workflows/ci.yml` jsou testy a release build CLI na macOS arm64,
macOS Intel, Linuxu a Windows. Desktop se testuje a sestavuje na obou typech
Maců; jiné desktopové platformy se nesestavují. Artefakty jsou tar archivy, aby se zachovala
práva spustitelných souborů; obsahují také původní `LICENSE` s copyrightem.
Při ruční distribuci aplikace nebo CLI přibal stejný licenční soubor.
Běžná CI nic nepublikuje a nemá distribuční tajné údaje. Spouští se při
pushi do `main`/`codex/**`, při pull requestu a ručně. Agregovaný check
`CI required` projde pouze při úspěchu všech testů, buildů a validace workflow.
Přerušený nebo přeskočený job se nebere jako úspěch. Starší souběžný běh
stejné větve se ruší. Rust cache je oddělená podle systému a typu buildu;
artefakty běžné CI zůstávají dostupné 14 dní.
Původní release, nightly, benchmark a package-manager workflow jsou odstraněné.
Nový ručně spouštěný workflow je popsaný níže.

Definice jsou pro GitHub Actions v `bckp/apiark`. Git remote `github` ukazuje
na tento veřejný fork, `origin` zůstává na Forgejo `code.kepakovi.cz`.
Push na Forgejo sám GitHub CI nespouští; změny určené pro CI posílej na GitHub.
Podporu Actions na Forgejo zde nepředpokládáme.

```fish
git push github HEAD:main
gh run list --repo bckp/apiark --workflow ci.yml
```

Vývojové změny je vhodné posílat na větev a do `main` přes pull request.
Nepřepisuj historii main ani již vydané tagy.

První hostované testy a buildy z 16. 10. 2026 jsou v
[běhu CI](https://github.com/bckp/apiark/actions/runs/38955818690).
Ověřené byly všechny čtyři varianty CLI a oba macOS desktop buildy.
Stažené archivy mají odpovídající architektury a MIT licenci; Apple Silicon
CLI bylo navíc lokálně spuštěné s `--help`. Nejde o ruční GUI test aplikace
na všech runnerech ani o ověření distribučního podpisu.

## Podpis a notarizace

Aktuální postup pro podpis, notarizaci aplikace i DMG a publikování je v
[anglickém návodu pro macOS release](MACOS_RELEASE.md). Lokální vydání používá
Keychain bez exportu privátního klíče. Univerzální aplikace obsahuje Apple
Silicon i Intel.

## Podepsaný build na GitHubu

Ručně spouštěný workflow `Signed macOS release` připravuje podepsané
a notarizované aplikace i DMG na dvou nativních Mac runnerech. Volitelně
založí draft releasu. Veřejné publikování je samostatný krok. Hosted podpis
stále vyžaduje distribuční Secrets v prostředí `macos-release`. Podrobnosti
a rozdíl oproti lokálnímu vydání popisuje [návod](MACOS_RELEASE.md#build-on-github).
