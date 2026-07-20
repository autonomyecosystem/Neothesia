# Neothesia Cyma - Piano di implementazione

## Stato e vincoli verificati

- Upstream: `PolyMeilex/Neothesia`, branch di base `master`, commit `bb3be49`, licenza GPL-3.0.
- Branch di lavoro: `feature/neothesia-cyma-phase-6`.
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
4. **Geometria 3D, particelle e prestazioni - completata il 18 luglio 2026**
   - Aggiungere solo dopo la validazione del renderer 2D; includere preset di qualità, resize, misure CPU/GPU e degradazione controllata.
   - Estendere `cyma-core/src/config.rs` con modalità 2D/3D, qualità Low/Medium/High e particelle opzionali, mantenendo 2D/Medium/particelle disattivate come default retrocompatibile.
   - Aggiungere campionamento e gradiente nodale puri in `cyma-core/src/modal.rs`, riutilizzati dalla simulazione particellare senza dipendenze GPU.
   - Suddividere il renderer in `neothesia-core/src/render/cyma/{mod,target,surface,particles,gpu_timer}.rs` con shader WGSL dedicati nella stessa directory.
   - Renderizzare Cyma su target off-screen scalati per preset, quindi comporre sulla surface; Low/Medium/High ridurranno risoluzione, modi massimi, densità mesh e particelle.
   - Richiedere `TIMESTAMP_QUERY` in `wgpu-jumpstart/src/gpu.rs` solo quando supportato dall'adapter; mostrare GPU `N/A` senza interrompere il renderer quando non disponibile.
   - Integrare opzioni e metriche in `neothesia/src/{context,main}.rs` e `neothesia/src/scene/menu_scene/settings.rs`, preservando il trait `Scene`.
5. **HUD educativo e validazione prestazionale Windows - completata il 18 luglio 2026**
   - Aggiungere a `cyma-core/src/config.rs` un toggle HUD disattivato per default e propagarlo tramite `neothesia-core/src/config/mod.rs` e il pannello `neothesia/src/scene/menu_scene/settings.rs`.
   - Creare un modello di presentazione testabile e un builder Nuon condiviso in `neothesia/src/cyma_hud.rs`, usato da `neothesia/src/scene/freeplay/mod.rs` e `neothesia/src/scene/playing_scene/mod.rs` senza duplicare logica armonica.
   - Mostrare accordo, pitch class attive, colore composto, consonanza/tensione, modalità/qualità e budget CPU/GPU, distinguendo esplicitamente euristiche artistiche e base modale fisicamente motivata.
   - Sostituire in `cyma-core/src/modal.rs` il gradiente nodale a differenze finite con un campionamento analitico valore+gradiente in un solo pass sui modi; `neothesia-core/src/render/cyma/particles.rs` riutilizzerà il risultato per attrazione e opacità.
   - Aggiungere probe prestazionali release ignorati di default in `cyma-core/tests/windows_performance.rs` e nei test particellari, eseguibili esplicitamente su Windows senza rendere flaky la suite standard.
   - Conservare il trait `Scene`, il flusso MIDI e il frame graph della fase 4; HUD disattivato e Cyma disattivato devono mantenere il comportamento upstream.
6. **Modalità Cyma autonoma, piastra quadrata e tastiera cromatica - completata il 19 luglio 2026**
   - Aggiungere `neothesia/src/scene/cyma_scene.rs` e `NeothesiaEvent::CymaMode` in `neothesia/src/main.rs`; la scena userà il renderer globale già registrato prima di `Scene::render`, la tastiera condivisa di `neothesia/src/scene/playing_scene/keyboard.rs` e l'HUD di `neothesia/src/cyma_hud.rs`, senza creare una seconda connessione MIDI. L'accesso sarà aggiunto a `neothesia/src/scene/menu_scene/{mod,state}.rs` e resterà nella finestra principale; una finestra Windows distaccata non fa parte di questa fase.
   - Esporre in `cyma-core/src/color.rs` la palette deterministica per singola pitch class e riutilizzarla in `neothesia/src/scene/playing_scene/keyboard.rs` per le note utente e file. I tasti attivi useranno i colori Cyma solo quando Cyma è abilitato; altrimenti resteranno identici all'upstream.
   - Interpretare `cyma-core/src/modal.rs` come piastra quadrata sottile idealmente appoggiata di lato `1 m`, area `1 m²`, usando coordinate fisiche `x/L` e `y/L`. Le forme modali separabili sono fisicamente motivate; associazione pitch class-modo, ampiezza, fase, colore e sovrapposizione restano trasformazioni artistiche.
   - Aggiornare `neothesia-core/src/render/cyma/{mod.rs,shader.wgsl,surface.wgsl}` senza cambiare dimensione o allineamento dell'uniform: `viewport.zw` conterrà i lati fisici della piastra, il renderer 2D manterrà un dominio quadrato centrato e la resa userà linee nodali granulari simili a sabbia; la mesh 3D conserverà proporzioni quadrate prima della proiezione.
   - Verificare con test puri palette, indipendenza dall'ottava, geometria `1 m × 1 m`, finitezza e determinismo; validare entrambi gli shader con Naga e provare manualmente su Windows accesso dal menu, input PC/MIDI, colori tasti, resize, ritorno al menu e chiusura pulita.

