# Předání forku ApiArk

Stav ověření: 17. 10. 2026, macOS arm64. Aktuální větev `codex/bundle-id`,
výchozí commit `46ba45acdc921bcf828c61190d8143e73525fd40`.
Origin je `https://code.kepakovi.cz/bckp/apiark.git`; původní GitHub zůstává
jako `upstream`. Remote `github` je `git@github.com:bckp/apiark.git`.
Po výslovném schválení uživatelem byl fork pushnut do veřejného GitHub main.
První CI běh: https://github.com/bckp/apiark/actions/runs/38955818690
Distribuční release zveřejněný není.
MIT licence a původní copyright v `LICENSE` jsou beze změny.

17. 10. 2026 byl na žádost uživatele změněn bundle ID na `dev.kepak.apiark`.
Nový lokální release build macOS arm64 prošel; ověřeny byly skutečné ID,
název, verze a přibalená MIT licence. Název zůstává `ApiArk`.
CI i podepsaný release nyní kontrolují ID a název v Info.plist proti Tauri konfiguraci.
Na další žádost uživatele je z názvu odstraněné `Internal`, včetně dalších
oken, výchozí složky pro nové importy (`~/ApiArk`) a distribučních archivů.
Datová složka `~/.apiark-internal` se nemění. WebView localStorage se může
pod novou identitou obnovit na výchozí hodnoty; jeho migrace není implementovaná.

Navazující úprava po kontrole `cemhun/apiark` přidává bezpečnou obnovu relace,
ErrorBoundary panelů a našeptávání názvů proměnných bez hodnot v URL,
parametrech, hlavičkách a Monaco editoru. Rozsah, ovládání a známá omezení
jsou v [VARIABLE_COMPLETION.md](VARIABLE_COMPLETION.md). Workspaces,
persistentní globals, nový DnD ani upgrade závislostí se nepřebíraly.

Následná kontrola dalších forků potvrdila dvojitý Content-Type i v našem
jádru. Oprava doplňuje výchozí typ JSON/XML/raw/binary pouze při chybějící
hlavičce a je ověřená 20 požadavky přes skutečný lokální HTTP server.
Přibalení MIT licence a workflow pro podepsaný/notarizovaný macOS preview
jsou na GitHubu; veřejné vydání aplikace neproběhlo. Environment `macos-release`
má omezení na main, Team ID a název Developer ID identity. Privátní klíč ani
notarizační credentials nebyly exportované ani přenesené do Secrets.
Podrobné výsledky porovnání tří forků, reprodukcí a kontroly podpisů jsou
v [reportu z 16. 10. 2026](FORK_REVIEW_2026-10-16.txt).

## Implementace

- Rust workspace obsahuje Tauri desktop, samostatné CLI a `apiark-core`.
  Jádro nezávisí na Tauri. Modely, YAML storage, prostředí, interpolace,
  HTTP/auth/cookies/redirecty, skripty, assertions, runner a Postman převody
  mají společnou implementaci.
- `execution::execute` je jediná pipeline pro jednotlivé requesty i runner.
  Pre-request změny předcházejí interpolaci. Chyby skriptů a neplatné testy
  selhávají; Promise runtime se nepředstírá. Samotný HTTP status není test.
- Desktop drží prostředí u kolekce aktuálního requestu, ne u první kolekce.
  Změna kolekce/prostředí čistí runtime overrides, staré dokončení načítání
  nemůže převzít nový kontext. `unset` zůstává odstraněný při dalším rozlišení
  hodnot prostředí. Nehotový nebo cizí kontext zablokuje odeslání requestu.
- Postman metadata doplňují existující YAML bez plošné změny formátu.
  Podporované scope skripty/auth/proměnné se provádějí. Duplicitní/disabled
  metadata zůstávají pro export s uvedenými omezeními.
- Typované `ark.visualize('image'|'text'|'json', options)` má společnou
  validaci v GUI/CLI. PNG/JPEG se dekódují a překódují na statický PNG.
  Výstupy patří requestu, záložce a konkrétnímu běhu; chyba skriptu je maže.
- Binární odpovědi do 10 MiB lze explicitně uložit bez dočasných souborů.
  Dialog drží původní odpověď i při přepnutí záložky nebo novém requestu.
- Globální data jsou v `~/.apiark-internal`, bundle ID je
  `dev.kepak.apiark`. Původní globální data se nečtou ani nemigrují.
  Updater včetně původního endpointu, klíče a oprávnění je odstraněný.
- Overlay titlebar se skrytým titulkem a nativními tlačítky je nakonfigurovaný
  pro hlavní i další okna. Toolbar má 52 logických pixelů, zleva rezervu 106;
  nastavení tlačítek `(16,20)` má mířit středem přibližně na 25 px odshora.
  Přetahování je na volných plochách záhlaví. Pohledová kontrola ještě chybí.

