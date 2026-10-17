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

Nová lokální kontrola 16. 10. 2026 našla platnou **Developer ID Application**
identitu a `notarytool` je dostupný. Přístup k notarizační službě dosud
ověřený není. Distribuční podpis ani notarizace tohoto forku neproběhly. Bundle sestavený
s `--no-sign` má pouze ad-hoc podpis samotného Mach-O od linkeru, bez Team ID
a bez podepsaných resources. To není Developer ID podpis aplikace.

S vlastní identitou v Keychainu:

```fish
security find-identity -v -p codesigning
set -lx APPLE_SIGNING_IDENTITY 'Developer ID Application: YOUR NAME (TEAMID)'
pnpm --filter @apiark/desktop tauri build --bundles app --ci
codesign --verify --deep --strict --verbose=2 'target/release/bundle/macos/ApiArk.app'
```

Hesla a privátní klíče patří do Keychainu nebo CI secrets, nikdy do checkoutu.
Notarizační profil můžeš uložit interaktivně, bez hesla v příkazu či historii:

```fish
xcrun notarytool store-credentials apiark-notary --apple-id YOUR_APPLE_ID --team-id YOUR_TEAM_ID
ditto -c -k --keepParent 'target/release/bundle/macos/ApiArk.app' /tmp/apiark-notarization.zip
xcrun notarytool submit /tmp/apiark-notarization.zip --keychain-profile apiark-notary --wait
xcrun stapler staple 'target/release/bundle/macos/ApiArk.app'
xcrun stapler validate 'target/release/bundle/macos/ApiArk.app'
spctl --assess --type execute --verbose=2 'target/release/bundle/macos/ApiArk.app'
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

## Podepsaný build na GitHubu

`.github/workflows/macos-release.yml` obsahuje ručně spouštěný workflow
**Signed macOS preview**. Přijímá pouze výchozí větev repozitáře, nepouští
podpis při pull requestu a běží na nativních runnerech pro Apple Silicon
(`macos-15`) a Intel (`macos-15-intel`). Běžné CLI buildy dál zajišťuje CI;
tento release workflow distribuuje desktopovou aplikaci.

Environment `macos-release` už je založený a omezený na větev `main`.
Veřejné údaje `APPLE_SIGNING_IDENTITY` a `APPLE_TEAM_ID` jsou nastavené.
V [nastavení prostředí](https://github.com/bckp/apiark/settings/environments)
je případně možné přidat schválení vydání. Ještě je potřeba nastavit tyto **Secrets**:

| Secret | Obsah |
|---|---|
| `APPLE_CERTIFICATE` | Base64 exportu vlastní Developer ID Application identity včetně privátního klíče do heslem chráněného `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | Heslo tohoto exportu |
| `APPLE_ID` | Účet pro notarizaci |
| `APPLE_PASSWORD` | App-specific password pro notarizaci; nepoužívej běžné heslo účtu |

A tyto environment **Variables**:

| Variable | Obsah |
|---|---|
| `APPLE_SIGNING_IDENTITY` | Přesný název vlastní `Developer ID Application: …` identity |
| `APPLE_TEAM_ID` | Team ID odpovídající distribučnímu certifikátu a notarizačnímu účtu |

Base64 pouze převádí export do textu, není to šifrování. Export klíče,
uložení Secrets a první notarizační přihlášení nebyly v této práci provedené.
Tauri Action importuje certifikát na dočasném GitHub runneru; konfigurace
nevyžaduje přístup k lokálnímu Macu během buildu.

Workflow vyžaduje úspěšný `CI required` pro přesně tentýž commit a znovu
provede `scripts/check.fish`. Potom oficiální Tauri Action
sestaví a podepíše `.app` a předá ji notarizaci. Navazující kontrola vyžaduje
platné sealed resources, Developer ID Application autoritu, očekávané Team ID,
hardened runtime, přiložený notarizační ticket a úspěšný Gatekeeper assessment.
Ověřuje také skutečnou verzi a architekturu aplikace oproti názvu artefaktu.
Nepodepsaná nebo pouze ad-hoc podepsaná aplikace nemůže projít jako vydání.
MIT licence je přibalená přímo do `Contents/Resources/LICENSE` před podpisem.

Výstupem jsou dva ZIP artefakty s podepsanou a staplovanou aplikací,
pro `arm64` a `x64`. Updater zůstává vypnutý, updater klíč není potřeba.
Při ponechání `create_draft = false` vzniknou pouze artefakty běhu.
Volba `true` po úspěchu obou architektur vytvoří **draft prerelease**;
veřejné publikování draftu je samostatný ruční krok.

Tag se odvozuje z `tauri.conf.json` jako `vVERSION`. Před prvním veřejným
vydáním zvol verzi forku a aktualizuj také `apps/desktop/src-tauri/Cargo.toml`
a workspace `Cargo.lock`. Původní `v0.4.6` může již ukazovat na upstream;
workflow odmítne založit release, pokud tag ukazuje na jiný commit.
Existující release se nepřepisuje.

Příprava workflow byla ověřená pomocí actionlint, parsováním YAML a kontrolou
Fish syntaxe. Kontrola přesného CI commitu byla ověřená přes skutečné GitHub
API; ochrana release tagů také pěti syntetickými scénáři s mockovaným `gh`
(nový tag, shodný tag, kolize, chyba API a chyba vytvoření releasu).
Podpis, notarizace a draft release musí ještě projít prvním skutečným
hostovaným během po nastavení Secrets. Buildy bez distribučního podpisu
na obou GitHub Mac runnerech již prošly v běžné CI.

Použité runner labels odpovídají [dokumentaci GitHubu](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
