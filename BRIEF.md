# Clipstack — Brief completo per Claude Code

> Nome di lavoro: **Clipstack** (rinominabile). Non usare il nome, le icone o gli asset di "Pasty": ispirati solo all'idea e al layout generale, il design deve essere originale.

## 0. Come lavorare (istruzioni per Claude Code)

- Questo file è la specifica completa. Leggila tutta prima di scrivere codice.
- Lavora **a fasi** (sezione 12). Prima di ogni fase scrivi un piano breve, poi implementa, poi verifica che compili (`cargo check`, `pnpm build`) e che l'app parta (`pnpm tauri dev`). Fai un commit git a fine fase con messaggio chiaro.
- Se una scelta non è specificata, scegli l'opzione più semplice che rispetta la specifica e annotala in `DECISIONS.md` (una riga: scelta + motivo).
- Le API di Tauri 2 e dei plugin cambiano spesso: **controlla la documentazione ufficiale e i README aggiornati** (docs.rs / GitHub) prima di usare un'API, non andare a memoria.
- Se un requisito è impossibile o troppo fragile su macOS, dillo chiaramente e proponi l'alternativa migliore invece di nasconderlo.
- Non lasciare codice finto, TODO silenziosi o funzioni "simulate": ogni funzione elencata deve funzionare davvero.

---

## 1. Obiettivo

Un gestore della cronologia degli appunti per **macOS**, veloce e bellissimo, che vive nella menu bar. Con una scorciatoia globale (**default: `⌘⇧V`**) compare un pannello con la cronologia di tutto ciò che ho copiato (testo, link, immagini, file, colori, codice). Posso cercare, filtrare, taggare, fissare, organizzare in Spaces e incollare nell'app che stavo usando con un tasto.

Ispirazione di layout (solo come riferimento): pannello che scende dalla parte alta dello schermo sotto la menu bar, a tutta larghezza, con **card orizzontali** in anteprima, **barra laterale** con le raccolte ("Spaces"), **barra di ricerca** al centro, **filtri per app di origine** in alto a destra, **icona impostazioni**. Ogni card mostra: icona e nome dell'app di origine nell'intestazione colorata, anteprima del contenuto, "tempo fa" e numero scorciatoia (`⌘1`…`⌘9`) nel piede.

Target: macOS 13+ (Ventura), Apple Silicon e Intel (universal binary).

---

## 2. Stack tecnico

- **Tauri 2** (Rust) come shell nativa.
- **Frontend**: React + TypeScript + Vite, Tailwind CSS, Framer Motion (animazioni), Zustand (stato), lucide-react (icone), `@tanstack/react-virtual` (liste virtualizzate), highlight.js o shiki (syntax highlight del codice).
- **Database**: SQLite via `rusqlite` (feature `bundled`) con **FTS5** per la ricerca. Opzionale: SQLCipher per cifrare il DB, con chiave salvata nel Keychain (crate `keyring`).
- **Crate Rust utili**: `objc2` / `objc2-app-kit` / `objc2-foundation` (NSPasteboard, NSWorkspace), `core-graphics` (CGEvent), `window-vibrancy` (effetto vetro), `image` (miniature), `sha2` (hash), `serde`/`serde_json`, `tokio` o thread dedicati, `chrono`, `url`, `regex`.
- **Plugin Tauri**: `global-shortcut`, `autostart`, `store` o impostazioni su SQLite, `opener`, `dialog`, `fs` (solo se serve). Per far comparire il pannello **senza rubare il focus** all'app corrente usa un **NSPanel** (es. plugin `tauri-nspanel`, verifica la versione compatibile con Tauri 2 oppure implementa con `objc2`).
- Gestore pacchetti: **pnpm**.
- `tauri.conf.json`: `"macOSPrivateApi": true` (finestra trasparente), app con `ActivationPolicy::Accessory` (niente icona nel Dock, solo menu bar).

---

## 3. Struttura progetto