## Fondamento fisico e trasformazioni artistiche

- Fondamento fisico implementato: funzioni proprie separabili `sin(m·π·x/Lx)·sin(n·π·y/Ly)` di una piastra quadrata sottile idealmente appoggiata, con `Lx = Ly = 1 m`; gli zeri del campo composto generano linee nodali di tipo Chladni e la frequenza modale relativa scala con `m²/Lx² + n²/Ly²`.
- Trasformazioni artistiche: associazione pitch class-colore, tabella pitch class-coppia modale, guadagno di fase, superposizione dei modi, spessore delle linee guidato dalla tensione e luminosità guidata dalla consonanza. Non costituiscono una simulazione quantitativa di una piastra reale, perché non includono materiale, spessore, smorzamento, eccitazione o condizioni al contorno calibrate.

## Verifica corrente

- `cargo fmt --all -- --check`: superato dopo le modifiche della fase 6.
- `cargo check -p cyma-core -p neothesia-core -p neothesia --offline`: superato su Windows GNU usando toolchain e target temporanei verificati.
- Test mirati superati: 59 test, eseguiti come 34 test `cyma-core`, 12 test `neothesia-core --lib` e 13 test `neothesia --bin neothesia`; i due probe prestazionali manuali restano ignorati nella suite standard.
- I test renderer verificano parsing e validazione Naga dei quattro shader WGSL, dimensione/allineamento/padding dell'uniform, preset di qualità, mesh, particelle e timer; il draw off-screen 64×64 con compositing e readback GPU non nero per C-E-G è superato su Intel Iris Xe/Vulkan.
- `cargo clippy -p cyma-core --all-targets -- -D warnings`: superato senza warning.
- `cargo clippy -p wgpu-jumpstart --all-targets -- -D warnings`: superato senza warning.
- Il Clippy mirato su `neothesia-core` e `neothesia`, ripetuto consentendo esclusivamente `dead_code` e `unused_mut` già presenti upstream, è superato senza nuovi warning Cyma.
- I probe release Windows sono superati: aggiornamento armonico medio `0,011016 ms/update` e simulazione di 768 particelle High `0,530 ms/frame`.
- `cargo build -p neothesia --offline`: superato usando un target temporaneo scrivibile, necessario perché il `target` nel workspace Google Drive è marcato read-only.
- La verifica manuale DPI-aware della fase 5 resta superata per Free Play e Playing. Il tentativo isolato della fase 6 ha avviato un processo Neothesia stabile e responsivo, ma la sessione GUI elevata non ha pubblicato un handle finestra winit; accesso al nuovo pulsante, input PC/MIDI, resize e ritorno al menu non sono quindi dichiarati verificati manualmente in questa sessione.
- `cargo check --workspace` e `cargo test --workspace`: non completabili per `ffmpeg-sys-next`; il build script non trova il comando `pkg-config` necessario per `libavutil`. Il blocco riguarda `ffmpeg-encoder`/CLI e non i crate modificati.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: non completabile perché `fluidlite-sys` non trova `gcc.exe` e l'header C `string.h`. Resta inoltre il warning upstream già verificato in `midi-file/examples/play.rs` (`useless_borrows_in_formatting` per `&out_ports[0]`) quando la build riesce a raggiungere quell'esempio.
- Clippy e build sui crate applicativi riportano inoltre warning upstream preesistenti: `home` e `xdg_config` inutilizzate su Windows in `neothesia-core/src/utils/resources.rs`, e `mut` non necessario per `attributes` in `neothesia/src/main.rs`. Non sono stati corretti perché estranei alla fase.
- Verifiche non eseguibili in questa sessione: tastiera MIDI hardware e misura strumentale esterna della fluidità percepita. Il budget CPU puro è certificato dai probe; l'HUD ha mostrato `60 FPS OK` e tempi GPU intorno a `0,21 ms` negli scenari manuali, ma non sostituisce un profiler esterno.

