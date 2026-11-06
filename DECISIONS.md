# Decisioni

Una riga per scelta non specificata nel brief: **scelta** — motivo.

## Fase 0
- **Progetto in `clipboard_custom/clipstack/`** — rispetta la struttura `clipstack/` della sezione 3; la cartella esterna resta il contenitore di lavoro.
- **Rust installato via rustup (stable 1.99.1) + target `x86_64-apple-darwin`** — Rust mancava; il target Intel serve per il binario universale (Fase 8).
- **Tailwind CSS v4 con `@tailwindcss/vite`** — è la versione corrente; niente `tailwind.config.js`, i token stanno in CSS variables (`src/styles/index.css`).
- **Tauri 2.12.5, Rust edition 2024** — versioni generate da `create-tauri-app`.
- **Dock nascosto sia con `ActivationPolicy::Accessory` sia con `LSUIElement` in `src-tauri/Info.plist`** — la policy agisce a runtime (anche in `tauri dev`), il plist evita il lampeggio dell'icona all'avvio dell'app bundle.
- **Finestra `panel` provvisoria (900×340, trasparente, senza decorazioni, centrata)** — serve solo per l'accettazione della Fase 0; in Fase 2 diventa un NSPanel non attivante.
- **Bundle target `app` + `dmg`** — gli unici utili su macOS.
- **CSP restrittiva da subito** (`default-src 'self'`, immagini da `asset:`/`data:`/`blob:`) — l'app non deve fare richieste di rete; verrà allargata solo per l'opzione opt-in favicon/titoli.
- **Moduli Rust della sezione 3 creati come file vuoti documentati** — scheletro della struttura; ognuno viene implementato nella fase indicata, nessuna funzione finta.
