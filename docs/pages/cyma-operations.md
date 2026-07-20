# Manuale operativo Neothesia Cyma

## Controllo del documento

| Campo | Valore |
| --- | --- |
| Versione manuale | `1.0.0` |
| Versione applicazione | Neothesia `0.4.0` con Cyma fase 6 |
| Baseline funzionale | Commit `dd6a88b` |
| Branch di riferimento | `feature/neothesia-cyma-phase-6` |
| Repository di sviluppo | `autonomyecosystem/Neothesia` |
| Data revisione | 19 luglio 2026 |
| Piattaforma verificata | Windows x86_64 GNU |
| Stato | Verifiche automatiche superate; collaudo manuale fase 6 da completare |

Questo manuale è il riferimento operativo per installare, configurare, usare,
verificare e aggiornare Neothesia Cyma. Deve essere aggiornato nello stesso
upgrade che modifica comportamento, configurazione, requisiti, interfaccia,
prestazioni, compatibilità o procedure di esercizio.

## Scopo e destinatari

Il documento è destinato a:

- utenti che utilizzano Neothesia come visualizzatore e strumento didattico;
- musicisti che collegano una tastiera MIDI;
- operatori incaricati di installazione, aggiornamento e ripristino;
- collaudatori che devono approvare una nuova build;
- manutentori che rilasciano una nuova versione di Cyma.

La versione descritta comprende:

- riproduzione e visualizzazione dei file MIDI di Neothesia;
- modalità Free Play;
- modalità Cyma autonoma nella finestra principale;
- analisi armonica delle note MIDI attive;
- campo modale 2D e superficie 3D;
- particelle nodali opzionali;
- tastiera cromatica basata sulle pitch class;
- HUD educativo e metriche CPU/GPU.

Non comprende una finestra Cyma distaccata, analisi da microfono, FFT o una
simulazione quantitativa di una piastra fisica reale.

## Requisiti operativi

### Requisiti minimi della build corrente

- Windows 64 bit;
- GPU e driver compatibili con un backend supportato da wgpu;
- tastiera e mouse;
- `default.sf2` accanto all'eseguibile per usare il sintetizzatore integrato,
  oppure un SoundFont `.sf2` selezionato nelle impostazioni;
- tastiera MIDI opzionale per l'input hardware.

La build degrada le funzioni GPU opzionali senza interrompere l'analisi
musicale: se la superficie 3D non è disponibile usa il campo 2D; se le
particelle non sono disponibili continua senza particelle; se i timestamp GPU
non sono supportati mostra `N/A`.

### Contenuto della directory operativa

Per una distribuzione Windows mantenere almeno:

