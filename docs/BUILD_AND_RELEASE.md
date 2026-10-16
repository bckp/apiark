# Build a interní vydání

Tento fork udržujeme pro macOS, včetně desktopu i CLI. CLI a `apiark-core`
nemají závislost na Tauri. Níže jsou postupy pro lokální build a vydání na Macu.

## Lokální build

Potřebuješ Rust stable s C toolchainem, Node 22+ a pnpm 11.25.0.
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
open 'target/release/bundle/macos/ApiArk Internal.app'
```

CLI sestavuj do samostatného adresáře, protože desktopový i CLI binární
soubor se jmenují `apiark`:

```fish
cargo build -p apiark-cli --release --locked --target-dir target/cli
target/cli/release/apiark --help
```

Variantu pro macOS Intel lze sestavit
na vhodném hostu s targetem `x86_64-apple-darwin`; zde byl sestaven pouze
`aarch64-apple-darwin`. Univerzální desktop potřebuje oba Apple Rust targety
a volbu Tauri `--target universal-apple-darwin`; jeho build zde neproběhl.

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

Bundle ID: `cz.kepakovi.apiark.internal`. Název: `ApiArk Internal`.
Globální nastavení, historie, logy, cookie persistence a koš jsou v
`~/.apiark-internal`. Původní `~/.apiark` se nemigruje ani nečte.
Kolekce dál používají `.apiark/apiark.yaml` a YAML requesty; jejich umístění
volí uživatel. Otevření stejné složky v obou aplikacích přirozeně sdílí soubory.
Formát kolekce je verze 1 (chybějící `version` znamená 1). Jiné verze obě
aplikace odmítnou bez migrace souborů. Specializované staré protokoly se
nespouštějí jako HTTP; otevření/běh vrátí srozumitelnou chybu.

Binární odpověď lze v desktopu uložit přes Save to file. Přijímání má limit
10 MiB i bez Content-Length; soubory větší než limit se odmítnou. Bajty
zůstávají u odpovědi v paměti, bez dočasných souborů. Přepnutí záložky nebo
nový request během dialogu neovlivní už zvolenou odpověď. Automatický
download velkých streamů není implementovaný.

## CI

Pro macOS jsou v `.github/workflows/ci.yml` připravené testy a release build
CLI a test/build desktopu. Artefakty jsou tar archivy, aby se zachovala
práva spustitelných souborů; obsahují také původní `LICENSE` s copyrightem.
Při ruční distribuci aplikace nebo CLI přibal stejný licenční soubor.
CI nic nepublikuje a nemá distribuční tajné údaje.
Původní release, nightly, benchmark a package-manager workflow jsou odstraněné.

Definice je pro GitHub Actions. Na `code.kepakovi.cz` je dostupný Git remote,
ale veřejné zjištění typu serveru/CI vrátilo HTTP 403. Hostované CI nebylo
spuštěno. Pro jiný Actions server je nutné přizpůsobit umístění workflow,
runner labels a artifact action; podporu serveru zde nepředpokládáme.

## Podpis a notarizace

Lokální kontrola `security find-identity -v -p codesigning` našla **0 platných
identit**. Distribuční podpis ani notarizace neproběhly. Bundle sestavený
s `--no-sign` má pouze ad-hoc podpis samotného Mach-O od linkeru, bez Team ID
a bez podepsaných resources. To není Developer ID podpis aplikace.

Po instalaci vlastní identity do Keychainu:

```fish
security find-identity -v -p codesigning
set -lx APPLE_SIGNING_IDENTITY 'Developer ID Application: YOUR NAME (TEAMID)'
pnpm --filter @apiark/desktop tauri build --bundles app --ci
codesign --verify --deep --strict --verbose=2 'target/release/bundle/macos/ApiArk Internal.app'
```

Hesla a privátní klíče patří do Keychainu nebo CI secrets, nikdy do checkoutu.
Notarizační profil můžeš uložit interaktivně, bez hesla v příkazu či historii:

```fish
xcrun notarytool store-credentials apiark-internal-notary --apple-id YOUR_APPLE_ID --team-id YOUR_TEAM_ID
ditto -c -k --keepParent 'target/release/bundle/macos/ApiArk Internal.app' /tmp/apiark-internal-notarization.zip
xcrun notarytool submit /tmp/apiark-internal-notarization.zip --keychain-profile apiark-internal-notary --wait
xcrun stapler staple 'target/release/bundle/macos/ApiArk Internal.app'
xcrun stapler validate 'target/release/bundle/macos/ApiArk Internal.app'
spctl --assess --type execute --verbose=2 'target/release/bundle/macos/ApiArk Internal.app'
```

Před odesláním Apple musí být aplikace podepsaná identitou výše. Odesílá se
vlastní build, nikoli kolekce, historie nebo data uživatele. Pro DMG build
použij Tauri `--bundles dmg` s vlastním podpisem a stejným ověřením výsledku.

macOS podpis/notarizace používá Apple Developer ID. Podpis updater artefaktů
Tauri je samostatný mechanismus s jiným klíčem. Updater je v tomto forku
vypnutý a jeho původní endpoint, veřejný klíč, plugin a oprávnění odstraněné.
Před případným návratem updateru je třeba vlastní endpoint, vlastní updater
klíč a ověření aktualizace na testovací instalaci.

Postup vychází z [oficiální dokumentace Tauri](https://v2.tauri.app/distribute/sign/macos/).
