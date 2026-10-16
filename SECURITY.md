# Bezpečnost interního forku

Rozsah tohoto repozitáře: macOS desktop, přenositelné CLI a `apiark-core`.
Původní web, pluginový systém a serverové nástroje nejsou součástí forku.
MIT licence a copyright zůstávají zachované.

Reprodukce používají syntetické hodnoty a lokální HTTP servery. Skutečné
credentialy ani privátní kolekce nepatří do issue, logu, testu nebo veřejného
reportu. Nález sdílej soukromě se správcem interního repozitáře.

Očekávané bezpečnostní hranice:

- Cookie výběr respektuje origin, host-only/Domain, Path, Secure a expiraci
  i po změně URL skriptem a v každém kroku přesměrování.
- Přesměrování na jiný origin nepřebírá explicitní cookies, auth ani hlavičky.
  307/308 zachovává metodu a podporované tělo.
- Importované názvy prostředí nemohou opustit zvolenou kolekci; adresářové
  capabilities chrání zápis i při změně symlinků, import nepřepisuje kolize.
- OAuth implicit i authorization-code flow kontroluje `state`.
- Historie neukládá hodnoty requestu; citlivé URL se konzervativně maskují.
- HTTP tělo je omezené při čtení na 10 MiB. QuickJS má 32 MiB paměti,
  1 MiB zásobníku a dvousekundový limit. Backup se celý validuje a omezuje
  před zápisem do aplikace.
- Vizualizace jsou typované a limitované. Aktivní HTML/SVG a vzdálené URL
  nejsou podporované; text se neinterpretuje jako markup.
- Globální data a identita jsou oddělené od upstream aplikace. Updater je
  vypnutý a neobsahuje upstream endpoint ani veřejný klíč.

Testy nejsou potvrzením úplné bezpečnosti všech cest. Zvlášť je třeba ověřit
reálnou distribuci, macOS podpis/notarizaci a ruční GUI scénáře v předání.
Uživatelem napsané skripty, vložené requesty a explicitně připojené soubory
jsou vědomý vstup uživatele; JavaScript nedostává filesystem ani síťové API.