```text
Neothesia/
|-- neothesia.exe
|-- default.sf2
`-- settings.ron        creato o aggiornato dall'applicazione
```

I file MIDI dell'utente possono risiedere in qualsiasi cartella accessibile.
I SoundFont personalizzati non devono essere sovrascritti durante un upgrade.

## Installazione e primo avvio

### Installazione da artefatto

1. Estrarre la distribuzione in una cartella scrivibile dall'utente.
2. Verificare che `neothesia.exe` e `default.sf2` siano nella stessa cartella.
3. Avviare `neothesia.exe` dalla cartella della distribuzione.
4. Aprire `Settings`.
5. Selezionare l'uscita audio in `Output`.
6. Se disponibile, selezionare la tastiera in `Input`.
7. Tornare al menu e aprire Free Play o Cyma per stabilire le connessioni.

Al primo avvio, se `settings.ron` non esiste o non è leggibile, Neothesia usa
i valori predefiniti e salva la configurazione alla chiusura.

### Build da sorgente

Dalla radice del workspace:

```powershell
cargo build -p neothesia --release --locked
```

L'eseguibile viene generato in `target/release/neothesia.exe`. Copiare accanto
all'eseguibile `default.sf2` prima del collaudo del sintetizzatore integrato.
La build dell'applicazione non richiede i componenti FFmpeg usati dal crate CLI.

Per un avvio di sviluppo:

```powershell
cargo run -p neothesia --locked
```

## Menu principale

Il menu espone questi flussi:

| Comando | Funzione | Requisito |
| --- | --- | --- |
| `Select File` | Carica un file MIDI | File MIDI leggibile |
| `Settings` | Configura audio, MIDI, aspetto e Cyma | Nessuno |
| `FreePlay` | Apre la tastiera libera | Output e input selezionati |
| `Cyma` | Apre la modalità Cyma autonoma | Abilita Cyma automaticamente |
| `Tracks` | Configura le tracce del file caricato | File MIDI caricato |
| `Play` | Riproduce il file caricato | File MIDI caricato |
| `Exit` | Chiude l'applicazione | Nessuno |

### Scorciatoie del menu

| Tasto | Azione |
| --- | --- |
| `Tab` | Apre la selezione file |
| `Invio` | Avvia il file MIDI caricato |
| `S` | Apre le impostazioni |
| `T` | Apre la configurazione tracce |
| `F` | Apre Free Play |
| `C` | Apre Cyma |
| `Esc` | Torna indietro o apre la conferma di uscita |

## Configurazione Cyma

Aprire `Settings`, scorrere fino alla sezione `Cyma` e attivare
`Enable Cyma`. Entrare direttamente dalla modalità Cyma abilita lo stesso
toggle e il valore viene persistito alla chiusura.

### Parametri disponibili

| Impostazione | Valori | Default | Effetto operativo |
| --- | --- | --- | --- |
| `Enable Cyma` | On/Off | Off | Attiva analisi armonica e renderer |
| `Detected Chord` | Sola lettura | `Unknown` | Mostra l'accordo riconosciuto |
| `Renderer` | Sola lettura | Dipende dalla GPU | Mostra stato attivo o fallback |
| `Visualization` | `2D Field`, `3D Surface` | `2D Field` | Seleziona campo piano o superficie deformata |
| `Quality` | `Low`, `Medium`, `High` | `Medium` | Regola risoluzione, modi, mesh e particelle |
| `Nodal Particles` | On/Off | Off | Aggiunge particelle attratte verso i nodi |
| `Educational HUD` | On/Off | Off | Mostra armonia, colore, modello e budget frame |
| `Response Time` | 0-2000 ms, passo 20 ms | 160 ms | Regola la velocità di interpolazione armonica |
| `Cyma CPU` | Sola lettura | `N/A` o misura | Tempo CPU medio del percorso Cyma |
| `Cyma GPU` | Sola lettura | `N/A` o misura | Tempo GPU, se supportato |

### Preset di qualità

| Qualità | Scala target | Modi massimi | Mesh 3D | Particelle massime | Uso consigliato |
| --- | ---: | ---: | ---: | ---: | --- |
| Low | 40% | 4 | 32 x 24 | 128 | GPU integrata o finestra grande |
| Medium | 65% | 8 | 64 x 40 | 384 | Uso ordinario |
| High | 100% | 12 | 96 x 64 | 768 | GPU con margine e acquisizione video |

Se il frame rate non è stabile, applicare nell'ordine:

1. impostare `Quality` su `Low`;
2. disattivare `Nodal Particles`;
3. scegliere `2D Field`;
4. disattivare `Educational HUD` se non serve;
5. ridurre la dimensione della finestra;
6. aggiornare il driver GPU.

## Modalità Cyma autonoma

### Apertura

Dal menu principale selezionare il pulsante Cyma oppure premere `C`. La scena:

- abilita Cyma se era disattivato;
- usa l'input MIDI già selezionato;
- usa l'output audio già selezionato;
- mostra la tastiera condivisa nella parte inferiore;
- renderizza il campo Cyma dietro tastiera e interfaccia;
- mostra l'HUD solo se `Educational HUD` è attivo.

### Uscita

Usare uno dei seguenti comandi:

- pulsante freccia in alto a sinistra;
- `Esc`;
- pulsante indietro del mouse.

Il ritorno al menu azzera lo stato delle note provenienti dal file senza
cancellare arbitrariamente lo stato dell'input utente.

### Tastiera del computer

La tastiera PC invia note MIDI dalla nota C4. Tenere premuti più tasti per
formare un accordo.

| Tasto PC | Nota MIDI | Nota |
| --- | ---: | --- |
| `A` | 60 | C4 |
| `W` | 61 | C#4 |
| `S` | 62 | D4 |
| `E` | 63 | D#4 |
| `D` | 64 | E4 |
| `F` | 65 | F4 |
| `T` | 66 | F#4 |
| `G` | 67 | G4 |
| `Y` | 68 | G#4 |
| `H` | 69 | A4 |
| `U` | 70 | A#4 |
| `J` | 71 | B4 |
| `K` | 72 | C5 |
| `O` | 73 | C#5 |
| `L` | 74 | D5 |
| `P` | 75 | D#5 |
| `;` | 76 | E5 |
| `'` | 77 | F5 |

