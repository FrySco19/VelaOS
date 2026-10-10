# M3 — Intent Router

Stato: contratto progettato, da implementare in Rust dopo i prerequisiti M2.

## Componenti e ordine delle verifiche

1. **Ingress:** riceve un Envelope limitato, assegna principal, correlazione e QoS tramite sessione autenticata. Token e chiavi non entrano nel prompt. Le dichiarazioni di identità del modello sono ignorate.
2. **Registry:** espone manifest firmati/versionati con nomi di servizio, metodo, schema di argomenti, tipo di input/output e impatto. Il planner vede soltanto il sottoinsieme consentito e necessario. Questo filtro non sostituisce l'autorizzazione finale.
3. **Route cache:** cerca una firma esatta del contesto. Hit valido evita completamente il modello. Il piano viene comunque rivalidato con permessi e stato correnti; la cache non conserva un diritto di esecuzione.
4. **Semantic compiler:** su miss, il SLM produce una proposta vincolata da grammatica. Output oltre quota, malformato o incompleto viene rifiutato; nessun recupero permissivo o esecuzione parziale.
5. **Validator Rust:** parsing stretto, campi extra vietati, schema, dimensioni, DAG aciclico, riferimenti e tipi compatibili, metodi presenti nel manifest. Nessun comando libero né risoluzione di risorse inventate.
6. **Policy:** Biscuit attenuato per ogni passo; verifica revoca/scadenza/quote e assegna Auto, Confirm o Deny. Il modello non può scegliere la classe di rischio.
7. **Executor:** esegue il DAG tramite task Wasm e host call Vela ristrette. I servizi LFS e bus ricontrollano l'autorità; solo loro coniano handle e route.
8. **Audit:** ricevuta firmata con versione dei manifest, modello/prompt, piano normalizzato, decisione e risultato. Nessun token segreto; testo personale ridotto secondo retention esplicita.

## Contratto IntentPlan v1

| Campo | Regola |
| --- | --- |
| version | Valore esatto `vela.intent-plan.v1`. |
| status | `Plan`, `Clarify` o `Unsupported`, unioni disgiunte. |
| steps | Per Plan: 1–4 passi, identificatori locali unici, ordine topologico. |
| service / method / schema | Valori enumerati dal registry ristretto della richiesta. |
| args | Argomenti dello schema; stringhe e numeri limitati. Vietati token, credenziali e puntatori. |
| inputs | Riferimenti a output tipizzati di passi precedenti o a risorse del contesto fornite dall'host. |
| context_refs | Soltanto ID presenti nella tabella host della richiesta; non sono handle con autorità. |
| question | Solo Clarify: una domanda breve per l'informazione realmente mancante; nessun passo. |
| reason_code | Solo Unsupported: enum deterministico, nessun passo. |

Limiti iniziali: Envelope 16 KiB; output piano 4 KiB; profondità argomenti 8; quattro passi. Le quote del prompt/decoding sono separate. Un oggetto grande viene referenziato, non inserito in args. Una richiesta eccedente viene rifiutata con errore leggibile.

Il planner usa nomi leggibili di manifest. **La codifica binaria è compito di Rust:** dopo parsing, controllo di tipi e policy, il router serializza un intent Vela versionato per il Neural Bus. Non accetta byte generati liberamente dal modello. Versione e hash dello schema vincolano mittente e destinatario; incompatibilità rifiutate prima dell'esecuzione.

## SAVE / RECALL / RESTORE come piani

- SAVE: `net.snapshot` → `lfs.put`; l'output snapshot è un riferimento di data plane. Il modello non compone una configurazione presunta.
- RECALL: `lfs.recall` restituisce One, Ambiguous o Empty. Gli score non concedono diritti. Ambiguous richiede selezione; Empty offre list autorizzato.
- RESTORE: recall o selezione esplicita → preparazione e diff → conferma → apply. La barriera di conferma è inserita dal router, anche se il planner tenta di ometterla. Nessun apply quando il risultato è Ambiguous/Empty.

Un handle già presente nel contesto resta nella tabella host; il modello vede un riferimento locale. L'host verifica identità, proprietario, schema e scadenza prima di ogni uso. La disponibilità in un piano precedente non basta.

## Cache senza amplificazione dei privilegi

Chiave: richiesta normalizzata conservativamente, contesto risolto, principal/tenant, versione della policy e revoca, hash manifest/schema, epoch planner e prompt. Le stringhe di identificatori e i valori significativi non vengono alterati da una normalizzazione linguistica aggressiva.

Valore: template validato privo di capability, handle attivi, conferme o route già emesse. Binding e autorizzazioni vengono rifatti su ogni hit. TTL massimo iniziale 120 s, subordinato alla scadenza delle risorse. Invalidation su aggiornamenti del registry, policy, revoche, contesto o modello. Budget cache 16 MB con espulsione limitata.

La cache ANN prevista dal PDF rimane una sorgente di **candidati di template**, senza autorità di esecuzione finché soglia e equivalenza dei parametri non sono calibrate. Un match semantico non deve riutilizzare l'oggetto, la conferma o i permessi di un altro intento.

## Taint e planner ostile

UserText può specificare la richiesta, non ridefinire registry o policy. Untrusted include contenuti salvati e segnali esterni: non viene concatenato al messaggio di sistema o alle istruzioni del planner. L'host rende disponibili solo fatti strutturati con provenienza, oppure un task di estrazione isolato produce uno schema di dati validato. Un fatto derivato resta non autorevole: non concede diritti.

Assumiamo anche un planner completamente compromesso. La grammatica previene output strutturalmente illegali; solo il validator e i servizi impediscono azioni non autorizzate. Temperature 0 e seed fissato aiutano riproducibilità, ma non provano determinismo fra backend/hardware differenti.

## Scheduling

Una inferenza attiva nel profilo iniziale; al massimo quattro richieste interattive pendenti, con deadline e cancellazione. Eventi ricorrenti già registrati usano template deterministici e non saturano il planner. Priorità e risorse di inferenza sono separate dai worker fast-lane. Nessun blocco del data plane in attesa di inferenza, logging o compilazione Wasm.

Fonti: specifica VelaOS §§4–5; [grammatiche llama.cpp](https://github.com/ggml-org/llama.cpp/blob/master/grammars/README.md). La grammatica limita la sintassi, non garantisce correttezza semantica.
