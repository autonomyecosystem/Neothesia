# Neothesia Cyma

## Missione del progetto

Estendere Neothesia con un sistema audiovisivo educativo che trasformi le note MIDI attive in:

1. stato armonico;
2. colore musicale composto;
3. campo modale di tipo Chladni;
4. visualizzazione geometrica 2D/3D;
5. eventuali particelle attratte dalle linee nodali.

Il nome di lavoro del progetto è Neothesia Cyma.

## Principi architetturali obbligatori

- Conservare l'architettura Rust e wgpu già presente.
- Usare shader WGSL nativi.
- Non integrare WebView, Three.js, Electron, projectM o motori JavaScript.
- Non introdurre analisi da microfono o FFT nella prima versione.
- Non sostituire o duplicare la gestione MIDI esistente.
- Individuare e riutilizzare il riconoscimento degli accordi già presente nella modalità Free Play.
- Separare sempre logica musicale e rendering.
- `cyma-core` non deve dipendere da wgpu, winit o componenti UI.
- Il rendering non deve contenere logica di riconoscimento armonico.
- Non effettuare grandi refactoring non necessari del codice upstream.
- Non cambiare il comportamento standard di Neothesia quando Cyma è disattivato.
- Ogni nuova impostazione deve avere un valore predefinito retrocompatibile.
- Non copiare direttamente codice da progetti esterni senza verificare licenza, attribuzione e compatibilità GPL.
- Documentare chiaramente quali trasformazioni sono artistiche e quali hanno fondamento fisico.

## Modalità di lavoro

Prima di modificare il codice:

1. ispezionare il repository corrente;
2. leggere i `Cargo.toml` del workspace e dei crate interessati;
3. individuare il flusso degli eventi MIDI;
4. individuare il ciclo di rendering;
5. individuare la configurazione persistente;
6. individuare l'implementazione Free Play e chord detection;
7. aggiornare il piano esecutivo con i percorsi reali dei file.

Non assumere che i percorsi proposti nel piano siano ancora corretti.

Completare una sola fase per volta. Al termine di ogni fase:

- eseguire i controlli previsti;
- aggiornare `docs/CYMA_IMPLEMENTATION_PLAN.md`;
- aggiornare `docs/pages/cyma-operations.md` nello stesso branch e nella stessa
  PR per ogni modifica funzionale, di configurazione o di release; registrare
  comunque `Nessun impatto operativo` nella cronologia quando applicabile;
- elencare i file modificati;
- descrivere decisioni e compromessi;
- segnalare test superati e test non eseguibili;
- non iniziare automaticamente la fase successiva.

## Qualità del codice

- Preferire tipi espliciti e piccoli moduli.
- Evitare `unwrap()` ed `expect()` nei percorsi eseguiti a runtime, salvo invarianti documentate.
- Non ignorare silenziosamente errori GPU, MIDI o configurazione.
- Evitare allocazioni per frame nel percorso di rendering.
- Evitare lock bloccanti nel render loop.
- Usare strutture compatibili con `bytemuck` per gli uniform buffer.
- Verificare sempre allineamento e padding degli uniform WGSL.
- Limitare il numero massimo di componenti modali a 12 nella prima versione.
- Rendere deterministico il comportamento: stesse note e stesso preset devono produrre lo stesso risultato.

## Compatibilità

La sessione di sviluppo e verifica corrente è limitata a Windows. Le modifiche pure e condivise non devono introdurre dipendenze Windows-specifiche non necessarie.

Le funzionalità Cyma devono degradare correttamente se una determinata funzione GPU non è disponibile.

## Comandi minimi di verifica

Eseguire dalla radice del workspace:

```text
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Quando il clippy dell'intero workspace fallisce per problemi upstream già esistenti:

1. documentare esattamente gli errori preesistenti;
2. eseguire clippy almeno sui crate modificati;
3. non mascherare nuovi warning introdotti dal lavoro Cyma.

Eseguire inoltre `cargo run -p neothesia` e verificare manualmente:

- avvio normale;
- caricamento di un file MIDI;
- modalità Free Play;
- input da tastiera MIDI, quando disponibile;
- attivazione e disattivazione di Cyma;
- ridimensionamento della finestra;
- ritorno al menu;
- chiusura pulita.

## Test obbligatori

La logica pura deve avere test unitari per:

- conversione MIDI -> pitch class;
- indipendenza dall'ottava;
- gestione Note On e Note Off;
- sustain pedal;
- note duplicate;
- calcolo del colore;
- accordo vuoto;
- normalizzazione dei pesi;
- assenza di NaN e infiniti;
- consonanza e tensione;
- interpolazione temporale;
- serializzazione e deserializzazione della configurazione.

## Prestazioni

Obiettivi iniziali:

- nessuna allocazione significativa per frame;
- aggiornamento armonico inferiore a 1 ms su hardware desktop comune;
- rendering fluido a 60 FPS con preset medio;
- possibilità di ridurre risoluzione e particelle;
- Cyma disattivato con costo praticamente nullo.

## Sicurezza delle modifiche

- Lavorare sempre su un branch dedicato.
- Non eseguire `git reset --hard`.
- Non sovrascrivere modifiche dell'utente.
- Non modificare file non collegati al compito.
- Non aggiungere binari, cache, artefatti di build o file personali.
- Non effettuare commit o push senza esplicita istruzione.