```
clipstack/
├─ BRIEF.md              (questo file)
├─ DECISIONS.md
├─ src/                  (frontend)
│  ├─ components/        Panel, Sidebar, SearchBar, AppFilterBar, Card/*, PreviewModal,
│  │                     TagPicker, ActionsMenu, Settings/*, Onboarding/*
│  ├─ store/             zustand stores (items, ui, settings)
│  ├─ hooks/             useKeyboardNav, useItems, useTauriEvent
│  ├─ lib/               ipc.ts, format.ts, search-syntax.ts, i18n/
│  └─ styles/
└─ src-tauri/
   ├─ src/
   │  ├─ main.rs / lib.rs
   │  ├─ clipboard/      monitor.rs, pasteboard.rs, kinds.rs (rilevamento tipo)
   │  ├─ db/             schema.rs, queries.rs, migrations.rs
   │  ├─ paste.rs        scrittura appunti + simulazione ⌘V
   │  ├─ panel.rs        finestra/NSPanel, posizionamento, vibrancy
   │  ├─ shortcuts.rs    registrazione scorciatoie globali
   │  ├─ tray.rs         icona e menu nella menu bar
   │  ├─ privacy.rs      esclusioni, rilevamento dati sensibili, scadenze
   │  ├─ actions.rs      trasformazioni di testo
   │  ├─ permissions.rs  Accessibility check/richiesta
   │  └─ settings.rs
   └─ tauri.conf.json
```

---

## 4. Funzionalità

