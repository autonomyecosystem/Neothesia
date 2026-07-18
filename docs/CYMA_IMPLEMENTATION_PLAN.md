# Neothesia Cyma - Piano di implementazione

## Stato e vincoli verificati

- Upstream: `PolyMeilex/Neothesia`, branch di base `master`, commit `bb3be49`, licenza GPL-3.0.
- Branch di lavoro: `feature/neothesia-cyma-phase-1`.
- Target operativo della sessione: Windows x86_64 GNU; il core resta privo di dipendenze specifiche di piattaforma.
- Il workspace è definito in `Cargo.toml`; l'app usa `neothesia/Cargo.toml` e il rendering/config condivisi sono in `neothesia-core/Cargo.toml`.
- Flusso MIDI reale: `neothesia/src/input_manager/mod.rs` normalizza Note On con velocity zero, invia `NeothesiaEvent::MidiInput` a `neothesia/src/main.rs`, che inoltra l'evento alla `Scene` attiva definita in `neothesia/src/scene/mod.rs`.
- Free Play e chord detection sono in `neothesia/src/scene/freeplay/mod.rs`; il riconoscitore è oggi un modulo privato della scena.
- Il frame loop è in `neothesia/src/main.rs`: `RedrawRequested` chiama `update` e poi `render`; le scene preparano risorse in `update` e registrano draw call in `render`.
- La configurazione RON persistente è in `neothesia-core/src/config/{mod.rs,model.rs}` ed è salvata dal `Drop` di `neothesia/src/context.rs`.

## Fasi

1. **Fondazione musicale pura - completata il 17 luglio 2026**
   - Aggiungere il crate workspace `cyma-core`, senza wgpu, winit o UI.
   - Estrarre e riutilizzare il chord detector di Free Play, conservandone i risultati esistenti.
   - Implementare stato MIDI deterministico, pesi per pitch class, colore composto, consonanza/tensione e interpolazione temporale senza allocazioni nel percorso di aggiornamento.
   - Aggiungere una configurazione Cyma serializzabile, versionata dall'host e disattivata per impostazione predefinita.
   - Coprire tutti i test unitari puri richiesti.
2. **Integrazione applicativa e impostazioni - non iniziata**
   - Collegare lo stato armonico al flusso MIDI esistente senza creare una seconda connessione o coda MIDI.
   - Esporre attivazione/disattivazione e parametri minimi nel pannello impostazioni, mantenendo costo nullo quando disattivato.
3. **Campo Chladni e renderer WGSL 2D - non iniziata**
   - Convertire lo stato armonico in massimo 12 componenti modali e renderizzare il campo con una pipeline wgpu dedicata e fallback GPU sicuro.
4. **Geometria 3D, particelle e prestazioni - non iniziata**
   - Aggiungere solo dopo la validazione del renderer 2D; includere preset di qualità, resize, misure CPU/GPU e degradazione controllata.

## Fondamento fisico e trasformazioni artistiche

- Fondamento fisico previsto: uso di funzioni modali di onde stazionarie e linee nodali ispirate a membrane/piastre di Chladni.
- Trasformazioni artistiche: associazione pitch class-colore, pesatura armonica, mapping nota-modo e indici di consonanza/tensione. Non saranno presentati come simulazione quantitativa di una piastra reale senza parametri materiali e condizioni al contorno calibrate.

## Verifica corrente

- `cargo fmt --all -- --check`: superato.
- `cargo metadata --no-deps --format-version 1`: superato.
- `cargo check -p cyma-core`, `cargo check -p neothesia-core` e `cargo check -p neothesia`: superati su Windows GNU.
- `cargo test -p cyma-core -p neothesia-core -p neothesia`: superato, 24 test totali (19 Cyma, 2 configurazione persistente, 3 recorder Free Play).
- `cargo clippy -p cyma-core --all-targets -- -D warnings`: superato senza warning.
- `cargo run -p neothesia`: avvio riuscito, GPU Intel Iris Xe inizializzata tramite Vulkan, processo responsivo e chiusura standard riuscita con exit code 0.
- `cargo check --workspace` e `cargo test --workspace`: non completabili per `ffmpeg-sys-next`, che richiede `pkg-config` e `libavutil` non installati. Il blocco riguarda `ffmpeg-encoder`/CLI e non i crate modificati.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: non completabile perché `fluidlite-sys` richiede `gcc.exe` e gli header C (`string.h`) non presenti.
- Clippy sui crate applicativi modificati raggiunge inoltre warning upstream preesistenti: `home` e `xdg_config` inutilizzate su Windows in `neothesia-core/src/utils/resources.rs`, e `mut` non necessario per `attributes` in `neothesia/src/main.rs`. Non sono stati mascherati né corretti perché estranei alla fase.
- Verifiche manuali non eseguibili in questa sessione: caricamento/interazione con file MIDI, navigazione Free Play, tastiera MIDI hardware, resize/menu e controllo visivo. Il toggle Cyma non esiste ancora per scelta di fase e sarà introdotto nella fase 2.

## Esito fase 1

- `cyma-core` usa strutture fisse per 16 canali, 128 note e 12 pitch class; l'aggiornamento MIDI/armonico non alloca sullo heap.
- Note On con velocity zero, Note Off, sustain per canale e Note On duplicate sono gestiti deterministicamente con errori espliciti per input fuori intervallo o overflow.
- Il chord detector Free Play è stato estratto senza cambiare le stringhe prodotte; i test originali sono stati trasferiti ed estesi nel crate puro.
- Il colore usa una palette artistica sul circolo delle quinte. Consonanza e tensione sono euristiche percettive per classi intervallari, non misure fisiche.
- La configurazione Cyma è serializzabile, inserita nel modello RON versionato e disattivata per impostazione predefinita. Non è ancora esposta nella UI né collegata al flusso live.
- Compromesso Windows: per i controlli è stato usato temporaneamente `binutils` MSYS2 verificato tramite checksum, perché il profilo Rust GNU non forniva un assembler utilizzabile. Nessun binario è stato aggiunto al repository.

## File modificati nella fase 1

- Workspace e regole: `AGENTS.md`, `Cargo.toml`, `Cargo.lock`.
- Core Cyma: `cyma-core/Cargo.toml` e `cyma-core/src/{lib,pitch,midi,chord,color,harmony,config}.rs`.
- Integrazione configurazione: `neothesia-core/Cargo.toml` e `neothesia-core/src/config/{mod,model}.rs`.
- Riutilizzo Free Play: `neothesia/Cargo.toml` e `neothesia/src/scene/freeplay/mod.rs`.
- Documentazione: `docs/CYMA_IMPLEMENTATION_PLAN.md`.
