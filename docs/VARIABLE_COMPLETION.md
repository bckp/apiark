# Našeptávání proměnných

V URL, hodnotách parametrů, hlaviček a formulářových polí i v Monaco editoru
začni psát `{{`. Například `{{api` nabídne `{{apiToken}}`. Filtr nerozlišuje
velikost písmen; vložený název zachová původní zápis.

- Šipky vybírají návrh, Enter nebo Tab jej vloží. V nabídce lze kliknout myší.
- Escape nebo opuštění pole nabídku zavře. Cmd+Enter dál odesílá request.
- V editoru lze nabídku vyvolat také přes Ctrl+Space uvnitř rozepsaného tokenu.
- Doplnění nahradí celý rozepsaný token včetně existujícího `}}` a ponechá
  okolní URL nebo tělo requestu.

Nabídka, tooltipy a detaily návrhů zobrazují **pouze názvy**. Platí to pro
všechny proměnné, nejen pro položky označené jako secrets. Monaco zároveň
nepoužívá slova z obsahu dokumentu jako návrhy. Ruční editor hodnot prostředí
a explicitní úprava proměnné kliknutím v URL zůstávají dostupné.

Seznam respektuje aktivní request, jeho kolekci a složky, zvolené prostředí,
kořenový `.env`, deklarovaná dostupná secrets a runtime změny `ark.env`.
U requestu bez přiřazené kolekce se nenabízejí její scoped Postman proměnné.
Vestavěné názvy jsou `$uuid`, `$timestamp`, `$timestampMs`, `$isoTimestamp`,
`$randomInt`, `$randomFloat`, `$randomString` a `$randomEmail`; jejich hodnoty
se pro našeptávání negenerují.

Pro načítání slouží samostatný Tauri příkaz `get_variable_names`, který
vrací jen pole názvů. Runtime přepis předává pouze název přidané nebo
odstraněné položky. Hodnoty se nedostávají do completion store ani do
objektů návrhů Monaca. Během změny kontextu a při chybě načítání se seznam
vyprázdní; pozdní výsledek předchozího requestu se ignoruje. Po externí
změně souborů proměnných přepni request nebo prostředí pro obnovení nabídky.

Obnova záložek sdílí jednu probíhající operaci i při React StrictMode,
neduplikuje otevřený soubor a během načítání nepřepisuje uloženou relaci
částečným seznamem. Chybu vykreslení izolují ErrorBoundary jednotlivých
panelů s možností opakovat vykreslení; fallback neobsahuje text výjimky.

Změny byly navržené po kontrole forku [cemhun/apiark](https://github.com/cemhun/apiark),
revize `eb1669a04db944a80138e6ae82c3be2958405426`. Jeho commity nejsou
hromadně převzaté. Workspaces, persistentní globals, nový DnD ani upgrade
závislostí nejsou součástí této úpravy.

Izolovaný náhled URL a skutečného Monaco editoru byl pohledově ověřený
v prohlížeči se syntetickými názvy. Vývojový server zde fungoval s Node
24.19.0. Lokální Node 26.8.2 narazil na odstraněné `fs.rmdirSync` s volbou
`recursive` ve stávajícím `vite-plugin-monaco-editor`; produkční build
prochází. Pro vývoj použij Node 22/24, závislosti se kvůli této změně
neupgradovaly.
