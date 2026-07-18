# Neothesia Cyma - Piano di implementazione

## Stato e vincoli verificati

- Upstream: `PolyMeilex/Neothesia`, branch di base `master`, commit `bb3be49`, licenza GPL-3.0.
- Branch di lavoro: `feature/neothesia-cyma-phase-3`.
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
3. **Campo Chladni e renderer WGSL 2D - completata il 18 luglio 2026**
   - Convertire lo stato armonico in massimo 12 componenti modali e renderizzare il campo con una pipeline wgpu dedicata e fallback GPU sicuro.
   - Implementare il mapping puro e deterministico in `cyma-core/src/modal.rs`, senza dipendenze GPU o UI.
   - Implementare pipeline, uniform allineati e shader nativo in `neothesia-core/src/render/cyma/{mod.rs,shader.wgsl}`, riutilizzando `wgpu_jumpstart::{Shape,Uniform}`.
   - Possedere il renderer lazy in `neothesia/src/context.rs`, aggiornandolo solo quando Cyma è attivo e rendendone visibile lo stato nel pannello `neothesia/src/scene/menu_scene/settings.rs`.
   - Registrare il draw del campo in `neothesia/src/main.rs` prima della scena attiva, così resta uno sfondo 2D e non richiede modifiche al trait `Scene`.
4. **Geometria 3D, particelle e prestazioni - non iniziata**
   - Aggiungere solo dopo la validazione del renderer 2D; includere preset di qualità, resize, misure CPU/GPU e degradazione controllata.

## Fondamento fisico e trasformazioni artistiche

- Fondamento fisico implementato: funzioni proprie separabili `sin(m·π·x)·sin(n·π·y)` di una membrana rettangolare ideale con bordo fisso; gli zeri del campo composto generano linee nodali di tipo Chladni.
- Trasformazioni artistiche: associazione pitch class-colore, tabella pitch class-coppia modale, guadagno di fase, superposizione dei modi, spessore delle linee guidato dalla tensione e luminosità guidata dalla consonanza. Non costituiscono una simulazione quantitativa di una piastra reale, perché non includono materiale, spessore, smorzamento, eccitazione o condizioni al contorno calibrate.

## Verifica corrente

- `cargo fmt --all -- --check`: superato.
- `cargo check -p cyma-core -p neothesia-core -p neothesia --offline`: superato su Windows GNU.
- `cargo test -p cyma-core -p neothesia-core -p neothesia --offline`: superato, 39 test totali (26 `cyma-core`, 8 `neothesia`, 5 `neothesia-core`).
- I test renderer verificano parsing e validazione Naga del WGSL, dimensione/allineamento/padding dell'uniform e un draw off-screen 64×64 con readback GPU non nero per C-E-G; il test GPU è superato su Intel Iris Xe/Vulkan.
- `cargo clippy -p cyma-core --all-targets -- -D warnings`: superato senza warning.
- Il Clippy mirato su `neothesia-core` e `neothesia`, ripetuto consentendo esclusivamente `dead_code` e `unused_mut` già presenti upstream, è superato senza nuovi warning Cyma.
- `cargo run -p neothesia`: avvio riuscito, GPU Intel Iris Xe inizializzata tramite Vulkan, processo responsivo e chiusura standard riuscita con exit code 0.
- `cargo check --workspace` e `cargo test --workspace`: non completabili per `ffmpeg-sys-next`, che richiede `pkg-config` e `libavutil` non installati. Il blocco riguarda `ffmpeg-encoder`/CLI e non i crate modificati.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: non completabile perché `fluidlite-sys` non trova `gcc.exe` e l'header C `string.h`. Resta inoltre il warning upstream già verificato in `midi-file/examples/play.rs` (`useless_borrows_in_formatting` per `&out_ports[0]`) quando la build riesce a raggiungere quell'esempio.
- Clippy e build sui crate applicativi riportano inoltre warning upstream preesistenti: `home` e `xdg_config` inutilizzate su Windows in `neothesia-core/src/utils/resources.rs`, e `mut` non necessario per `attributes` in `neothesia/src/main.rs`. Non sono stati corretti perché estranei alla fase.
- Verifiche manuali riuscite: avvio normale, caricamento da riga di comando di un MIDI temporaneo, ingresso in Free Play, inizializzazione con configurazione Cyma temporanea attiva, responsività e chiusura pulita. L'automazione Windows non ha mantenuto note attive né avviato la Playing Scene in modo abbastanza affidabile per una validazione visiva completa; il draw reale è stato quindi verificato con il test GPU off-screen.
- Verifiche manuali non eseguibili: tastiera MIDI hardware, click sul toggle Cyma, resize durante un campo attivo, ritorno al menu dalla Playing Scene e misura visiva dei 60 FPS.

## Esito fase 3

- `cyma-core/src/modal.rs` converte lo stato armonico interpolato in un array fisso di massimo 12 `ModalComponent`, ordinati per pitch class e privi di allocazioni. Valori non finiti vengono neutralizzati prima di raggiungere il renderer.
- Il mapping modale è deterministico e il campo non usa tempo globale o casualità: a parità di stato armonico e viewport produce gli stessi parametri e gli stessi pixel.
- `neothesia-core/src/render/cyma/shader.wgsl` calcola nel fragment shader una superposizione di modi di membrana e mette in evidenza le linee nodali. Il loop termina al numero di componenti attive e non supera mai 12 iterazioni.
- L'uniform Rust/WGSL occupa 240 byte, ha allineamento 16 e offset verificati (`viewport` 0, `color` 16, `metrics` 32, `modes` 48). Usa esclusivamente campi `vec4<f32>` compatibili con `bytemuck`.
- Il renderer viene creato in modo lazy quando Cyma è attivato. La creazione di shader, uniform e pipeline è protetta da error scope wgpu per validation, internal e out-of-memory; in caso di errore l'analisi armonica resta disponibile, il draw viene saltato e la UI mostra `GPU unavailable`.
- Il renderer pronto è conservato in un `Box` allocato una sola volta per evitare un enum applicativo sovradimensionato. Nel percorso per frame non vengono creati `Vec`, lock o altre allocazioni esplicite: viene aggiornato un buffer fisso da 240 byte e registrato un solo draw full-screen.
- Il draw Cyma viene registrato nel pass principale prima della scena attiva, quindi rimane dietro tastiera, waterfall e UI senza modificare il trait `Scene` o duplicare i renderer delle scene.
- Compromesso prestazionale: la fase 3 renderizza direttamente alla risoluzione della surface. Riduzione di risoluzione e preset di qualità restano nella fase 4; il costo corrente scala con i pixel e con il numero di pitch class attive.
- Cyma disattivato non crea la pipeline GPU al primo avvio, non aggiorna uniform e non registra draw; il comportamento standard di Neothesia resta invariato.

## File modificati nella fase 3

- Workspace: `Cargo.lock`.
- Mapping puro: `cyma-core/src/{lib,modal}.rs`.
- Dipendenze e export renderer: `neothesia-core/Cargo.toml` e `neothesia-core/src/render/mod.rs`.
- Renderer 2D: `neothesia-core/src/render/cyma/{mod.rs,shader.wgsl}`.
- Adattatore e integrazione applicativa: `neothesia/src/{cyma,context,main}.rs`.
- Stato renderer nella UI: `neothesia/src/scene/menu_scene/settings.rs`.
- Documentazione: `docs/CYMA_IMPLEMENTATION_PLAN.md`.

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