Odstraněny jsou celé aplikace web/MCP/VS Code, další formátové konvertory,
desktopové AI integrace, třetí pluginy, terminál, proxy capture, mock a
monitoring, HTML generátor dokumentace, specializované protokolové backendy,
jejich UI/stores/IPC/závislosti, stará release/distribuční CI a veřejný npm
instalátor. Odešly také neaktuální marketingové dokumenty a screenshoty.
Tauri shell/dialog/fs jsou infrastrukturní pluginy nutné pro ponechané funkce.
Žádné soukromé kolekce mimo checkout nebyly kontrolovány ani mazány.

## Ověření bezpečnostních oprav

Jde o konkrétní regresní ověření, nikoli potvrzení úplné bezpečnosti aplikace.
Použita byla syntetická data a lokální servery, bez skutečných credentialů.

| Původní oblast | Oprava a důkaz |
|---|---|
| Cookies | Origin-aware parsování, host-only, hranice domény, public suffix, case-sensitive Path, Secure, expirace/Max-Age; testy výběru a mazání |
| Redirecty | Nový výběr jar cookies pro každou URL, odstranění explicitních hlaviček/auth/cookies při změně originu; lokální test 307/308 s JSON i multipart |
| URL změněná skriptem | Cookie výběr až podle výsledné URL; lokální reprodukce přechodu localhost → 127.0.0.1 |
| Prostředí | Capability-based zápis do adresáře, validace názvu, symlink a create-only kolize; testy traversal, absolutních cest, symlinků a běžných editací |
| OAuth implicit | Přijetí tokenu až po kontrole state; callback test správné, chybějící a cizí hodnoty |
| Historie | Konzervativní maskování všech string hodnot requestu a celé URL za originem; regresní test custom API key, Authorization, body a cookie jmen id/key |
| Prostředky | QuickJS 2 s/32 MiB/1 MiB stack; HTTP streaming cap 10 MiB bez Content-Length; backup členy i celková dekomprese limitované před zápisem |
| HTML dokumentace | Generátor, UI a dostupné IPC odstraněné. Zachovaný CLI HTML report escapuje vložená jména; release smoke ověřil inertní značky |
| Vizualizace | Testy počtu, decoded/combined bytes, text/JSON velikosti, os a pixelů, aktivních formátů a chyb skriptů. JPEG uvnitř JSON je otestovaný přes HTTP a post-response skript |

Hlavní regrese jsou v `crates/apiark-core/tests/runtime.rs`,
`tests/postman.rs`, unit testech `backup`, `safe_environment`, `redaction`,
`scripting/visualization` a v desktopových testech záložek/prostředí/downloadu.
Neproběhl útok proti skutečným citlivým API ani úplný audit všech funkcí.

## Výsledky kontrol

Finální `scripts/check.fish` skončil exit 0:

| Kontrola | Výsledek na tomto Macu |
|---|---|
| Rust fmt | PASS |
| apiark-core | 113 testů PASS: 64 unit, 32 převzatých regresí, 4 Postman, 13 runtime |
| CLI subprocess integrace | 2 testy PASS, včetně exit codes, pm/Promise chyb a výběru složky |
| Desktop OAuth | 1 test PASS |
| Frontend | 45 testů v 8 souborech PASS, žádné přeskočené testy |
| ESLint | PASS, 0 chyb a varování |
| TypeScript + Vite production | PASS |
| cargo check desktop | PASS |
| Clippy core/CLI, všechny targety | PASS; 5 unikátních neblokujících stylových/layout varování |
| CI YAML a Fish syntax | PASS lokální parsování; nejde o hostovaný CI běh |
| Release CLI macOS arm64 | PASS build + skutečný subprocess smoke |
| Release macOS .app | PASS build bez distribučního podpisu |

Release smoke CLI provedl import, HTTP + `ark.test` + vizualizaci, JSON/HTML
report (včetně escapování), export a reimport, JUnit a vynucené selhání s
exit 1. Native proces aplikace při lokálním spuštění zůstal živý bez pádu;
to není pohledové ověření vykresleného GUI. Byl ukončen pouze testovací proces.

Našeptávání URL a skutečného Monaco editoru bylo navíc pohledově ověřené
v izolovaném prohlížečovém náhledu se syntetickými daty, včetně vložení
`{{apiToken}}` do JSON těla. Nejde o nové ruční ověření celé nativní aplikace.
Release CLI a archivy v `target/artifacts` pocházejí z předchozí fáze;
v této úpravě se znovu sestavuje macOS `.app` a provádějí CLI subprocess testy.

