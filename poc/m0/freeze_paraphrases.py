"""Author a fixed synthetic M0 corpus before retrieval; no model or search is called."""
import hashlib
import json
from pathlib import Path

GROUPS = {
"exact_profile": """Mostrami la configurazione della rete ZEPHYR-731
Recupera lo stato salvato per ZEPHYR-731
Cerco il profilo che contiene ZEPHYR-731
Quale configurazione usa il DNS 9.9.9.9
Trova la rete associata al resolver 9.9.9.9
Richiama il vecchio stato con ZEPHYR-731
Apri la scheda relativa a ZEPHYR-731
Vorrei rivedere i parametri con DNS 9.9.9.9
Individua lo snapshot con identificativo 8b9539b7-3c6f-4bb1-9355-ae58471cf013
Cerca la configurazione con UUID 8b9539b7-3c6f-4bb1-9355-ae58471cf013
Mostra i dettagli della connessione ZEPHYR-731
Ho bisogno delle impostazioni registrate per ZEPHYR-731
Qual era il profilo salvato con 9.9.9.9
Ritrova lo stato precedente che cita ZEPHYR-731
Seleziona la scheda dove compare 9.9.9.9
Mi serve il salvataggio legato a ZEPHYR-731
Il mio riferimento è 8b9539b7-3c6f-4bb1-9355-ae58471cf013
Voglio consultare il resolver 9.9.9.9 nel profilo registrato
Riporta la configurazione corrispondente a ZEPHYR-731
In quale oggetto è memorizzato ZEPHYR-731
Find the saved settings for ZEPHYR-731
Retrieve the archived network using 9.9.9.9
Show the profile containing ZEPHYR-731
Locate UUID 8b9539b7-3c6f-4bb1-9355-ae58471cf013
I need the stored configuration for ZEPHYR-731
Which saved state has resolver 9.9.9.9
Look up the earlier connection named ZEPHYR-731
Find the record associated with 8b9539b7-3c6f-4bb1-9355-ae58471cf013
Display the archived profile with DNS 9.9.9.9
Recall the configuration identified by ZEPHYR-731
Confronta il ricordo della caffetteria con il riferimento esatto ZEPHYR-731
Anche se parlo di VPN a Milano cerco ZEPHYR-731
Tra i profili per portatili mi interessa quello con 9.9.9.9
Non la guida ai guasti ma il record ZEPHYR-731
Non il video della barca ma lo stato ZEPHYR-731
Il riferimento preciso resta 8b9539b7-3c6f-4bb1-9355-ae58471cf013 anche se ricordo il bar
Per il resolver voglio proprio 9.9.9.9 nel record salvato
Il router di casa non è quello che cerco voglio ZEPHYR-731
Find ZEPHYR-731 even though I mentioned the secure coffee shop connection
The exact record is 8b9539b7-3c6f-4bb1-9355-ae58471cf013 rather than the sunset movie""",
"semantic_profile": """Ritrova la connessione protetta del portatile in caffetteria
Cerco le impostazioni del Wi-Fi usato al bar con il tunnel verso Milano
Mostrami il profilo del laptop sul punto di accesso pubblico con traffico cifrato
Quale stato salvato proteggeva la navigazione mentre prendevo un caffè
Mi serve il collegamento sicuro del computer portatile nel locale
Recupera la rete della caffetteria dove passavo attraverso una VPN
Trova la connessione del portatile con un tunnel sicuro diretto a Milano
La configurazione che difendeva i dati sulla rete pubblica del bar
Vorrei lo stato della connessione cifrata mentre lavoravo al caffè
Cerca il profilo wireless del laptop sulla rete ospiti della caffetteria
Mostra il salvataggio del computer con VPN milanese su hotspot pubblico
Richiama le impostazioni usate per navigare al bar in sicurezza
Individua la rete pubblica dove il tunnel proteggeva il portatile
Il collegamento senza fili del locale con instradamento protetto verso Milano
Ritrova la sessione di lavoro al caffè con traffico dentro il tunnel
Mi interessa la configurazione del laptop sul Wi-Fi della caffetteria
Cerca la connessione protetta quando il computer era collegato alla rete del bar
Recupera il salvataggio del portatile con connessione cifrata al punto di accesso pubblico
Quale profilo usava un collegamento sicuro verso Milano dalla caffetteria
Lo stato di rete del laptop mentre usavo una VPN nel locale
Find the laptop connection protected by a tunnel at the coffee shop
Recall the encrypted wireless session on the cafe public hotspot
Show the saved laptop profile with a secure tunnel towards Milan
I need the coffee shop network where traffic was protected
Locate the portable computer configuration used on public cafe Wi-Fi
Retrieve my secure browsing setup while working over coffee
Find the cafe hotspot profile with the Milan tunnel
Which connection protected the laptop on a public wireless network
Show the saved VPN session at the coffee shop
Recall the encrypted cafe connection for the laptop
Trova il profilo con SSID CafeGuest-5G
Recupera la configurazione del bar con DNS 1.1.1.1
Mostra il tunnel del portatile associato a CafeGuest-5G
Cerco la connessione protetta della caffetteria che usa 1.1.1.1
La rete ospiti CafeGuest-5G del computer portatile
Vorrei rivedere le impostazioni della VPN mentre ero sulla rete del locale
Il salvataggio della sessione wireless pubblica protetta del laptop
Recupera i parametri del collegamento cifrato dal bar verso Milano
La connessione del portatile al caffè che teneva privato il traffico
Il profilo della caffetteria con VPN e rete senza fili""",
"home_profile": """Trova la rete domestica usata dalla televisione e dalla console
Recupera il profilo del router di casa senza VPN
Cerco la connessione della console con indirizzi assegnati automaticamente
Mostra le impostazioni della rete di casa per giocare e guardare la TV
Quale configurazione collega il televisore al router domestico
Vorrei il salvataggio della rete casalinga senza tunnel sicuro
Individua il profilo di rete per console e televisione
La connessione del router domestico con indirizzi automatici
Recupera le impostazioni usate a casa per la console da gioco
Mostra la rete privata di casa che serve il televisore
Il profilo per l'intrattenimento domestico con assegnazione automatica degli indirizzi
Cerco il collegamento del televisore e della console al router
Trova lo stato della rete casalinga dove la VPN non era attiva
La configurazione senza tunnel usata dal router di casa
Mi serve il salvataggio del Wi-Fi domestico per TV e giochi
Recupera la connessione di casa con indirizzi automatici per i dispositivi
Qual era la rete della console collegata al router casalingo
Ritrova il profilo del router che collegava insieme console e TV
Voglio consultare le impostazioni della connessione domestica senza VPN
Lo stato salvato della rete dedicata all'intrattenimento a casa
Find the home router profile serving the television and game console
Retrieve the household connection with automatic addresses and no tunnel
Show the network used for gaming and TV at home
I need the domestic router settings without an active VPN
Locate the saved connection for the console and television
Recall the home entertainment network configuration
Which profile gives automatic addresses to the TV and game console
Show the household network with no secure tunnel enabled
Find the home connection used by entertainment devices
Retrieve the router state for television and console gaming
Trova la configurazione con SSID HomeLan-5G
Cerco il profilo domestico che usa DNS 8.8.8.8
Mostra i parametri del router per HomeLan-5G
Richiama la rete della console con resolver 8.8.8.8
Il televisore era collegato alla rete HomeLan-5G
Vorrei il profilo della LAN di casa con indirizzi assegnati dal router
La rete usata dalla console senza instradare il traffico in una VPN
Recupera il salvataggio casalingo per televisione e videogiochi
Trova la connessione locale del router domestico per dispositivi multimediali
Mostra il profilo di casa con televisore console e indirizzi automatici""",
"guide": """Cerco le istruzioni per diagnosticare problemi di connettività
Trova il manuale per risolvere i guasti della rete wireless
Mostrami la guida per controllare il nome della rete e il resolver
Mi serve un documento che spieghi come analizzare una connessione che non funziona
Recupera le istruzioni per individuare problemi del DNS e della rete
Vorrei la guida alla diagnosi della connessione senza fili
Cerca il manuale di risoluzione dei problemi del Wi-Fi
Quale nota spiega come verificare connettività e resolver
Ritrova il documento per capire perché una connessione wireless non va
Mostra le istruzioni per controllare il nome della rete in caso di guasto
Cerco una guida tecnica per diagnosticare la connessione
Mi interessa il manuale che tratta i problemi del resolver
Trova il documento di aiuto per analizzare i guasti della rete
Recupera la nota con le istruzioni di troubleshooting della connessione
Vorrei leggere come individuare un problema nel collegamento senza fili
La guida che aiuta a verificare il DNS quando manca la connettività
Mostrami il manuale sulle verifiche della rete wireless
Cerco le istruzioni per diagnosticare nome della rete e accesso
Trova la nota esplicativa per risolvere problemi del collegamento
Il documento per capire se il guasto dipende dal resolver o dalla rete
Find the wireless troubleshooting handbook
Retrieve the guide for diagnosing network names and resolvers
Show instructions for investigating connectivity failures
I need the manual explaining how to diagnose a wireless connection
Locate the troubleshooting note about the resolver and network name
Find the document that explains connectivity diagnosis
Show the guide rather than a saved network configuration
Recall the instructions for identifying wireless network problems
Retrieve the handbook about resolver troubleshooting
Find the note explaining checks for a failing connection
Non voglio applicare un profilo cerco la guida per capire il guasto
Mostra un documento con istruzioni di diagnosi del Wi-Fi
Cerco spiegazioni sulla risoluzione dei problemi non impostazioni salvate
Trova il manuale che insegna a controllare il resolver
Recupera le istruzioni per indagare una rete senza connessione
Il documento che spiega come riconoscere problemi del nome della rete
Vorrei la nota tecnica che guida alla verifica della connettività
Cerco il testo di supporto per il troubleshooting wireless
Mi serve il manuale di diagnostica del collegamento alla rete
Mostra la guida di connettività che non rappresenta una configurazione""",
"media": """Trova il filmato della barca al tramonto
Recupera il video della barca a vela nella baia con luce serale
Cerco il breve film del traghetto durante il tramonto
Mostra la ripresa della barca che attraversa la baia
Vorrei il video di navigazione nella luce dorata della sera
Ritrova il filmato con la barca a vela sull'acqua al calar del sole
Mi serve il breve video girato nella baia al tramonto
Individua la ripresa serale del traghetto
Cerca il film della barca in navigazione con luce dorata
Mostrami il video della barca che attraversava la baia la sera
Recupera il filmato di navigazione a vela al tramonto
Il breve video con una barca a vela nella luce del sole basso
Vorrei rivedere la ripresa della barca nella baia
Trova il filmato del traghetto con illuminazione serale
Cerco un video marittimo girato al tramonto
Ritrova la scena della barca a vela che passa sull'acqua
Mostra il film della traversata nella baia al calar del sole
La ripresa breve della navigazione nella luce calda della sera
Recupera il video con la barca e il cielo al tramonto
Cerco la scena filmata del traghetto nella baia
Find the short sailboat film at sunset
Retrieve the ferry video in golden evening light
Show the movie of a boat crossing the bay
I want the sailing clip filmed at the end of the day
Locate the short maritime video with warm sunset light
Find the sailboat crossing the bay in the evening
Retrieve the sunset ferry film
Show the boat video captured in golden light
Recall the short movie of sailing across the water
Find the evening clip with a boat in the bay
Mi interessa un contenuto video con una barca al tramonto
Trova la ripresa del mare con una barca a vela nella baia
Vorrei il filmato della traversata del traghetto la sera
Recupera la scena di navigazione con il sole che scende
Il video della barca nella luce dorata prima della notte
Mostra il breve film marino ambientato nella baia
Cerco la ripresa della barca mentre attraversa lo specchio d'acqua
Ritrova il contenuto multimediale della navigazione al tramonto
Mi serve il filmato serale della barca a vela
Il video della traversata nella baia illuminata dal tramonto""",
}
def main():
    rows = []
    for label, text in GROUPS.items():
        queries = text.splitlines()
        assert len(queries) == 40, (label, len(queries))
        rows.extend({"id":f"{label}-{i+1:02}","query":q,"expected_label":label}
                    for i,q in enumerate(queries))
    assert len(rows) == 200 and len({r["query"].casefold() for r in rows}) == 200
    output = Path(__file__).with_name("paraphrases-200.json")
    document = {"version":1,"origin":"authored synthetic paraphrases of the five existing M0 cards and supplied architecture use cases",
        "frozen_before_first_evaluation":True,"not_independent_human_evaluation":True,"queries":rows}
    raw = (json.dumps(document,ensure_ascii=False,indent=2)+"\n").encode()
    output.write_bytes(raw)
    print(hashlib.sha256(raw).hexdigest(), output.name)
if __name__ == "__main__": main()