I caratteri `;` e `'` dipendono dal layout logico prodotto dalla tastiera del
sistema operativo.

### Mouse e tastiera MIDI

- Fare clic sui tasti del pianoforte per inviare Note On e rilasciare per Note Off.
- Trascinare con il pulsante sinistro premuto per passare da una nota all'altra.
- La tastiera MIDI usa la connessione scelta in `Settings > Input`.
- Note On con velocity zero vengono trattate come Note Off.
- Il sustain CC64 mantiene le note finché il pedale non viene rilasciato.
- Eventi sul canale percussioni MIDI 10 non partecipano all'analisi armonica.

## Colori cromatici

Quando Cyma è attivo, le note utente e le note dei file usano una palette
deterministica per pitch class. La stessa nota in ottave diverse usa lo stesso
colore. Quando Cyma è disattivato restano validi gli schemi colore standard di
Neothesia.

| Pitch class | RGB base |
| --- | --- |
| C | `(255, 0, 0)` |
| C# | `(0, 128, 255)` |
| D | `(255, 255, 0)` |
| D# | `(128, 0, 255)` |
| E | `(0, 255, 0)` |
| F | `(255, 0, 128)` |
| F# | `(0, 255, 255)` |
| G | `(255, 128, 0)` |
| G# | `(0, 0, 255)` |
| A | `(128, 255, 0)` |
| A# | `(255, 0, 255)` |
| B | `(0, 255, 128)` |

La variante usata sui tasti neri conserva la tonalità con luminosità ridotta.

## Lettura dell'HUD educativo

L'HUD mostra:

- accordo riconosciuto;
- pitch class attive;
- colore musicale composto;
- consonanza e tensione;
- modalità 2D/3D e qualità;
- stato del renderer;
- tempo CPU e GPU;
- stato rispetto al budget di 60 FPS.

`Consonance` e `Tension` sono euristiche percettive. Colore, associazione
pitch-modo, ampiezza, fase e sovrapposizione sono trasformazioni artistiche.
La base modale usa funzioni separabili di una piastra quadrata sottile
idealmente appoggiata di `1 m x 1 m`; non rappresenta materiale, spessore,
smorzamento, eccitazione o attrito misurati su una piastra reale.

## Uso con Free Play e file MIDI

### Free Play

1. Selezionare input e output nel menu.
2. Attivare Cyma nelle impostazioni.
3. Aprire `FreePlay` oppure premere `F`.
4. Suonare dalla tastiera PC, dal mouse o dalla tastiera MIDI.
5. Verificare tastiera cromatica, accordo e visualizzazione.
6. Premere `Esc` per tornare al menu.

### Riproduzione di un file

1. Selezionare `Select File` oppure premere `Tab`.
2. Aprire `Tracks` per scegliere visibilità e modalità delle tracce.
3. Attivare Cyma nelle impostazioni.
4. Selezionare `Play` oppure premere `Invio`.
5. Verificare che le note file colorino i tasti per pitch class.
6. Usare `Spazio` per pausa/ripresa.
7. Usare `Esc` per tornare al menu.

