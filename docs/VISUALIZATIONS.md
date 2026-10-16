# Typované vizualizace odpovědi

Pouze v post-response nebo test skriptu:

```javascript
const body = ark.response.json();
ark.visualize('image', {
  name: 'Image',
  data: body.result[0].data,
  mimeType: 'image/jpeg'
});
ark.visualize('text', { name: 'Summary', data: 'Hotovo' });
ark.visualize('json', { name: 'Result', data: { ok: true } });
```

Přesné rozhraní:

```typescript
ark.visualize('image', { name?: string, data: string, mimeType: 'image/png' | 'image/jpeg' }): void;
ark.visualize('text',  { name?: string, data: string }): void;
ark.visualize('json',  { name?: string, data: JSONValue }): void;
```

`name` má nejvýš 100 znaků a výchozí hodnotu `Visualization`. `image.data`
je čisté standardní Base64 s paddingem, bez prefixu `data:`. PNG/JPEG se
ověřují podle signatury, rozměrů i dekódovatelnosti a převádějí na statický PNG.
Animace, přidaná metadata a obsah za obrázkem se nepřenášejí do GUI.

Limity celého requestu, společné pro post-response i test skript:

| Omezení | Hodnota |
|---|---|
| Počet výstupů | 4 |
| Obrázek po Base64 dekódování | 4 MiB |
| Součet dat výstupů | 4 MiB |
| Kanonický PNG výstup | 4 MiB na obrázek |
| Každá osa obrázku | 4096 pixelů |
| Počet pixelů | 8 × 1024 × 1024 |
| Text nebo serializované JSON | 256 KiB na výstup |
| Dekodér rastrového obrázku | maximálně 32 MiB alokace |

HTML, SVG, ostatní formáty a automatické vzdálené URL jsou odmítnuté.
Text i JSON se renderují jako text, bez interpretace HTML.

Výstupy se ukazují v response záložce **Visualizations**. Každý výstup patří
requestu, záložce a konkrétnímu spuštění. Nové spuštění výstupy smaže.
Přepnutí záložky nepřesměruje odpověď. Starší dokončení nemůže přepsat novější
běh. Uzavřená záložka nepřijímá dodatečný výsledek.

Při chybě skriptu nebo neplatné vizualizaci se zachová HTTP odpověď,
přidá se neúspěšný test a všechny vizualizace tohoto běhu se zahodí.
Samotné selhání assertion uvnitř platného `ark.test` může ponechat výstupy
pro diagnostiku. Pre-request vizualizace jsou odmítnuté.

CLI používá tutéž validaci. Výstupy nerenderuje ani automaticky neukládá,
v reportu uvádí jejich počet. Překročení limitu znamená selhání běhu a exit 1.