## Esito fase 6

- Il menu espone una modalità Cyma autonoma tramite pulsante dedicato e tasto `C`. L'ingresso abilita Cyma se necessario, riusa connessioni input/output, stato armonico, renderer globale, HUD e tastiera esistenti e non modifica il trait `Scene`.
- `CymaScene` resta nella finestra principale, gestisce input PC, mouse e MIDI attraverso gli adattatori condivisi e offre ritorno al menu tramite pulsante icona, `Esc` e pulsante indietro del mouse. Non crea una seconda connessione MIDI e non duplica il riconoscimento armonico.
- `cyma-core` espone una palette pura per pitch class e conversione RGB8 finita. Tastiera utente e note file usano colori cromatici indipendenti dall'ottava soltanto quando Cyma è attivo; il percorso disattivato conserva gli schemi upstream.
- Il dominio modale è una piastra quadrata idealmente appoggiata `1 m × 1 m`. Coordinate e frequenza modale relativa includono esplicitamente i lati fisici; mapping pitch-modo, ampiezza, fase, colore e sovrapposizione restano trasformazioni artistiche.
- Il renderer 2D centra un quadrato nel viewport e aggiunge granularità deterministica alle linee nodali; la mesh 3D è quadrata prima della proiezione. Dimensione, allineamento e offset dell'uniform restano invariati a 240 byte e i quattro shader WGSL superano parsing e validazione Naga.
- Compromesso: l'hash granulare è deterministico rispetto ai pixel del target e al preset di qualità, ma non pretende di simulare dinamica, massa o attrito di granelli reali. Una finestra Windows distaccata resta fuori ambito.

## File modificati nella fase 6

- Core musicale e modello fisico: `cyma-core/src/{color,lib,modal}.rs`.
- Renderer e shader: `neothesia-core/src/render/cyma/{mod.rs,shader.wgsl,surface.wgsl}`.
- Scena autonoma e navigazione: `neothesia/src/{main,cyma_hud}.rs`, `neothesia/src/scene/cyma_scene.rs`, `neothesia/src/scene/mod.rs` e `neothesia/src/scene/menu_scene/{mod,state}.rs`.
- Tastiera condivisa: `neothesia/src/scene/playing_scene/{keyboard,mod}.rs` e `neothesia/src/scene/freeplay/mod.rs`.
- Documentazione: `docs/CYMA_IMPLEMENTATION_PLAN.md` e il manuale operativo
  versionato `docs/pages/cyma-operations.md`.

## Esito fase 5

- `CymaConfig` espone `hud_enabled`, disattivato per default e deserializzato retrocompatibilmente. Il pannello impostazioni persiste il toggle senza modificare il comportamento standard quando Cyma o HUD sono disattivati.
- `neothesia/src/cyma_hud.rs` costruisce un unico modello educativo condiviso da Free Play e Playing. Mostra accordo, pitch class, colore, consonanza/tensione, modalità, qualità, stato renderer e budget CPU/GPU, distinguendo euristiche percettive/artistiche e base modale fisicamente motivata.
- `ModalField::sample_with_node_gradient` calcola valore del campo e gradiente analitico di `abs(campo)` in un solo pass sui modi. Input o risultati non finiti vengono neutralizzati e il comportamento resta deterministico.
- La simulazione particellare usa un solo campionamento modale per particella e frame invece dei cinque campionamenti precedenti. Il probe High con 768 particelle resta ampiamente entro il budget iniziale.
- L'HUD usa un layer Nuon overlay condiviso senza modificare il trait `Scene`. Il primo screenshot apparentemente tagliato dipendeva dalla finestra predefinita più larga dell'area visibile a DPI 125%; la ripetizione DPI-aware a 850×760 ha confermato ancoraggio e resize corretti.
- Compromesso: l'HUD opzionale costruisce piccole stringhe di presentazione per frame, coerentemente con il modello retained-immediate di Nuon. Non introduce allocazioni nel renderer Cyma, nella simulazione particellare o nella logica armonica pura.