Pausa, seek, rewind, loop e cambio scena azzerano le note provenienti dal file
per evitare note bloccate. Le note dell'utente restano in uno stato separato.

## Configurazione persistente

Su Windows `settings.ron` si trova nella directory di avvio. La sezione Cyma
della versione corrente ha questa forma:

```ron
cyma: V1(
    enabled: false,
    response_time_ms: 160,
    visualization: Field2d,
    quality: Medium,
    particles_enabled: false,
    hud_enabled: false,
),
```

Modificare preferibilmente i valori dall'interfaccia. Se è necessaria una
modifica manuale:

1. chiudere Neothesia;
2. creare una copia di `settings.ron`;
3. modificare soltanto valori e varianti documentati;
4. riavviare e controllare il log per errori RON;
5. verificare che i valori siano ancora presenti dopo una chiusura pulita.

Campi sconosciuti o sintassi RON non valida possono causare il caricamento dei
default. Conservare sempre una copia della configurazione compatibile con la
versione installata.

## Backup, upgrade e rollback

### Backup prima dell'upgrade

Con Neothesia chiuso, salvare:

- `settings.ron`;
- SoundFont personalizzati;
- eventuali file MIDI distribuiti insieme all'applicazione;
- eseguibile e numero versione attualmente in uso;
- log o screenshot del collaudo precedente.

### Procedura di upgrade

1. Verificare versione, commit o tag della nuova build.
2. Leggere la cronologia del manuale e le note di rilascio.
3. Eseguire il backup.
4. Sostituire l'eseguibile e gli asset distribuiti dalla release.
5. Non sovrascrivere `settings.ron` o SoundFont personali senza migrazione esplicita.
6. Avviare l'applicazione.
7. Controllare `Output`, `Input` e la sezione `Cyma`.
8. Eseguire lo smoke test e il collaudo Cyma descritti sotto.
9. Approvare la build soltanto se non restano regressioni bloccanti.

### Procedura di rollback

1. Chiudere Neothesia.
2. Ripristinare l'eseguibile e gli asset della versione precedente.
3. Ripristinare il `settings.ron` associato alla versione precedente.
4. Avviare e verificare menu, audio e input MIDI.
5. Registrare motivo del rollback, build rifiutata ed evidenza dell'errore.

Il ripristino del file configurazione è importante: una versione precedente
può rifiutare campi introdotti da una versione successiva.

## Collaudo operativo

### Smoke test minimo

1. Avviare Neothesia con Cyma disattivato.
2. Verificare menu e comportamento standard.
3. Aprire `Settings` e selezionare input/output.
4. Entrare in Cyma dal pulsante.
5. Tornare al menu con la freccia.
6. Entrare in Cyma con `C`.
7. Premere contemporaneamente `A`, `D` e `G`.
8. Verificare accordo `CM`, C rosso, E verde e G arancione.
9. Rilasciare le note e verificare lo spegnimento dei tasti.
10. Ridimensionare la finestra e controllare che la piastra 2D resti quadrata e centrata.
11. Premere `Esc`, tornare al menu e chiudere normalmente.
12. Riavviare e verificare la persistenza delle impostazioni.

### Matrice completa di accettazione

