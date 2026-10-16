# Interní fork ApiArku

Výchozí revize: `46ba45acdc921bcf828c61190d8143e73525fd40`.
Checkout byl čistý. Pracovní větev: `codex/internal-fork`.
Origin: `https://code.kepakovi.cz/bckp/apiark.git`; původní GitHub je `upstream`.
MIT licence a copyright zůstávají zachované.

## Ověřený výchozí stav

- CLI má vlastní modely, interpolaci, HTTP a Postman převody. Nespouští
  skripty ani testy, úspěch určuje pomocí HTTP statusu `< 400`.
- Desktop runner vynechává pre-request skript. Desktop jednotlivého requestu
  zaznamenává chyby post-response/test skriptu pouze do konzole.
- Kolekce mají YAML formát; hlavičky/parametry jako mapy neumějí opakované
  klíče ani disabled položky. Postman převody musejí ztráty hlásit.
- Cookies a redirecty odpovídají popsaným rizikům. Ověření a opravy proběhnou
  proti této revizi pomocí syntetických dat a lokálních serverů.
- `security find-identity -v -p codesigning`: 0 platných identit.
- CLI baseline: `cargo test --manifest-path apps/cli/Cargo.toml`: PASS,
  **0 testů**, čistý debug build 30,47 s (včetně kompilace závislostí).
  Toto není měření rychlosti aplikace.

## Pořadí změn

1. Oddělit existující Rust modely, storage, HTTP, scripting a Postman převody
   do `apiark-core` bez Tauri. Zachovat YAML, sjednotit provádění requestů,
   pre-request/post-response/testů a výsledků desktopu/CLI.
2. Opravit bezpečnostní hranice a přidat reprodukční regresní testy.
3. Postman roundtrip s upozorněními a typované `ark.visualize` (image/text/json).
4. Po kontrole vazeb odstranit web, MCP, VS Code a nepotřebné desktopové
   funkce i backendy a CI. Nikdy nemazat uživatelské kolekce.
5. Izolovat identitu a globální data, vypnout původní updater, upravit macOS
   okna, připravit multiplatformní CLI CI a návod podepsaného vydání.
6. Build, automatické testy, lokální macOS kontrola. Neověřené kroky uvést
   výslovně; bez měření netvrdit výkonnostní zlepšení.

## Upstream

- Issue #124 načteno přes GitHub connector; první verze bude typovaná,
  bez HTML/SVG/automatických vzdálených URL.
- PR #120: zkontrolován celý aktuální diff (1 soubor, +30 řádků,
  head `d1e040cc5568145c7e4aaf1f93d8cef7ea8e591f`). Jednoduchý převod
  `-b/--cookie` na Cookie hlavičku. Nutno rozlišit soubor cookie jar a
  ošetřit chybějící argument, nikoli slepě převzít patch.
- Ostatní PR nejsou převzaté; před případným použitím načíst celý diff.

## Ochrana dat

Nekontrolovat obsah skutečných credentialů ani kolekcí mimo repozitář.
Testy používají výhradně syntetické hodnoty. Podpisové klíče se do repozitáře
neukládají. Push ani publikace nejsou součástí lokálního ověřování.