## File modificati nella fase 5

- Configurazione e campionamento puro: `cyma-core/src/{config,lib,modal}.rs` e `cyma-core/tests/windows_performance.rs`.
- Configurazione persistente: `neothesia-core/src/config/mod.rs`.
- Simulazione e probe particellare: `neothesia-core/src/render/cyma/particles.rs`.
- HUD e integrazione scene: `neothesia/src/{main,cyma_hud}.rs`, `neothesia/src/scene/freeplay/mod.rs`, `neothesia/src/scene/playing_scene/mod.rs` e `neothesia/src/scene/menu_scene/settings.rs`.
- Documentazione: `docs/CYMA_IMPLEMENTATION_PLAN.md`.

## Esito fase 4

- `CymaConfig` espone modalità `Field2d`/`Surface3d`, qualità `Low`/`Medium`/`High` e particelle opzionali. I default restano 2D, Medium e particelle disattivate; la deserializzazione di configurazioni precedenti usa questi valori senza modificare il comportamento standard.
- I preset sono deterministici: Low usa scala 40%, massimo 4 modi, mesh 32×24 e 128 particelle; Medium usa scala 65%, massimo 8 modi, mesh 64×40 e 384 particelle; High usa scala 100%, massimo 12 modi, mesh 96×64 e 768 particelle.
- `ModalField::sample` e `ModalField::node_gradient` restano logica pura in `cyma-core`. Il guadagno di fase è precomputato una volta per componente, evitando `sqrt` e `cos` nei percorsi shader e di campionamento ripetuto.
- Cyma renderizza prima su un target off-screen scalato. La modalità 2D usa un full-screen pass con blend `REPLACE`; la modalità 3D deforma una mesh nel vertex shader e usa `Depth24Plus`. Un secondo pass compone la texture sulla surface, dopo di che la scena upstream usa `Load` senza modificare il trait `Scene`.
- Le particelle usano 768 stati e istanze allocati una sola volta. La simulazione CPU è deterministica, non usa casualità globale né lock e scrive soltanto la slice attiva nel buffer GPU; l'attrazione verso il gradiente di `abs(campo modale)` è una trasformazione artistica, non un modello quantitativo di granelli su una piastra reale.
- Il timer GPU usa quattro timestamp per i pass off-screen/composite, un resolve buffer e due readback buffer. Le callback comunicano tramite `Arc<AtomicU8>` e `Device::poll(Poll)` non bloccante; se `TIMESTAMP_QUERY` manca o il readback fallisce, la UI mostra `N/A` e il rendering continua.
- Superficie 3D, particelle e timer sono capacità opzionali create in error scope separati. Se depth/pipeline 3D non sono disponibili si usa il campo 2D; se le particelle falliscono il campo continua senza particelle; un errore del renderer base mantiene disponibile l'analisi musicale e disattiva soltanto il draw.
- `Context` misura il costo CPU Cyma totale di update e registrazione dei pass con una media mobile esponenziale. La UI mostra stato/fallback, modalità, qualità, particelle, tempo CPU e tempo GPU.
- Non vengono creati `Vec` o lock nel percorso per frame. Le uniche riallocazioni esplicite del renderer avvengono al primo campo attivo o quando cambiano dimensione, qualità o necessità del depth target.
- Compromesso prestazionale: le particelle sono simulate sulla CPU e ogni particella campiona più volte il campo per il gradiente; il limite di 768 e i preset inferiori contengono il costo. Il timer GPU introduce un mapping asincrono opzionale per frame, senza attese bloccanti nel render loop.

## File modificati nella fase 4

- Configurazione e campo puro: `cyma-core/src/{config,lib,modal}.rs`.
- Configurazione persistente: `neothesia-core/src/config/mod.rs`.
- Renderer e shader: `neothesia-core/src/render/mod.rs` e `neothesia-core/src/render/cyma/{mod,target,surface,particles,gpu_timer}.rs`, più `shader.wgsl`, `composite.wgsl`, `surface.wgsl` e `particles.wgsl`.
- Feature GPU opzionale: `wgpu-jumpstart/src/gpu.rs`.
- Integrazione applicativa e metriche: `neothesia/src/{context,main}.rs`.
- Impostazioni e stato renderer: `neothesia/src/scene/menu_scene/settings.rs`.
- Documentazione: `docs/CYMA_IMPLEMENTATION_PLAN.md`.

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
