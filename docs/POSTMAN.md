# Postman Collection v2.1

Desktop i CLI volají stejné Rust převody v `apiark-core`.
Interní formát zůstává YAML, s volitelným polem `postman` pro uchování
původních údajů, které původní mapový model neuměl vyjádřit.

Import → export → opětovný import se testuje v `tests/postman.rs`, včetně
skutečného HTTP běhu scope skriptů proti lokálnímu serveru.

| Oblast | Chování |
|---|---|
| Requesty a složky | Zachování názvů, hierarchie a pořadí; bezpečné názvy souborů a kolize |
| URL | Řetězec nebo objekt; raw/host/path/port, query, path variables |
| Query | Opakované parametry zůstávají v URL; disabled položky se neposílají |
| Hlavičky | Posílá se poslední enabled hodnota každého názvu; duplicity a disabled údaje zůstávají v metadatech a import varuje |
| Těla | raw text, JSON, XML, urlencoded a textová multipart form-data |
| Autentizace při importu | Bearer, Basic, API key a explicitní noauth; ostatní import odmítne |
| Dědičnost auth | Request → nejbližší složka → kolekce; explicitní noauth dědičnost vypíná |
| Proměnné kolekce | Zachované, dostupné bez nutnosti vybrat prostředí; předané environment/data hodnoty je přepíší |
| Skripty | Zachované bez konverze pm.* na ark.*; s upozorněním na potřebnou úpravu |
| Scope skripty | Kolekce → nadřazené složky → request pro pre-request i test fázi |
| Metadata a příklady | Uchované pro export; import oznamuje, že neovlivňují běh |

Nepodporovaný HTTP method, body mode, file form field nebo auth při importu
vrátí chybu před zápisem kolekce. Souborové přílohy připoj ručně v requestu.
Nevznikne částečně úspěšný import, který by údaj tiše vynechal.
Raw string hlavičky, externí zdroje skriptů, jiný skriptový jazyk a disabled
skriptové eventy se také odmítnou; nelze je vydávat za úspěšně provedené testy.
Disabled tělo ani disabled proměnné se neposílají/nepoužívají.

`pm.*` runtime, `pm.sendRequest`, asynchronní skripty, vlastní workflow
Postmanu a automatická konverze skriptů nejsou implementované. Skript s `pm.*`
bez úpravy selže; CLI ho neoznačí jako úspěch. Neznámé eventy jsou zachované
pro export s upozorněním, nikoli prováděné jako testy.

Import je převod podporované podmnožiny v2.1, nikoli úplná validace JSON
Schema. Před změnou interního formátu rozšiř reprezentativní roundtrip testy.

Při exportu se původní hlavičky, těla a lokální skripty zachovají, pokud
odpovídající pole nezměníš. Upravené pole se vytvoří z aktuálního requestu.
Metadata tedy nejsou alternativní runtime a nemohou tajně přebít editor.

Nativní Digest/NTLM challenge runtime není dostupný; místo tiché náhrady Basic
vrací chybu. CLI u OAuth vyžaduje předem získaný Bearer token v prostředí.
Desktop ponechává explicitní OAuth získání tokenu s kontrolou `state`.

Omezení mapového YAML modelu je úmyslně viditelné. Pro souběžné editace
opakovaných hlaviček nebo disabled položek je třeba upravit model i editor;
samotný roundtrip uchovává původní Postman údaje.
