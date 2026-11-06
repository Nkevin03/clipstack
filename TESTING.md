# Clipstack — Test

## Test automatici

```bash
cd src-tauri && cargo test                          # 62 test Rust
cd src-tauri && cargo test --release -- --ignored --nocapture   # profilazione 50k elementi
pnpm test                                           # Vitest: search-syntax, format, store
pnpm typecheck && pnpm lint
cd src-tauri && cargo clippy --all-targets && cargo fmt --check
```

Copertura Rust: dedup, FTS (prefissi/accenti/maiuscole, sync su update/delete, tag ricercabili), filtri,
retention (fissati e Spaces non scadono, scadenza dati sensibili, limite numero), undo con id stabile,
rilevamento tipo (colori, link, codice + linguaggio, prosa), dati sensibili (Luhn, chiavi API, chiavi
private, IBAN), pianificazione cattura (concealed/transient, policy sensibili, troncamento UTF-8,
priorità file/rich), miniature, colore dominante icone, azioni di trasformazione, impostazioni
(validazione e fallback per chiave), export/import, SQLCipher disponibile, layout del pannello,
scorciatoie, icona menu bar.

Profilazione misurata (MacBook Apple Silicon, `--release`, 50.000 elementi):

| Query | Tempo |
|---|---|
| prima pagina (60) | 0,4 ms |
| ricerca FTS parola | 2 ms |
| ricerca FTS prefisso multiplo | 18 ms |
| pagina a offset 40.000 | 16 ms |
| filtro app | 0,4 ms |
| pulizia 50k → 5000 | ~380 ms (thread in background) |

## Anteprima del design senza backend

`pnpm dev`, poi apri `http://localhost:1420/preview.html` con parametri:
`layout=top|bottom|center`, `theme=light|dark`, `lang=it|en`, `size=small|medium|large`,
`sidebar=0`, `paused=1`, `ax=0` (banner permesso), `modal=context|tags|actions|preview|toast`,
`window=settings&section=privacy`, `window=onboarding`, `window=hud&seq=0..5&seqdone=1`.

## Checklist manuale

Prerequisito: in sviluppo il permesso Accessibilità va dato al terminale/IDE da cui lanci `pnpm tauri dev`;
con l'app installata va dato a **Clipstack.app**.

### Base
- [ ] Primo avvio: compare l'onboarding (3 schermate); "Concedi permesso" apre Impostazioni di Sistema → Accessibilità; la schermata 3 rileva la pressione della scorciatoia.
- [ ] Nessuna icona nel Dock; icona nella menu bar (si adatta a chiaro/scuro).
- [ ] `⌘⇧V` apre il pannello, premuto di nuovo lo chiude. `Esc` (con ricerca vuota) e click fuori lo chiudono.
- [ ] Aprendo il pannello l'app in primo piano resta attiva (la sua menu bar non cambia).
- [ ] Digitare subito cerca; `type:image`, `app:safari`, `tag:x`, `date:7d`, `pinned:yes` diventano chip.

### Incolla
- [ ] TextEdit, Note, Chrome: selezione + `Invio` incolla nel punto del cursore.
- [ ] `⇧Invio` incolla senza formattazione (copiare testo formattato da Safari e incollarlo in TextEdit RTF).
- [ ] `⌘1`…`⌘9` incollano la n-esima card.
- [ ] `⇧→` ×2 poi `Invio` incolla 3 elementi uniti col separatore scelto.
- [ ] Immagine e file: incollati come immagine / come file (Finder, Mail, Slack).

