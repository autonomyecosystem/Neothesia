# Neothesia Cyma - Piano di implementazione

## Stato e vincoli verificati

- Upstream: `PolyMeilex/Neothesia`, branch di base `master`, commit `bb3be49`, licenza GPL-3.0.
- Branch di lavoro: `feature/neothesia-cyma-phase-2`.
- Target operativo della sessione: Windows x86_64 GNU; il core resta privo di dipendenze specifiche di piattaforma.
- Il workspace è definito in `Cargo.toml`; l'app usa `neothesia/Cargo.toml` e il rendering/config condivisi sono in `neothesia-core/Cargo.toml`.
- Flusso MIDI reale: `neothesia/src/input_manager/mod.rs` normalizza Note On con velocity zero, invia `NeothesiaEvent::MidiInput` a `neothesia/src/main.rs`, che inoltra l'evento alla `Scene` attiva definita in `neothesia/src/scene/mod.rs`.
- Free Play resta in `neothesia/src/scene/freeplay/mod.rs` e usa il riconoscitore armonico estratto in `cyma-core`, condiviso anche dal runtime Cyma.
- Il frame loop è in `neothesia/src/main.rs`: `RedrawRequested` chiama `update` e poi `render`; le scene preparano risorse in `update` e registrano draw call in `render`.
- La configurazione RON persistente è in `neothesia-core/src/config/{mod.rs,model.rs}` ed è salvata dal `Drop` di `neothesia/src/context.rs`.

## Fasi

1. **Fondazione musicale pura - completata il 17 luglio 2026**
   - Aggiungere il crate workspace `cyma-core`, senza wgpu, winit o UI.
   - Estrarre e riutilizzare il chord detector di Free Play, conservandone i risultati esistenti.
   - Implementare stato MIDI deterministico, pesi per pitch class, colore composto, consonanza/tensione e interpolazione temporale senza allocazioni nel percorso di aggiornamento.
   - Aggiungere una configurazione Cyma serializzabile, versionata dall'host e disattivata per impostazione predefinita.
   - Coprire tutti i test unitari puri richiesti.
2. **Integrazione applicativa e impostazioni - completata il 18 luglio 2026**
   - Collegare lo stato armonico al flusso MIDI esistente senza creare una seconda connessione o coda MIDI.
   - Esporre attivazione/disattivazione e parametri minimi nel pannello impostazioni, mantenendo costo nullo quando disattivato.
   - Usare `neothesia/src/cyma.rs` come adattatore tra `midly` e `cyma-core`, posseduto opzionalmente da `neothesia/src/context.rs`.
   - Osservare l'input utente in `neothesia/src/main.rs` e gli eventi file effettivamente emessi in `neothesia/src/scene/playing_scene/midi_player.rs`.
   - Tenere separati stato utente e stato file, azzerando solo quest'ultimo su pausa, seek, rewind, loop e cambio scena.
   - Aggiungere nel pannello `neothesia/src/scene/menu_scene/settings.rs` toggle, accordo rilevato e tempo di risposta.
3. **Campo Chladni e renderer WGSL 2D - non iniziata**
   - Convertire lo stato armonico in massimo 12 componenti modali e renderizzare il campo con una pipeline wgpu dedicata e fallback GPU sicuro.
4. **Geometria 3D, particelle e prestazioni - non iniziata**
   - Aggiungere solo dopo la validazione del renderer 2D; includere preset di qualità, resize, misure CPU/GPU e degradazione controllata.

## Fondamento fisico e trasformazioni artistiche

- Fondamento fisico previsto: uso di funzioni modali di onde stazionarie e linee nodali ispirate a membrane/piastre di Chladni.
- Trasformazioni artistiche: associazione pitch class-colore, pesatura armonica, mapping nota-modo e indici di consonanza/tensione. Non saranno presentati come simulazione quantitativa di una piastra reale senza parametri materiali e condizioni al contorno calibrate.

## Verifica corrente