| ID | Prova | Esito atteso |
| --- | --- | --- |
| CYMA-01 | Avvio con Cyma Off | Nessun renderer Cyma creato, comportamento standard invariato |
| CYMA-02 | Ingresso da pulsante | Scena Cyma aperta e Cyma abilitato |
| CYMA-03 | Ingresso con `C` | Stesso risultato del pulsante |
| CYMA-04 | Input `A` + `D` + `G` | Accordo `CM`, tre colori corretti, campo visibile |
| CYMA-05 | Note in ottave diverse | Stessa pitch class, stesso colore |
| CYMA-06 | Note On/Off duplicate | Nessuna nota rilasciata prematuramente |
| CYMA-07 | Sustain CC64 | Note mantenute fino al rilascio pedale |
| CYMA-08 | Canale MIDI 10 | Percussioni escluse dall'armonia |
| CYMA-09 | Mouse sui tasti | Note e colori seguono pressione e trascinamento |
| CYMA-10 | Campo 2D | Piastra quadrata centrata con linee nodali granulari |
| CYMA-11 | Superficie 3D | Piastra quadrata prima della proiezione, nessun errore GPU |
| CYMA-12 | Low/Medium/High | Preset applicati senza riavvio o crash |
| CYMA-13 | Particelle On/Off | Particelle presenti solo quando abilitate e supportate |
| CYMA-14 | HUD On/Off | Overlay completo, nessuna sovrapposizione incoerente |
| CYMA-15 | Resize 850 x 760 | Piastra, tastiera e HUD restano visibili |
| CYMA-16 | Free Play | Armonia, colori e output audio corretti |
| CYMA-17 | File MIDI | Note file, pausa, seek, loop e reset corretti |
| CYMA-18 | Freccia, `Esc`, mouse back | Ritorno al menu da tutti i comandi |
| CYMA-19 | Chiusura | Codice di uscita 0 e configurazione salvata |
| CYMA-20 | GPU senza timestamp | Rendering attivo e metrica GPU `N/A` |
| CYMA-21 | GPU senza 3D/particelle | Fallback dichiarato, analisi musicale attiva |
| CYMA-22 | Cyma nuovamente Off | Colori e comportamento upstream ripristinati |

### Verifiche automatiche per una release

Dalla radice del workspace:

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Se dipendenze opzionali upstream bloccano il workspace completo, registrare
l'errore esatto ed eseguire almeno:

```powershell
cargo check -p cyma-core -p neothesia-core -p neothesia
cargo test -p cyma-core
cargo test -p neothesia-core --lib
cargo test -p neothesia --bin neothesia
cargo clippy -p cyma-core --all-targets -- -D warnings
```

I warning upstream eventualmente consentiti devono essere nominati; non è
ammesso mascherare nuovi warning introdotti da Cyma.

## Stato di verifica della versione 1.0.0 del manuale

| Area | Stato | Evidenza o limite |
| --- | --- | --- |
| Formattazione Rust | Superata | `cargo fmt --all -- --check` |
| Check crate modificati | Superato | `cyma-core`, `neothesia-core`, `neothesia` |
| Test mirati | Superati | 59 test: 34 + 12 + 13 |
| Shader WGSL | Superati | Parsing e validazione Naga di quattro shader |
| Draw GPU off-screen | Superato | Readback non nero per C-E-G |
| Clippy mirato | Superato | Nessun nuovo warning Cyma |
| Build applicazione | Superata | `cargo build -p neothesia` |
| GUI fase 6 | Da completare | La sessione elevata non ha pubblicato la finestra winit |
| Tastiera MIDI hardware | Da completare | Hardware non disponibile nella sessione corrente |
| Workspace completo | Bloccato dall'ambiente | Mancano `pkg-config`/FFmpeg e header C per feature opzionali |

Questa build è pronta per il collaudo manuale, ma non deve essere dichiarata
release operativamente approvata finché CYMA-01-CYMA-22 non sono registrati.

## Risoluzione problemi

### Cyma non appare

1. Verificare `Enable Cyma`.
2. Suonare almeno una nota: un campo vuoto non viene disegnato.
3. Controllare la riga `Renderer`.
4. Provare `2D Field` e qualità `Low`.
5. Aggiornare il driver GPU.
6. Riavviare dopo aver salvato una copia di `settings.ron`.

### Nessun suono

1. Verificare `Settings > Output`.
2. Se si usa `Buildin Synth`, controllare `default.sf2`.
3. Verificare `Audio Gain`.
4. Rientrare nella scena per ristabilire la connessione.
5. Provare un output MIDI esterno noto funzionante.

### La tastiera MIDI non risponde

1. Collegare e accendere la tastiera prima della selezione.
2. Selezionarla in `Settings > Input`.
3. Tornare al menu e riaprire Cyma o Free Play.
4. Verificare che il dispositivo non sia aperto in esclusiva da un'altra app.
5. Scollegare e ricollegare il dispositivo, quindi riselezionarlo.