### Sequenza di copia
- [ ] ⌥⇧⌘C: compare il mini-pannello in alto a destra ("Copia qualcosa…"), il focus resta nell'app corrente.
- [ ] In TextEdit copia "uno", "due", "tre": il mini-pannello li mostra in diretta, "tre" in cima con il badge ⌘V.
- [ ] ⌘V ⌘V ⌘V → "tredueuno"; ogni elemento incollato sparisce dalla pila; alla fine "Sequenza completata" e il pannello si chiude.
- [ ] Durante la sequenza copia "a", incolla, copia "b": "b" va in cima, poi ⌘V incolla "b" e poi il resto.
- [ ] Copia una password da un password manager durante la sequenza: ⌘V incolla la password, la pila resta invariata.
- [ ] ⌥⇧⌘C di nuovo (o la ×, o la voce in menu bar) termina la sequenza; fuori sequenza ⌘V ripetuto incolla sempre lo stesso elemento.
- [ ] Ordine "Prima il primo copiato": copia uno, due, tre → ⌘V ⌘V ⌘V → "unoduetre".
- [ ] "A ciclo" attivo: ⌘V ×6 → uno due tre uno due tre; la sequenza resta aperta.
- [ ] × su una riga la toglie; se era la prossima (⌘V), il ⌘V successivo incolla quella dopo.
- [ ] Cambia ordine o ciclo a sequenza aperta: il mini-pannello si riordina subito.
- [ ] "Mostra gli ultimi copiati dopo ogni ⌘V" attivo, numero 5: dopo un ⌘V compare la finestrella con 5 elementi per 5 s; click su una riga la incolla al posto della precedente (annulla con ⌘Z nell'app se serve).
- [ ] Cambia la scorciatoia in Impostazioni → Generale: la nuova funziona e compare nel menu bar e nel mini-pannello.

### App a schermo intero, Spaces, più monitor
- [ ] Safari a schermo intero: il pannello compare sopra l'app.
- [ ] Cambiando Space di macOS il pannello compare nello Space corrente.
- [ ] Due monitor: il pannello compare sullo schermo dove si trova il cursore, nei 3 layout.
- [ ] Stage Manager attivo: il pannello compare e incolla nella finestra attiva.

### Permesso Accessibilità negato
- [ ] Rimuovere Clipstack dall'elenco Accessibilità; `Invio` copia l'elemento, chiude il pannello, mostra la notifica "Elemento copiato, premi ⌘V"; il pannello mostra il banner "Attiva".
- [ ] Riattivando il permesso il banner sparisce alla riapertura del pannello.

### Password manager e privacy
- [ ] Copiare una password da 1Password/Bitwarden/Passwords: **non** compare nel pannello né nel DB
  (`sqlite3 ~/Library/Application\ Support/com.clipstack.app/clipstack.sqlite "select count(*) from items"` invariato).
- [ ] Copiare `4111 1111 1111 1111`: con "Salva e cancella dopo 10 minuti" compare con il timer e sparisce entro 1 minuto dalla scadenza; con "Non salvare" non compare.
- [ ] Pausa dalla menu bar: le copie non vengono salvate, l'icona mostra lo stato di pausa.
- [ ] App aggiunta alle escluse: le sue copie non vengono salvate.
- [ ] Cifratura database attivata: il file `clipstack.sqlite` non inizia più con `SQLite format 3`; riavvio OK; disattivandola torna in chiaro.
- [ ] Con "Recupera titoli e icone dei link" spento, nessuna connessione di rete (Little Snitch / `nettop -p <pid>`).

### Immagini grandi e volumi
- [ ] Screenshot 5K (`⌘⇧⌃4`): miniatura generata, nessun blocco dell'interfaccia; OCR rende il testo ricercabile.
- [ ] Immagine > limite impostato: non salvata.
- [ ] 50k elementi (import di un backup o script): apertura pannello istantanea, scroll fluido, ricerca immediata.
- [ ] Memoria a riposo (Monitoraggio Attività, processo + "Web Content") < 100 MB; CPU idle < 1%.

### Organizzazione
- [ ] `⌘D` fissa, `⌘T` tag (crea al volo), `⌘M` sposta in Space, `⌘E` modifica, `⌘K` trasforma (Invio incolla, ⌘Invio salva), `⌘⌫` elimina e `⌘Z` ripristina.
- [ ] Trascinare una card su uno Space nella sidebar la sposta; trascinare uno Space lo riordina.
- [ ] Menu contestuale con tutte le azioni, "Apri link nel browser", "Mostra nel Finder".
- [ ] `Tab`/`⇧Tab` cambia Space, `⌘B` mostra/nasconde la sidebar, `⌘,` apre le Impostazioni.

### Impostazioni
- [ ] Ogni impostazione persiste dopo il riavvio.
- [ ] Cambio scorciatoia con il registratore: la nuova funziona, la vecchia no; scorciatoia non valida → errore e ripristino.
- [ ] Layout Alto/Basso/Centro, dimensione card, densità, accento (anche quello di sistema), tema Chiaro/Scuro/Auto.
- [ ] Avvio al login: dopo logout/login Clipstack è nella menu bar.
- [ ] Esporta → Cancella cronologia → Importa: elementi, tag, Spaces e immagini tornano.