### 4.1 Monitoraggio degli appunti
- Un thread dedicato controlla `NSPasteboard.general.changeCount` ogni ~250 ms (macOS non ha notifiche per i cambi). Dentro un autorelease pool. CPU idle ≈ 0%.
- Al cambio legge i tipi disponibili (`public.utf8-plain-text`, `public.rtf`, `public.html`, `public.png`/`public.tiff`, `public.file-url`, `public.url`, ecc.) e salva **tutte le rappresentazioni utili** in modo che l'incolla possa ripristinare formattazione originale.
- **App di origine**: al momento della copia leggi `NSWorkspace.frontmostApplication` (bundle id, nome, icona). Estrai il **colore dominante dell'icona** (per colorare l'intestazione della card) e mettilo in cache.
- **Deduplica** con hash SHA-256 del contenuto: se esiste già, non creare un duplicato ma porta l'elemento in cima aggiornando `last_used_at`.
- **Evita il loop**: quando l'app stessa scrive negli appunti (incolla), ricorda il `changeCount` risultante e ignoralo.
- Limiti configurabili: dimensione massima per elemento (default 25 MB per immagini, 1 MB per testo, oltre si tronca o si ignora), numero massimo elementi (default 5000) e/o giorni di conservazione (default 90). Gli elementi **fissati** e quelli in **Spaces personalizzati** non scadono mai.
- Pulizia automatica in background (all'avvio e ogni ora), rimuovendo anche i file immagine orfani.

### 4.2 Tipi di contenuto (rilevamento automatico)
| Tipo | Rilevamento | Card |
|---|---|---|
| Testo | testo semplice | prime ~8 righe, font di sistema |
| Rich text | RTF/HTML presenti | anteprima formattata, opzione "incolla senza formattazione" |
| Link | URL valido | favicon + titolo + dominio (titolo/favicon recuperati **solo se l'opzione di rete è attiva**, default spento) |
| Immagine | PNG/TIFF/JPEG | miniatura (max 400 px) + dimensioni e peso |
| File | file-url (anche multipli) | icona del tipo di file (da NSWorkspace) + nome + conteggio |
| Colore | `#hex`, `rgb()`, `hsl()` | grande campione di colore + valori convertibili |
| Codice | euristica (indentazione, parentesi, keyword) | sfondo scuro, monospazio, syntax highlight, linguaggio rilevato |

### 4.3 Pannello e scorciatoia globale
- **Scorciatoia di default: `⌘⇧V`**, modificabile nelle impostazioni con un "registratore di scorciatoie". Premere la scorciatoia a pannello aperto lo chiude (toggle).
- Il pannello è un **NSPanel non attivante**: non sottrae il focus all'app in primo piano, appare anche sopra app a schermo intero e su tutti gli Spaces di macOS.
- **Multi-monitor**: appare sullo schermo dove si trova il cursore.
- **Tre layout** selezionabili nelle impostazioni:
  1. **Alto** (default, come nel riferimento): a tutta larghezza sotto la menu bar, card orizzontali.
  2. **Basso**: a tutta larghezza in fondo allo schermo, card orizzontali.
  3. **Centro**: finestra flottante stile Spotlight, lista verticale a sinistra + anteprima grande a destra.
- Si chiude con `Esc` (se la ricerca ha testo, il primo `Esc` la svuota), con click fuori dal pannello, o dopo l'incolla.
- Apertura istantanea (< 100 ms percepiti): la finestra resta caricata e viene solo mostrata/nascosta, non ricreata.

### 4.4 Incolla
- `Invio` o doppio click: scrive l'elemento negli appunti, nasconde il pannello e **simula `⌘V` nell'app precedente** con `CGEvent` (keycode V = 9, flag Command). Piccolo ritardo (~50–80 ms) tra scrittura appunti, chiusura pannello e invio dell'evento.
- `⇧Invio`: incolla **come testo semplice** (senza formattazione).
- `⌘1`…`⌘9`: incolla direttamente l'n-esima card visibile.
- **Selezione multipla** (`⇧+frecce` o `⌘+click`): incolla tutti gli elementi uniti con un separatore configurabile (a capo, spazio, virgola, nessuno).
- Se manca il permesso Accessibilità: copia comunque negli appunti, chiudi il pannello e mostra una notifica "Elemento copiato, premi ⌘V" + pulsante per aprire le impostazioni.
- Opzione "Solo copia, non incollare automaticamente".
- Opzione "Sposta in cima dopo l'incolla" (default attiva) e contatore `use_count`.

### 4.5 Ricerca
- Barra di ricerca sempre pronta: **basta iniziare a scrivere** a pannello aperto (anche `⌘F`).
- Ricerca istantanea con **FTS5** (prefissi, senza distinzione di accenti/maiuscole) su testo, titolo dei link, nome app di origine, testo OCR, tag. Risultati con **evidenziazione** delle parole trovate nell'anteprima.
- **Sintassi filtri** nella stessa barra: `type:image`, `type:link`, `app:safari`, `tag:lavoro`, `space:snippets`, `date:today` / `date:7d`, `pinned:yes`. I filtri compaiono come "chip" rimovibili.
- **Barra filtri per app** (in alto a destra, come nel riferimento): icone delle app di origine più recenti, click per filtrare, click di nuovo per togliere il filtro. Più filtri per tipo (testo, link, immagini, file, colori, codice).
- Ordinamento: più recenti (default), più usati, fissati prima.
- Risposta sotto i 50 ms anche con 10.000 elementi.

### 4.6 Spaces e Tag
- **Spaces** (barra laterale, come "History / Presentation / Snippets / Read later" nel riferimento): "History" (tutto), "Fissati", e Spaces personalizzati con nome, colore e icona. Creazione con "+ Nuovo Space", rinomina, riordino con drag & drop, eliminazione (con conferma). Un elemento può essere spostato in uno Space con trascinamento, menu contestuale o `⌘M`.
- Gli elementi in Spaces personalizzati **non scadono mai**. Lo Space "Snippets" è preinstallato per i testi riutilizzabili.
- **Tag**: etichette colorate, più tag per elemento. `⌘T` apre il selettore rapido (cerca o crea al volo). I tag appaiono come pallini/etichette sulla card, e nella sidebar c'è una sezione "Tag" per filtrare con un click. Gestione completa (rinomina, cambia colore, elimina, unisci) nelle impostazioni.
- Contatori accanto a Spaces e Tag.

### 4.7 Azioni sugli elementi
- **Fissa/Rimuovi** (`⌘D`), **Elimina** (`⌘⌫`, con annulla `⌘Z` per qualche secondo), **Modifica testo** (`⌘E`, editor semplice), **Anteprima grande** (`Spazio`: testo completo scorrevole, immagine a piena risoluzione, link con titolo, codice con highlight).
- **Menu azioni `⌘K`** sul testo selezionato, con trasformazioni (il risultato viene incollato oppure salvato come nuovo elemento): MAIUSCOLO / minuscolo / Title Case, rimuovi formattazione, trim spazi, rimuovi righe duplicate, ordina righe, JSON prettify/minify, URL encode/decode, Base64 encode/decode, conta caratteri/parole.
- **Menu contestuale** (tasto destro) con tutte le azioni sopra più "Apri link nel browser", "Mostra nel Finder", "Copia senza incollare".
- **Trascinamento fuori** della card verso un'altra app (immagini e file; se in Tauri è troppo instabile, segnalalo e salta).

### 4.8 Privacy e sicurezza
- **Rispetta i flag standard** `org.nspasteboard.ConcealedType` e `org.nspasteboard.TransientType`: quei contenuti (password manager, ecc.) **non vengono mai salvati**.
- **Lista app escluse** configurabile, precompilata con: 1Password, Bitwarden, Dashlane, LastPass, Enpass, KeePassXC, Keychain Access. Aggiunta di un'app dalle impostazioni con selettore di app.
- **Rilevamento dati sensibili** (numeri di carta valida con Luhn, chiavi API/token comuni, chiavi private `-----BEGIN`, IBAN): configurabile tra "non salvare", "salva e cancella dopo N minuti" o "salva normalmente".
- **Pausa registrazione** con un click dalla menu bar (e scorciatoia opzionale). L'icona cambia stato quando è in pausa.
- **Tutto locale**: nessuna telemetria, nessuna connessione di rete di default. L'unica funzione di rete (titoli/favicon dei link) è opt-in.
- Opzione per cifrare il database (SQLCipher + Keychain).
- "Cancella tutta la cronologia" (esclusi i fissati, scelta con checkbox) con conferma.
- Esporta/Importa in JSON (e immagini in zip) per backup.

### 4.9 Menu bar e impostazioni
- **Icona nella menu bar** (template image, si adatta a chiaro/scuro) con menu: Apri cronologia, Pausa/Riprendi, Ultimi 5 elementi (click = incolla), Impostazioni…, Esci.
- **Finestra Impostazioni** (`⌘,`) con sezioni: Generale (avvio al login, lingua IT/EN, tema Chiaro/Scuro/Auto, suono alla copia), Scorciatoie, Aspetto (layout pannello, dimensione card, colore d'accento, densità), Cronologia (limiti, scadenze), Privacy (app escluse, dati sensibili, rete, cifratura), Tag e Spaces, Dati (esporta/importa/cancella), Info (versione, licenze).
- **Onboarding al primo avvio** (3 schermate): benvenuto → richiesta permesso Accessibilità con spiegazione e pulsante che apre le Impostazioni di Sistema → prova della scorciatoia (con possibilità di cambiarla subito).
- Localizzazione **italiano e inglese** (default: lingua del sistema), stringhe in file i18n.

---

## 5. Design e grafica (priorità alta)

Obiettivo: sembrare un'app **nativa Apple di alta qualità**, non una pagina web in una finestra.

- **Materiale**: effetto vetro con `window-vibrancy` (`NSVisualEffectMaterial::HudWindow` o `UnderWindowBackground`), angoli arrotondati, ombra morbida, bordo sottile semitrasparente (1px, bianco al 10–15% in scuro). Tema **chiaro e scuro** che segue il sistema, con override manuale.
- **Tipografia**: `-apple-system` / SF Pro, base 13 px, titoli 11 px maiuscoletto con spaziatura, **SF Mono** (fallback `JetBrains Mono`) per il codice.
- **Colori**: palette neutra con un colore d'accento (default indaco `#6366F1`, personalizzabile; opzione "usa il colore d'accento di macOS"). Definisci tutto come **design token** (CSS variables) per tema chiaro/scuro.
- **Card** (default 230×250 px, raggio 16 px): intestazione di ~38 px **colorata col colore dominante dell'app di origine** con icona 20 px e nome app; corpo con anteprima specifica per tipo (sezione 4.2); piede con "2 min fa" a sinistra e badge `⌘1` a destra; caratteri/peso in piccolo. Card selezionata: anello d'accento 2 px + leggero `scale(1.02)`. Fissata: piccola puntina in alto a destra. Tag: pallini colorati sotto l'anteprima.
- **Sidebar** (collassabile con un pulsante in alto a sinistra): voci con pallino colorato, nome, contatore; sezione Tag; pulsante "+ Nuovo Space".
- **Barra superiore**: campo di ricerca centrale ampio con icona lente e chip filtro, a destra icone app di origine (filtri), selettore tipo, pulsante impostazioni.
- **Animazioni** (Framer Motion, molle morbide): il pannello scende con slide+fade in ~200 ms; le card entrano con leggero stagger (solo le prime ~10); transizioni fluide su selezione, ricerca (layout animation), eliminazione. Rispetta `prefers-reduced-motion`.
- **Stati vuoti curati** (nessun elemento, nessun risultato di ricerca, cronologia in pausa) con piccola illustrazione SVG originale e testo amichevole.
- **Feedback**: piccolo toast non invasivo ("Copiato", "Elemento eliminato — Annulla").
- **Accessibilità**: navigazione completa da tastiera, etichette ARIA/VoiceOver, contrasto AA, dimensioni testo che seguono le impostazioni.
- Icona dell'app e icona menu bar **originali** (generale la icon set in tutte le dimensioni richieste da Tauri).

---

## 6. Scorciatoie da tastiera

| Tasto | Azione |
|---|---|
| `⌘⇧V` (globale, modificabile) | Apri/chiudi pannello |
| `←` `→` / `↑` `↓` | Naviga tra gli elementi |
| `Invio` | Incolla |
| `⇧Invio` | Incolla come testo semplice |
| `⌘1`…`⌘9` | Incolla l'n-esimo elemento visibile |
| `⇧←` / `⇧→` | Selezione multipla |
| `Spazio` | Anteprima grande |
| `⌘F` o digitare | Ricerca |
| `Esc` | Svuota ricerca / chiudi anteprima / chiudi pannello |
| `⌘D` | Fissa / togli |
| `⌘T` | Tag |
| `⌘M` | Sposta in uno Space |
| `⌘E` | Modifica testo |
| `⌘K` | Menu azioni (trasformazioni) |
| `⌘⌫` | Elimina (annullabile con `⌘Z`) |
| `Tab` / `⇧Tab` | Cambia Space |
| `⌘B` | Mostra/nascondi sidebar |
| `⌘,` | Impostazioni |

---

## 7. Schema database (SQLite)

```sql
CREATE TABLE spaces (
  id INTEGER PRIMARY KEY, name TEXT NOT NULL, color TEXT, icon TEXT, position INTEGER DEFAULT 0
);

CREATE TABLE items (
  id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL,                 -- text|rich|link|image|file|color|code
  content_hash TEXT NOT NULL UNIQUE,
  text_content TEXT,                  -- testo normalizzato per anteprima e ricerca
  rich_rtf BLOB, rich_html TEXT,
  file_paths TEXT,                    -- JSON array
  image_path TEXT, thumb_path TEXT, width INTEGER, height INTEGER,
  byte_size INTEGER,
  source_bundle_id TEXT, source_app_name TEXT, source_color TEXT,
  link_title TEXT, link_favicon_path TEXT,
  language TEXT,                      -- per kind=code
  ocr_text TEXT,
  pinned INTEGER NOT NULL DEFAULT 0,
  space_id INTEGER REFERENCES spaces(id) ON DELETE SET NULL,
  use_count INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL, last_used_at INTEGER NOT NULL,
  expires_at INTEGER                  -- per dati sensibili con scadenza
);
CREATE INDEX idx_items_last_used ON items(last_used_at DESC);
CREATE INDEX idx_items_space ON items(space_id);
CREATE INDEX idx_items_kind ON items(kind);

CREATE VIRTUAL TABLE items_fts USING fts5(
  text_content, ocr_text, link_title, source_app_name,
  content='items', content_rowid='id',
  tokenize='unicode61 remove_diacritics 2', prefix='2 3'
);
-- + trigger AFTER INSERT/UPDATE/DELETE per tenere items_fts sincronizzata

CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, color TEXT);
CREATE TABLE item_tags (
  item_id INTEGER REFERENCES items(id) ON DELETE CASCADE,
  tag_id  INTEGER REFERENCES tags(id)  ON DELETE CASCADE,
  PRIMARY KEY (item_id, tag_id)
);
CREATE TABLE excluded_apps (bundle_id TEXT PRIMARY KEY, name TEXT);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
```
Usa migrazioni versionate (`PRAGMA user_version`). Le immagini originali e le miniature stanno su disco in `~/Library/Application Support/<bundle-id>/`, non nel DB.

---

## 8. Interfaccia IPC (comandi ed eventi)

Comandi Tauri (tipizzati end-to-end, genera i tipi TS dai tipi Rust se possibile, es. `specta`/`ts-rs`):
`get_items(query, filters, sort, offset, limit)`, `get_item(id)`, `paste_item(id, plain)`, `paste_many(ids, separator, plain)`, `copy_item(id)`, `pin_item(id, value)`, `delete_items(ids)`, `undo_delete()`, `update_item_text(id, text)`, `move_to_space(ids, space_id)`, `create_space/rename_space/delete_space/reorder_spaces`, `create_tag/update_tag/delete_tag/merge_tags`, `set_item_tags(id, tag_ids)`, `run_action(id, action)`, `list_source_apps()`, `get_settings/set_setting`, `set_shortcut(accelerator)`, `check_accessibility()/request_accessibility()`, `add_excluded_app/remove_excluded_app`, `pause_monitoring(value)`, `export_data(path)/import_data(path)`, `clear_history(keep_pinned)`.

Eventi Rust → frontend: `clipboard:new-item`, `panel:shown`, `panel:hidden`, `monitoring:state`, `settings:changed`, `item:deleted`.

---

## 9. Prestazioni

- Memoria a riposo < 100 MB, CPU idle < 1%.
- Liste **virtualizzate** (orizzontale e verticale), immagini caricate in modo pigro, miniature generate in Rust fuori dal thread UI.
- Query paginate (es. 60 elementi per volta) con scroll infinito.
- Avvio app < 1 s; apertura pannello percepita istantanea.
- Nessun lavoro pesante sul thread principale.

---

## 10. Permessi e dettagli macOS da gestire

- **Accessibilità** (per simulare `⌘V`): controlla con `AXIsProcessTrustedWithOptions`, mostra l'onboarding e link diretto a *Impostazioni di Sistema → Privacy e sicurezza → Accessibilità*. Ricontrolla il permesso quando l'app torna attiva. Nota: in sviluppo (`tauri dev`) il permesso va dato al terminale/IDE da cui parte, in release all'app.
- **Info.plist**: `LSUIElement = true` (nessun Dock), eventuali descrizioni d'uso necessarie.
- **Avvio al login** tramite plugin autostart (opzione, default spenta).
- **Scorciatoia in conflitto**: `⌘⇧V` in alcune app (es. Chrome) significa "incolla senza formattazione". Registrandola come globale la sovrascrive: tienila come default come richiesto, ma rendi evidente nelle impostazioni come cambiarla e avvisa nell'onboarding.
- Gestione di più schermi, Stage Manager, app a schermo intero e cambio di Space di macOS.

---

## 11. Cose da NON fare

- Non usare Electron né wrapper pesanti.
- Non fare polling di rete o chiamate a servizi esterni senza opt-in esplicito.
- Non salvare contenuti segnalati come concealed/transient.
- Non rubare il focus all'app in primo piano all'apertura del pannello.
- Non copiare nome, icone, grafiche o testi di Pasty o di altre app.
- Non lasciare funzioni "a metà" senza dichiararlo in `DECISIONS.md`.

---

## 12. Piano a fasi (con criteri di accettazione)

**Fase 0 — Setup.** Verifica ambiente (Rust stable, Node ≥ 20, pnpm, Xcode Command Line Tools), crea il progetto Tauri 2 + React/TS/Vite/Tailwind, configura `macOSPrivateApi`, Accessory policy, struttura cartelle, `DECISIONS.md`. *Accettazione:* `pnpm tauri dev` apre una finestra vuota senza icona nel Dock.

**Fase 1 — Cuore: monitor + database.** Monitor NSPasteboard, rilevamento tipi e app di origine, hash e dedup, schema SQLite + FTS5 + migrazioni, salvataggio immagini/miniature. Test unitari (dedup, FTS, rilevamento tipo). *Accettazione:* copiando testo/immagine/file/link in qualunque app, compaiono righe corrette nel DB (log o comando di debug) senza duplicati e senza loop.

**Fase 2 — Pannello e scorciatoia.** NSPanel non attivante, vibrancy, scorciatoia globale `⌘⇧V` con toggle, posizionamento sullo schermo del cursore, tre layout, apertura rapida. Lista base delle card. *Accettazione:* la scorciatoia apre il pannello sopra qualsiasi app (anche a schermo intero) senza togliere il focus; `Esc` e click fuori lo chiudono.

**Fase 3 — Incolla e permessi.** Scrittura appunti, simulazione `⌘V`, incolla plain text, `⌘1…9`, onboarding Accessibilità, fallback senza permesso. *Accettazione:* da TextEdit, Note e Chrome, selezionare una card e premere Invio incolla nel punto del cursore.

**Fase 4 — Interfaccia completa.** Card per ogni tipo, sidebar, barra di ricerca con sintassi filtri, filtri per app e tipo, navigazione da tastiera, anteprima grande, animazioni, tema chiaro/scuro, stati vuoti. *Accettazione:* il pannello ha l'aspetto descritto in sezione 5; ricerca istantanea su 10.000 elementi di test.

**Fase 5 — Organizzazione.** Spaces, tag (`⌘T`), fissa, sposta, modifica, elimina con annulla, selezione multipla e incolla multiplo, menu azioni `⌘K`, menu contestuale. *Accettazione:* tutti i flussi funzionano solo con tastiera e con mouse.

**Fase 6 — Privacy, tray, impostazioni.** Esclusioni e flag nspasteboard, rilevamento dati sensibili con scadenze, pausa, icona menu bar con ultimi elementi, finestra impostazioni completa, avvio al login, cifratura DB opzionale, export/import, pulizia automatica. *Accettazione:* una password copiata da un password manager non finisce mai nel DB; le impostazioni persistono.

**Fase 7 — Rifinitura.** i18n IT/EN, accessibilità, performance (profilazione con 50k elementi), icone originali, onboarding finale, gestione errori e log. Extra se c'è tempo: OCR delle immagini con Vision framework (testo ricercabile), titolo/favicon link (opt-in), trascinamento fuori dalla card.

**Fase 8 — Release.** `pnpm tauri build` con target **universal** (`--target universal-apple-darwin`), `.app` + `.dmg`; spiega i passi per firma Developer ID e notarizzazione (opzionali per uso personale: indica come aprire un'app non firmata). README con installazione, permessi e scorciatoie.

---

## 13. Qualità e test

- Rust: test unitari su dedup, FTS e sintassi filtri, rilevamento tipo, rilevamento dati sensibili, azioni di trasformazione, retention/pulizia. `cargo clippy` e `cargo fmt` senza warning.
- Frontend: TypeScript strict, ESLint, qualche test (Vitest) su `search-syntax` e store.
- Test manuale documentato in `TESTING.md` con checklist per: app a schermo intero, più monitor, permesso Accessibilità negato, password manager, immagini grandi, 50k elementi.

## 14. Definizione di "finito"

- Tutte le funzioni delle sezioni 4–6 funzionano e sono raggiungibili da tastiera.
- `⌘⇧V` apre il pannello di default e si può cambiare.
- Nessun dato lascia il computer senza opt-in.
- L'app si installa da `.dmg`, si avvia al login se scelto e non pesa su CPU e memoria.
- Aspetto curato, coerente e fluido in chiaro e scuro.
- README, DECISIONS.md e TESTING.md presenti e aggiornati.