### Note bloccate

1. Rilasciare il sustain.
2. Tornare al menu per azzerare lo stato file.
3. Riaprire la scena.
4. Se il problema riguarda l'input hardware, riconnettere il dispositivo per
   garantire l'invio dei Note Off mancanti.

### Prestazioni insufficienti

1. Impostare qualità `Low`.
2. Disattivare particelle e HUD.
3. Usare il campo 2D.
4. Ridurre la finestra.
5. Confrontare `Cyma CPU` e `Cyma GPU` con il budget di 16,67 ms.
6. Se `Cyma GPU` mostra `N/A`, usare un profiler esterno per la diagnosi.

### Configurazione non caricata

1. Chiudere Neothesia.
2. Copiare `settings.ron` in una posizione sicura.
3. Controllare sintassi, nomi dei campi e varianti RON.
4. Rinominare temporaneamente il file per consentire la generazione dei default.
5. Reimpostare i valori dall'interfaccia.
6. Conservare il file non valido insieme al log per l'analisi.

## Limitazioni note

- La piattaforma collaudata nella sessione corrente è Windows x86_64 GNU.
- Il collaudo manuale specifico della scena Cyma fase 6 resta da completare.
- La tastiera MIDI hardware non è stata provata nella sessione corrente.
- Il timer GPU è opzionale e può mostrare `N/A`.
- Una finestra Cyma distaccata non è inclusa.
- Il modello visuale è fisicamente motivato ma artistico, non calibrato.
- I controlli completi del workspace richiedono dipendenze FFmpeg e C esterne
  che non servono ai tre crate Cyma verificati.

## Politica obbligatoria di aggiornamento del manuale

Ogni upgrade deve aggiornare questo file nello stesso branch e nella stessa PR.
Per upgrade si intende qualsiasi modifica a:

- funzionalità o flusso utente;
- controlli, testi, scorciatoie o layout;
- configurazione persistente e relativi default;
- requisiti di sistema, GPU, audio o MIDI;
- installazione, build, distribuzione, backup o rollback;
- fallback, errori, metriche e prestazioni;
- test automatici o matrice di collaudo;
- limitazioni note e stato di verifica;
- dipendenze o compatibilità di piattaforma.

### Regola di versionamento del manuale

- Incremento **major**: procedura incompatibile o ristrutturazione operativa.
- Incremento **minor**: nuova funzione, impostazione o procedura.
- Incremento **patch**: correzione senza variazione del comportamento.

### Checklist del manutentore

Prima di approvare ogni upgrade:

1. aggiornare la tabella `Controllo del documento`;
2. aggiornare versione software, branch, tag o baseline funzionale;
3. aggiungere una riga alla cronologia revisioni;
4. confrontare menu, impostazioni e scorciatoie con il codice corrente;
5. aggiornare esempio `settings.ron` e valori predefiniti;
6. aggiornare installazione, upgrade e rollback;
7. aggiornare matrice di accettazione ed eseguire i casi interessati;
8. aggiornare stato di verifica e limitazioni note;
9. verificare tutti i link e compilare il sito VitePress;
10. includere manuale e codice nello stesso commit o nella stessa PR;
11. se non esiste impatto operativo, registrare comunque `Nessun impatto operativo` nella cronologia;
12. bloccare la release se il manuale descrive un comportamento diverso dalla build.

### Cronologia revisioni

| Manuale | Data | Versione software | Modifica | Impatto operativo |
| --- | --- | --- | --- | --- |
| 1.0.0 | 19/07/2026 | Neothesia 0.4.0 + Cyma fase 6 (`dd6a88b`) | Prima edizione completa | Introduce procedure operative, collaudo e manutenzione Cyma |

## Riferimenti

- [Piano di implementazione Cyma](../CYMA_IMPLEMENTATION_PLAN.md)
- [Installazione Neothesia](installation.md)
- [Scorciatoie](shortcuts.md)
- [Personalizzazione](customization.md)
- Repository: <https://github.com/autonomyecosystem/Neothesia>