- `cargo fmt --all -- --check`: superato.
- `cargo check -p cyma-core -p neothesia-core -p neothesia --offline`: superato su Windows GNU.
- `cargo test -p cyma-core -p neothesia-core -p neothesia --offline`: superato, 32 test totali (22 `cyma-core`, 8 `neothesia`, 2 configurazione persistente in `neothesia-core`).
- `cargo clippy -p cyma-core --all-targets -- -D warnings`: superato senza warning.
- Il Clippy mirato su `neothesia-core` e `neothesia`, ripetuto consentendo esclusivamente `dead_code` e `unused_mut` già presenti upstream, è superato senza nuovi warning Cyma.
- `cargo run -p neothesia`: avvio riuscito, GPU Intel Iris Xe inizializzata tramite Vulkan, processo responsivo e chiusura standard riuscita con exit code 0.
- `cargo check --workspace` e `cargo test --workspace`: non completabili per `ffmpeg-sys-next`, che richiede `pkg-config` e `libavutil` non installati. Il blocco riguarda `ffmpeg-encoder`/CLI e non i crate modificati.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: si arresta prima su un warning upstream in `midi-file/examples/play.rs` (`useless_borrows_in_formatting` per `&out_ports[0]`). Consentendo solo quel lint, raggiunge poi `fluidlite-sys`, che non trova `gcc.exe` e l'header C `string.h`.
- Clippy e build sui crate applicativi riportano inoltre warning upstream preesistenti: `home` e `xdg_config` inutilizzate su Windows in `neothesia-core/src/utils/resources.rs`, e `mut` non necessario per `attributes` in `neothesia/src/main.rs`. Non sono stati corretti perché estranei alla fase.
- Verifiche manuali non eseguibili in questa sessione: caricamento e interazione con un file MIDI, navigazione Free Play, tastiera MIDI hardware, attivazione/disattivazione del toggle Cyma, resize e ritorno al menu. Avvio, inizializzazione GPU, responsività della finestra e chiusura pulita sono stati verificati.

## Esito fase 2

- Il runtime applicativo è un `Option`: quando Cyma è disattivato non conserva un `MidiState`, non analizza eventi e il costo per frame è limitato a un controllo del valore opzionale.
- Gli stati MIDI `User` e `File` restano separati e vengono combinati solo nell'analisi armonica pura. Le percussioni sul canale MIDI 10 (indice 9) sono escluse.
- Gli eventi MIDI utente sono osservati nel dispatcher globale già esistente; non sono state aggiunte connessioni MIDI, code o duplicazioni della gestione input.
- Gli eventi del file sono osservati in `MidiPlayer` solo quando vengono realmente inoltrati all'output. In modalità Human le note richieste restano rappresentate dall'input `User`, mentre gli eventi non-nota effettivamente emessi continuano a essere osservati.
- Una modifica MIDI marca il runtime come `dirty`; la scansione fissa di 16 canali × 128 note e l'analisi armonica avvengono al massimo una volta per frame, senza lock e senza allocazioni heap nel percorso di aggiornamento.
- Pausa, seek, rewind, loop, cambio scena e preview azzerano esclusivamente lo stato `File`, preservando le note dell'utente. Compromesso: dopo un seek non viene ricostruita retroattivamente una nota iniziata prima della nuova posizione; lo stato riparte dagli eventi successivamente emessi, evitando note bloccate e scansioni aggiuntive.
- Il pannello impostazioni espone toggle Cyma, accordo rilevato e tempo di risposta deterministico da 0 a 2000 ms con passo di 20 ms. Il valore predefinito resta retrocompatibile e Cyma resta disattivato.
- Non sono stati aggiunti renderer, shader o tipi wgpu: la separazione tra logica musicale e rendering resta intatta e la fase 3 non è stata avviata.

## File modificati nella fase 2

- Core musicale: `cyma-core/src/{config,harmony,lib,midi}.rs`.
- Configurazione persistente: `neothesia-core/src/config/mod.rs`.
- Adattatore e contesto applicativo: `neothesia/src/cyma.rs` e `neothesia/src/{context,main}.rs`.
- Free Play e preview registrazione: `neothesia/src/scene/freeplay/{mod,recorder}.rs`.
- Impostazioni: `neothesia/src/scene/menu_scene/settings.rs`.
- Riproduzione file e controlli: `neothesia/src/scene/playing_scene/{mod,midi_player}.rs` e `neothesia/src/scene/playing_scene/top_bar/mod.rs`.
- Documentazione: `docs/CYMA_IMPLEMENTATION_PLAN.md`.

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