Artefakty nejsou commitované:

- `target/release/bundle/macos/ApiArk.app`
- `target/cli/release/apiark`

Starší archivy v `target/artifacts` pocházejí z předchozí fáze a neobsahují
aktuální název/identitu aplikace; pro nový build používej `.app` výše nebo CI.

Finální bundle má přibližně 22 MiB a CLI 7,1 MiB. Kontrola Info.plist
potvrdila interní bundle ID. `codesign -dv` ukázal pouze linkerový ad-hoc
podpis bez Team ID a podepsaných resources, jak odpovídá `--no-sign` buildu.

Měření HTTP a frontend chunků je v [PERFORMANCE.md](PERFORMANCE.md).
Loopback medián poklesl z 22,981 na 13,282 ms / 100 requestů. Počáteční JS
chunk je výrazně menší díky lazy editoru a odstraněným funkcím; měření
rychlosti skutečného startu GUI se neprovedlo.

## Zbývající ruční kroky a omezení

1. Na odemčeném Macu otevřít výslednou aplikaci. Ověřit tmavé záhlaví bez
   samostatného titulkového pruhu, střed tlačítek 24–28 px, kolize s logem
   a Collections, drag region, fullscreen, zen, skrytý sidebar a další okno.
2. V GUI ručně projít novou/uloženou kolekci, dvě různá prostředí/kolekce,
   JPEG v JSON, přepnutí/zavření záložky za běhu, chybový a nový běh,
   uložení binárních dat a lokální přetrvání po restartu. Automatické testy
   těchto stavů nenahrazují skutečný WebKit/native dialog test.
3. Provést podpis/notarizaci podle [BUILD_AND_RELEASE.md](BUILD_AND_RELEASE.md).
   Platná Developer ID identita je nyní dostupná; notarizační přístup ještě
   ověřený není. Nový GitHub workflow vyžaduje vlastní Secrets a jeho první
   skutečný běh teprve musí proběhnout. Lokální build zůstává bez distribučního podpisu.
4. Hostované testy a buildy už prošly na GitHubu: core/CLI na macOS arm64,
   macOS Intel, Linuxu a Windows; desktop na obou typech Maců. Archivy,
   skutečné architektury a MIT licence byly zkontrolované. Universal Mac
   build ani ruční GUI testy na jednotlivých runnerech neproběhly.
   Origin na Forgejo se v této práci neměnil.

Postman runtime není implementovaný; podporovaná podmnožina a odmítnuté
konstrukce jsou v [POSTMAN.md](POSTMAN.md). Mapový editor neumí editovat
duplicitní hlavičky/disabled metadata jednotlivě, roundtrip je zachovává.
`ark.globals`/`ark.variables` nejsou persistentní mezirequestový store;
pro návazné requesty používej `ark.env`. Promise/asynchronní testy selžou.
Nativní Digest/NTLM runtime chybí a vrací chybu. OAuth v CLI řeš předem
získaným Bearer tokenem. Legacy AWS/JWT/SAML nemají zde ověřenou integraci
se skutečnými službami. Velké streamované downloady nad 10 MiB nejsou
součástí této verze. HTML/SVG/remote vizualizace jsou odložené.

## Upstream rozhodnutí a commity

Žádný PR se nepřevzal hromadně ani cherry-pickem:

- #120 celý aktuální diff zkontrolován (`d1e040cc…`): vlastní implementace
  literal cURL cookies; soubor cookie jar se nečte a je odmítnutý.
- #44 celý diff zkontrolován (`a6fb9c94…`): místo lokální regex opravy je
  společná URL logika včetně načítání requestu, editace, odeslání a generování.
- #49 celý diff zkontrolován (`ceacb480…`): vlastní limitovaný přenos bajtů
  a ukládání správné odpovědi; bez původních dočasných souborů.
- #39 celý diff zkontrolován (`137b48a3…`): vložení do URL pole nepřevzato;
  ponechán cURL import dialog s atomickým vytvořením nového requestu.
- #102 a #40 nepřevzaty; duplicitní tab a společná opravená cookie implementace
  pokrývají potřebný rozsah. Jejich úplné diffy se zde nepoužily.
- #129 Flatpak mimo macOS desktop rozsah, nepřevzato.

Průběžné commity obsahují baseline/plán, extrakci jádra, společnou pipeline
a opravy hranic, vizualizace, cURL cookies, Postman převody, odstranění
funkcí/identitu, měření výkonu, okrajové regrese, zjednodušení storage/UI,
izolaci prostředí a závěrečné CI/dokumentaci. Přesná historie:

```fish
git log --oneline 46ba45acdc921bcf828c61190d8143e73525fd40..codex/internal-fork
```
