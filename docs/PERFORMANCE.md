# Měření výkonu

Lokální macOS arm64, Rust 1.98.0, Node 26.8.2, Vite 6.4.1.
Žádné měření níže není měřením startu GUI. Mac může být při buildu zamčený;
ruční pohledová kontrola okna zůstává samostatným krokem.

## HTTP

Stejný debug profil, loopback server, 10 zahřívacích requestů, 5 sérií po
100 sekvenčních requestech. Příkaz:

```fish
cargo run -p apiark-core --example transport_profile --locked
```

| Varianta | Série v ms | Medián / 100 requestů |
|---|---|---|
| nový klient pro každý request | 34.477, 24.534, 22.981, 22.177, 22.237 | 22.981 ms |
| omezená cache klientů | 17.746, 14.636, 12.884, 13.282, 12.550 | 13.282 ms |

Medián je o 42 % nižší. Jde o mikrobenchmark na localhostu bez TLS a bez
síťové latence; nezobecňuje se na libovolné API. Cache obsahuje nejvýš osm
klientů, odděluje timeout/redirect/TLS nastavení, neobsahuje cookies ani
autentizační hlavičky. Požadavky s proxy nebo vlastními certifikáty ji obcházejí.

## Frontend

Produkční build ve stejném checkoutu a prostředí:

```fish
pnpm --filter @apiark/desktop build
```

| Varianta | Vstupní JS chunk | gzip | Vite build |
|---|---:|---:|---:|
| původní desktop | 4690.04 kB | 1228.17 kB | 10.12 s |
| zmenšený desktop, editor eager | 4537.42 kB | 1196.90 kB | 9.19 s |
| zmenšený desktop, editor lazy | 702.84 kB | 205.20 kB | 8.97 s |
| finální build, včetně izolace prostředí | 654.05 kB | 192.35 kB | 8.73 s |

Vstupní chunk se oproti baseline zmenšil o 85 %. Editor se načte při prvním
zobrazení těla, odpovědi nebo skriptu; jeho samostatný chunk má 3827.40 kB
(989.20 kB gzip). Celkové množství kódu editoru se tím neodstranilo.
Časy buildu jsou jednotlivé vzorky, nikoli statisticky potvrzené zrychlení.

Formátování JSON se memoizuje podle těla a typu odpovědi. Odpovědi nad
1 MiB se zobrazují bez automatického pretty-printu. Tato změna nebyla
samostatně časově změřena; netvrdíme u ní číselné zrychlení.
