"""UI strings (Italian default, English)."""
from __future__ import annotations

_LANG = None

S = {
    "app_name": ("Terminale IA", "AI Terminal"),
    "copy": ("Copia", "Copy"),
    "paste": ("Incolla", "Paste"),
    "select_all": ("Seleziona tutto", "Select all"),
    "ask_ai_selection": ("Chiedi all'IA sul testo selezionato", "Ask the AI about the selection"),
    "clear_screen": ("Pulisci lo schermo", "Clear screen"),
    "zoom_in": ("Testo più grande", "Bigger text"),
    "zoom_out": ("Testo più piccolo", "Smaller text"),
    "cancel": ("Annulla", "Cancel"),
    "run": ("Esegui", "Run"),
    "run_all": ("Esegui tutti in ordine", "Run all in order"),
    "run_anyway": ("Esegui comunque", "Run anyway"),
    "edit": ("Modifica", "Edit"),
    "done_edit": ("Fatto", "Done"),
    "skip": ("Salta", "Skip"),
    "stop": ("Ferma", "Stop"),
    "retry": ("Riprova", "Retry"),
    "preview": ("Anteprima", "Preview"),
    "preview_loading": ("Calcolo l'anteprima…", "Computing preview…"),
    "danger_title": ("Conferma comando pericoloso", "Confirm dangerous command"),
    "danger_heading": ("Questo comando può fare danni irreversibili", "This command can cause irreversible damage"),
    "danger_requested_by": ("Richiesto da: {caller}", "Requested by: {caller}"),
    "danger_ack": ("Ho letto e voglio eseguire proprio questo comando", "I have read it and want to run exactly this command"),
    "risk_safe": ("Sicuro", "Safe"),
    "risk_change": ("Modifica", "Changes"),
    "risk_danger": ("Pericoloso", "Dangerous"),
    "composer_placeholder": ("Descrivi cosa vuoi fare, per esempio «trova i file più grandi in Scaricati»",
                             "Describe what you want to do, e.g. “find the biggest files in Downloads”"),
    "composer_hint": ("Invio per inviare · Maiusc+Invio per andare a capo · $ davanti per un comando diretto",
                      "Enter to send · Shift+Enter for a new line · start with $ for a direct command"),
    "agent_mode": ("Modalità agente", "Agent mode"),
    "agent_tip": ("L'IA porta a termine il compito in più passi. Ogni comando aspetta la tua conferma.",
                  "The AI completes the task in several steps. Every command waits for your confirmation."),
    "panel_title": ("Assistente", "Assistant"),
    "thinking": ("Sto preparando i comandi…", "Preparing commands…"),
    "thinking_next": ("Penso al passo successivo…", "Thinking about the next step…"),
    "explaining": ("Analizzo l'errore…", "Analysing the error…"),
    "status_ok": ("{model}", "{model}"),
    "status_off": ("IA disattivata", "AI turned off"),
    "status_none": ("IA non configurata", "AI not set up"),
    "status_unreachable": ("Modello non raggiungibile", "Model not reachable"),
    "status_checking": ("Controllo l'IA…", "Checking AI…"),
    "setup_title": ("I suggerimenti dell'IA non sono attivi", "AI suggestions are not active"),
    "setup_disabled": ("L'assistente è disattivato in Impostazioni.", "The assistant is turned off in Settings."),
    "setup_cloud": ("Il servizio scelto è esterno, ma l'invio fuori dal computer non è consentito in Impostazioni → Privacy e memoria.",
                    "The chosen service is external, but sending data off this computer is not allowed in Settings → Privacy."),
    "setup_nokey": ("Manca la chiave del servizio IA esterno.", "The external AI service key is missing."),
    "setup_nomodel": ("Nessun modello IA configurato.", "No AI model configured."),
    "setup_unreachable": ("Il modello IA non risponde ({detail}). Se usi il modello locale, installalo o avvialo da Impostazioni → Intelligenza artificiale.",
                          "The AI model does not respond ({detail}). If you use the local model, install or start it in Settings → AI."),
    "setup_missing_model": ("Il modello «{model}» non è installato. Scaricalo da Impostazioni → Intelligenza artificiale.",
                            "Model “{model}” is not installed. Download it in Settings → AI."),
    "setup_auth": ("Il servizio IA ha rifiutato la chiave. Controllala in Impostazioni.", "The AI service rejected the key. Check it in Settings."),
    "setup_run_never": ("L'esecuzione di comandi da parte dell'IA è disattivata in Impostazioni.",
                        "Running commands from the AI is turned off in Settings."),
    "setup_bad": ("Il modello ha risposto in un formato non valido. Riprova o scegli un modello più capace.",
                  "The model replied in an invalid format. Retry or choose a more capable model."),
    "setup_still_works": ("Il terminale funziona normalmente. Puoi anche scrivere $ seguito da un comando per vederne rischio e anteprima prima di eseguirlo.",
                          "The terminal works normally. You can also type $ followed by a command to see its risk and preview before running it."),
    "det_refused": ("il servizio non è in esecuzione", "the service is not running"),
    "det_timeout": ("tempo di risposta scaduto", "response timed out"),
    "det_dns": ("indirizzo del servizio non trovato", "service address not found"),
    "det_net": ("rete non raggiungibile", "network unreachable"),
    "open_settings": ("Apri Impostazioni IA", "Open AI settings"),
    "no_commands": ("Nessun comando proposto.", "No command proposed."),
    "busy": ("Il terminale sta eseguendo un altro programma. Attendi che finisca o premi Ctrl+C nel terminale.",
             "The terminal is running another program. Wait for it to finish or press Ctrl+C in the terminal."),
    "running": ("In esecuzione…", "Running…"),
    "exit_ok": ("Completato", "Done"),
    "exit_fail": ("Errore (codice {rc})", "Failed (exit code {rc})"),
    "interrupted": ("Interrotto", "Interrupted"),
    "show_all": ("Mostra tutto ({n} righe)", "Show all ({n} lines)"),
    "show_less": ("Mostra meno", "Show less"),
    "copy_output": ("Copia output", "Copy output"),
    "explain_error": ("Spiega l'errore", "Explain the error"),
    "no_output": ("Nessun output.", "No output."),
    "last_failed": ("Il comando «{cmd}» è terminato con un errore (codice {rc}).", "Command “{cmd}” failed (exit code {rc})."),
    "history": ("Cronologia", "History"),
    "history_search": ("Cerca per intento o comando…", "Search by intent or command…"),
    "history_empty": ("Nessuna voce nella cronologia.", "No history yet."),
    "history_reuse": ("Riproponi", "Propose again"),
    "history_clear": ("Cancella cronologia", "Clear history"),
    "history_clear_q": ("Cancellare tutta la cronologia del Terminale IA?", "Delete the whole AI Terminal history?"),
    "new_tab": ("Nuova scheda", "New tab"),
    "close_tab": ("Chiudi scheda", "Close tab"),
    "toggle_panel": ("Mostra o nascondi l'assistente", "Show or hide the assistant"),
    "menu": ("Altro", "More"),
    "about": ("Informazioni", "About"),
    "about_text": ("Terminale IA di Vela. Un terminale vero (bash) con un assistente che propone comandi, mostra rischi e anteprime ed esegue solo dopo la tua conferma.",
                   "Vela AI Terminal. A real terminal (bash) with an assistant that proposes commands, shows risks and previews and runs only after you confirm."),
    "goal": ("Obiettivo", "Goal"),
    "plan": ("Piano", "Plan"),
    "agent_done": ("Compito concluso", "Task finished"),
    "agent_stopped": ("Agente fermato", "Agent stopped"),
    "agent_limit": ("Raggiunto il limite di {n} passi: mi fermo qui.", "Reached the limit of {n} steps: stopping here."),
    "you": ("Tu", "You"),
    "manual": ("Comando scritto da te", "Command typed by you"),
    "sent_cloud": ("Inviato a un servizio esterno", "Sent to an external service"),
    "partial_preview": ("anteprima parziale", "partial preview"),
    "shell_exited": ("La shell è terminata (codice {rc}). Premi Invio per riavviarla.", "The shell exited (code {rc}). Press Enter to restart it."),
    "welcome_title": ("Scrivi qui cosa vuoi fare", "Type what you want to do here"),
    "welcome_body": ("L'assistente propone i comandi, ti spiega cosa fanno, ne valuta il rischio e mostra un'anteprima. Si eseguono nel terminale a sinistra solo quando premi Esegui.",
                     "The assistant proposes commands, explains them, rates their risk and shows a preview. They run in the terminal on the left only when you press Run."),
    "examples": ("Esempi", "Examples"),
    "ex1": ("Quanto spazio libero ho sul disco?", "How much free disk space do I have?"),
    "ex2": ("Trova i 10 file più grandi in Scaricati", "Find the 10 biggest files in Downloads"),
    "ex3": ("Sposta gli screenshot di oggi in una cartella", "Move today's screenshots into a folder"),
}


def set_lang(lang):
    global _LANG
    _LANG = lang


def lang():
    global _LANG
    if _LANG is None:
        try:
            from . import aiconfig
            _LANG = aiconfig.language()
        except Exception:
            _LANG = "it"
    return _LANG


def tr(key, lang_=None):
    pair = S.get(key)
    if not pair:
        return key
    return pair[0] if (lang_ or lang()) == "it" else pair[1]
