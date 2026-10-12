# Glass allowed components (ephemeral surfaces v0)

Glass draws **temporary panels above** the desktop. It is not a second desktop.

The desktop is the ISO visual base in section 7 of the OS vision brief, as closed
in commit [`60a5b8b9ec1e566f20c450366e8fa16c10ea34dd`](https://github.com/FrySco19/VelaOS/commit/60a5b8b9ec1e566f20c450366e8fa16c10ea34dd)
(`feat(os): section 7 desktop shell in the VirtualBox ISO (v0.2)`). The files
cited below live in that commit under `os/x86_64/`. This branch does not vendor
them and does not modify `os/x86_64/`.

From `os/x86_64/README.md` at that commit, the base is:

> top system bar + floating bottom-centre dock with six monochrome glyphs on frosted glass
> (active app = frosted tile + teal underline), windows with right-aligned – □ ×, Focus mode
> (hides bar and dock), notifications top-right, Spaces = virtual desktops, themes Light / Dawn / Deep,
> English by default with Italian selectable on the first-boot Welcome card.

That image has no Linux, no browser, and no host web page. `os/x86_64/src/kernel.c`
draws the pieces this catalog refuses to reimplement: `glass_rrect` (frost),
`draw_dock` (teal underline `TEAL_BRIGHT` `0x34919B` on the active glyph),
window caption buttons – □ ×, top-right notifications, `theme_t` Light / Dawn / Deep,
and the Welcome card (`Welcome to VelaOS` / `Benvenuto in VelaOS`).

The normative lists are [`src/catalog.rs`](../src/catalog.rs). A crate test fails
if a component, anti-pattern, or inventory row is missing from this file.
`GlassApp::handle` enforces the submit fields, click fields, and click actions
from that module.

## How to read a row

Each component records its name, purpose, when it is allowed, what it must not
do, how it relates to the shell and toolbar, how it relates to Intent, how it
relates to Policy, and whether it is **§7 base chrome** or an **ephemeral overlay**.
The loopback page is a third kind: **host-demo transport**. It is neither chrome
nor an overlay, and it is not the shell.

Verdict **allow** means Glass may render or carry that row. Verdict **deny** means
Glass must not own it. A deny row in the inventory can still exist in the host
demo as a stand-in; the stand-in is not permission to grow it.

## Hard bounds

- Ephemeral surface, rendered from shell and intent context.
- Frost and type from Art tokens `ef8ec09` (`docs/art/TOKENS_v0.md` on `art/tokens-v0`). Palettes Light, Dawn, and Deep only.
- Permitted components below, and no others. The side panel is Shell chrome and is not a component here.
- Never `shell=browser`. Never authority. Never replace the familiar toolbar.
- No chat-as-OS. No LLM-as-authority. No forging grants, selections, or authority.

## Shell z-order and hit-testing

Glass fills ephemeral content. It does not draw Shell chrome. The side panel is Shell chrome with three levels (full, reduced, hidden). Its frost is the permanent shell stack, blur 24px and opacity 0.72. Glass does not own that frost and does not draw the panel. Glass supplies Proposal, gesture, and outcome content only when the Shell is showing the panel.

1. **Normal mode.** The top bar and the dock are permanent chrome above Glass. Both stay visible and clickable.
2. **Hit-testing.** Any point in the top-bar bounds or the dock bounds goes to the Shell, not to Glass.
3. **Content band.** Glass may sit above windows and desktop content. It must not cover the floating dock's interactive area or the top-bar band.
4. **Focus (F11).** The shell hides the bar and the dock. Ephemeral Glass may remain on content. It must not fake a dock or a toolbar, and it must not reintroduce a second dock. Focus does not delete the shell side-panel slot.
5. **Art tokens.** Ephemeral frost and type consume `ef8ec09`: Inter; ephemeral blur 20px; ephemeral opacity 0.88; accent `#2EC4B6` on the primary gesture only. Permanent frost (blur 24px, opacity 0.72) stays on Shell chrome. Glass does not fork the teal dock underline or the six 28px monochrome glyphs.
6. **Dismissal.** A Glass panel leaves the bar and the dock hittable, or it is dismissible without a modal model. Annulla and Chiudi dismiss. There is no modal that traps hits over Shell chrome.

Vela Shell owns the toolbar, the dock, and the trusted click. Intent proposes
candidates and does not authorize. Policy admits or denies the selection the
shell derived. Glass shows the proposal and the outcome.

## Scan

| Component | Layer | Verdict |
| --- | --- | --- |
| Top system bar | §7 base chrome | deny |
| Dock | §7 base chrome | deny |
| Window controls | §7 base chrome | deny |
| Focus mode | §7 base chrome | deny |
| Notifications | §7 base chrome | deny |
| Spaces | §7 base chrome | deny |
| Welcome | §7 base chrome | deny |
| Theme choice | §7 base chrome | deny |
| Frost material | ephemeral overlay | allow |
| Theme materials | ephemeral overlay | allow |
| Objective surface | host-demo transport | deny |
| Proposal panel | ephemeral overlay | allow |
| Gesture row | ephemeral overlay | allow |
| Rationale note | ephemeral overlay | allow |
| Outcome panel | ephemeral overlay | allow |
| Audit strip | ephemeral overlay | allow |
| Loopback bridge | host-demo transport | allow |

## §7 base chrome

These rows are the familiar desktop. Glass does not draw them. No symbol under
`poc/product/glass` implements window controls, Focus, notifications, Spaces, or
Welcome. The header and the theme row in `assets/glass.html` are host-demo
stand-ins, classified deny in the inventory. They are not this chrome.

### Top system bar

- **Layer:** §7 base chrome
- **Verdict:** deny
- **Purpose:** Frosted top bar of the desktop: sail, VelaOS wordmark, app menus, Spaces dots, search, network, volume, battery, live date, and clock. The objective is composed in the Shell search field.
- **When allowed:** Never as a Glass component. The §7 shell draws it.
- **Must not:** Must not replace, hide, cover, or restyle this bar. Must not move the clock, Spaces, search, or system status into a Glass panel.
- **Shell / toolbar:** This is the familiar toolbar. Vela Shell owns it. In normal mode it stays visible and clickable above Glass. Hits inside the bar belong to the Shell.
- **Intent:** No role.
- **Policy:** No role. The bar does not admit a task.

### Dock

- **Layer:** §7 base chrome
- **Verdict:** deny
- **Purpose:** Floating bottom-centre dock. Six monochrome glyphs at 28px, in order: Vela, Find, Spaces, Notes, Media, Settings (Italian labels Trova, Note, Impostazioni where the shell translates them). Frosted glass. The active app is a frosted tile plus a teal underline (`TEAL_BRIGHT` `0x34919B` in `draw_dock`; Art accent `#2EC4B6`). Shell only.
- **When allowed:** Never as a Glass component.
- **Must not:** Must not add a second dock, a theme dock, or a glyph launcher. Must not paint the 28px glyphs or the teal underline. The host page's `nav.dock` is a deny stand-in, not this dock.
- **Shell / toolbar:** Shell chrome next to the top bar. In normal mode it stays visible and clickable. Hits inside the dock belong to the Shell. Not a Glass toolbar.
- **Intent:** No role.
- **Policy:** No role. Launching an app from the dock is a shell gesture, not a PDP admission from Glass.

### Window controls

- **Layer:** §7 base chrome
- **Verdict:** deny
- **Purpose:** Right-aligned minimise (–), maximise (□), and close (×) on each window title row.
- **When allowed:** Never as a Glass component. Windows belong to the §7 shell.
- **Must not:** Must not frame a Glass panel as a managed window or invent another –□×. Dismissing a panel is Chiudi or Annulla, which are shell clicks, not window chrome.
- **Shell / toolbar:** Shell window manager. Not the toolbar and not Glass.
- **Intent:** No role.
- **Policy:** No role. Closing a window is not a policy decision.

### Focus mode

- **Layer:** §7 base chrome
- **Verdict:** deny
- **Purpose:** Hides the top bar and the dock. Entered with F11, the View menu, or `focus`. Esc exits. The shell side-panel slot stays.
- **When allowed:** Never toggled by Glass.
- **Must not:** Must not enter or exit Focus, treat an open panel as the focused desktop, fake a dock or toolbar, or reintroduce a second dock.
- **Shell / toolbar:** Shell mode that hides the toolbar and the dock. Ephemeral Glass may remain on content. Glass does not become the thing left on screen.
- **Intent:** No role.
- **Policy:** No role.

### Notifications

- **Layer:** §7 base chrome
- **Verdict:** deny
- **Purpose:** Frosted notices stacked at the top-right. After Welcome the shell posts "Welcome aboard" / "Benvenuto a bordo".
- **When allowed:** Never posted by Glass. The shell notifier owns that corner.
- **Must not:** Must not draw a notification stack, toast, or banner in the top-right. A policy denial is an outcome panel, not a system notification.
- **Shell / toolbar:** Shell chrome, separate from the toolbar.
- **Intent:** No role.
- **Policy:** No role. A notification is not an admission and not a grant.

### Spaces

- **Layer:** §7 base chrome
- **Verdict:** deny
- **Purpose:** Virtual desktops. Dots in the top bar, Ctrl+Alt+Left/Right, and the Spaces app. Four spaces in the §7 shell.
- **When allowed:** Never managed by Glass.
- **Must not:** Must not create desktops, move windows between spaces, or present a panel as a space.
- **Shell / toolbar:** Shell owns Spaces. The dots live on the toolbar.
- **Intent:** No role.
- **Policy:** No role.

### Welcome

- **Layer:** §7 base chrome
- **Verdict:** deny
- **Purpose:** First-boot card. English by default, Italiano selectable. Appearance offers Light / Dawn / Deep (Chiaro / Alba / Profondo). Language, privacy ("On this device"), and accessibility (larger text, reduce motion) sit on the same card. The title is "Welcome to VelaOS" / "Benvenuto in VelaOS". Continue is "Make yourself at home" / "Mettiti a tuo agio".
- **When allowed:** Never shown by Glass. First boot belongs to the §7 shell.
- **Must not:** Must not replace Welcome, choose the locale, or present onboarding as a chat.
- **Shell / toolbar:** Shell first-boot surface, shown before the toolbar is the workspace.
- **Intent:** No role. Welcome is not an objective field.
- **Policy:** No role. Continuing Welcome grants nothing.

### Theme choice

- **Layer:** §7 base chrome
- **Verdict:** deny
- **Purpose:** The active theme is one of Light, Dawn, or Deep, chosen on the Welcome card or in Settings (`theme light`, `theme dawn`, `theme deep`).
- **When allowed:** Never switched by a Glass control. An overlay may only follow the choice the shell already made.
- **Must not:** Must not ship a theme dock, a settings window, or a second palette editor. `[data-theme]` on the host page is a deny stand-in.
- **Shell / toolbar:** Welcome and Settings own the choice. The dock is not a theme switcher.
- **Intent:** No role.
- **Policy:** No role.

## Ephemeral overlays

These are the panels Glass may draw. They open from context the shell already
holds, they use ephemeral frost, they follow Light / Dawn / Deep, and they go
away. Dismiss (Annulla or Chiudi) returns to the base. Nothing in this list
persists as chrome. Objective surface is the exception filed here: composition
input belongs to the Shell top-bar search, and the host field is a denied
stand-in. The Proposal panel shows the utterance.

### Frost material

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** Ephemeral material from Art tokens `ef8ec09`. Blur 20px, saturate 140%, opacity 0.88, the theme frost-tint, a hairline, a top sheen, and the shared shadow. Type on the overlay is Inter.
- **When allowed:** On a temporary overlay only, for as long as that panel is open.
- **Must not:** Must not frost a fake toolbar, a fake dock, a side panel, a window frame, or a full-screen desktop. Must not paint Shell permanent frost (blur 24px, opacity 0.72). Must not fork the teal underline or the six 28px glyphs.
- **Shell / toolbar:** The bar, the dock, and the side panel keep permanent Shell frost. Glass does not become the toolbar.
- **Intent:** No role.
- **Policy:** No role. Frost carries no authority.

### Theme materials

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** Light, Dawn, and Deep are the only palettes an overlay may follow. The names are the §7 `theme_t` records. Colours are the Art token set. Deep is the night-sea home. The only Glass accent is teal `#2EC4B6`, and only on the primary gesture.
- **When allowed:** When the shell has already selected one of the three, the open panel renders in that material.
- **Must not:** Must not invent a fourth theme, a second accent, or a dock underline. Theme classes are named slots. They are not a palette editor.
- **Shell / toolbar:** The shell selects the theme. The panel does not own a switcher and does not put one on the toolbar.
- **Intent:** No role.
- **Policy:** No role.

### Objective surface

- **Layer:** host-demo transport
- **Verdict:** deny
- **Purpose:** Composition input is the Shell top-bar search, not a Glass field. The host control `#q` plus Interpreta is a loopback stand-in. The Proposal panel shows the utterance as its lead line, in body type. Submit from the stand-in sends `{ token, utterance }` and nothing else.
- **When allowed:** Never as product Glass chrome. The host demo may keep the stand-in. It is not a resident bar.
- **Must not:** Must not grow a Glass objective field into a toolbar replacement. Must not become a chat transcript. Must not attach candidates, handles, versions, or grants to the submit body.
- **Shell / toolbar:** The familiar input is the Shell top-bar search. Glass must not replace that toolbar. `ShellRuntime::submit` is the sink for the utterance the shell already composed.
- **Intent:** The utterance reaches Intent from the shell field. Intent returns ranked candidates and does not authorize.
- **Policy:** Submit does not reach the PDP. No grant is issued for typing or for interpreting.

### Proposal panel

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** The shell's glass phase, shown as data. The objective lead is the utterance, body type, theme text colour, not teal and not muted. Rows show candidate index, title, version, score, sources, and the handle in hex so a person can audit the row. `auto_resolve` stays false. The page never sends that handle back.
- **When allowed:** Only while the shell is in the glass phase and is showing the panel, and only for candidates the shell is actually showing.
- **Must not:** Must not let the page post a handle or version. Must not collect the objective in this panel. Must not style the list as a policy decision. Must not hide that showing the rows authorizes nothing. Counters stay at zero until a gesture.
- **Shell / toolbar:** Content above windows and desktop content, never over the top-bar band or the dock hit area. Not an application dock, not the side panel, and not a menu on the toolbar.
- **Intent:** Rows are Intent candidates (BM25, M0, RRF). Data only. The model does not pick the row.
- **Policy:** Showing the panel issues no grant, no frame, and no `UserSelection`. Admission happens only after a gesture, inside the shell.

### Gesture row

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** The only actions. Avvia is the one teal primary (`#2EC4B6`) and takes the index of a shown candidate. Rivedi piano, Annulla, and Chiudi stay neutral. Annulla returns to idle and authorizes nothing. Chiudi dismisses a result or an error. Wire names: `avvia`, `rivedi`, `annulla`, `chiudi`. The click body is `{ action, candidate index }` and never a handle.
- **When allowed:** Avvia and Rivedi piano only in the glass phase, and only on an index the panel is showing. Annulla only from that phase. Chiudi only from result or error. Any other action is rejected.
- **Must not:** Must not add actions (`approve_all`, grant, select, admit). Must not click a row that is not shown. Must not skip `ShellRuntime::click`. Must not send a handle with the click.
- **Shell / toolbar:** These are shell clicks. The toolbar does not gain Avvia or Rivedi piano. The shell re-derives `UserSelection { handle, expected_version }` from the displayed row.
- **Intent:** The index picks a shown Intent row. Intent does not choose and does not confirm.
- **Policy:** The shell asks the PDP with the selection it derived. The gesture is not itself a grant. Rivedi piano can admit without dispatch. Avvia can admit and dispatch. Annulla authorizes nothing. A wrong state or an unshown index is refused.

### Rationale note

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** Planner or model text, shown as an unverified caption with no effect and no teal. The host page labels it as a note that does not execute. Untrusted text is inserted with `textContent`, never parsed as HTML.
- **When allowed:** Only as a label on the open proposal, and only when the shell supplies a rationale.
- **Must not:** Must not treat the note as a decision. Must not parse it as commands. Must not let it set `auto_resolve`, select a row, or call the PDP. A note that says the user already approved is still just a note.
- **Shell / toolbar:** Not chrome. The shell forwards the text only as `planner_note`.
- **Intent:** Intent or model output. Untrusted by construction.
- **Policy:** Ignored as authority. An AI-only path, with no user selection, is denied.

### Outcome panel

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** What the shell recorded after a gesture, in body-strong type. Executed, reviewed, denied (stable code, stage, reason), nothing found, or an executor status. The selection the shell emitted may be shown for audit, including when the grant is valid until. Outcome text is not a second accent.
- **When allowed:** After the shell enters result or error, until Chiudi.
- **Must not:** Must not mint a grant, extend a TTL, or retry a denial. Must not restyle a denial as a model success. Must not present generated text as the original object. Must not offer a generic confirm that commits a further effect.
- **Shell / toolbar:** Displays shell state above the base. Does not replace the toolbar.
- **Intent:** No further proposal. The rationale is not rewritten into the decision.
- **Policy:** The text is the outcome the shell already produced. Glass does not admit, and it does not overturn a denial.

### Audit strip

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** Read-only caption next to the panel: recall-lane label, M0 status, how many selections were emitted, how many grants were issued, how many frames were signed, and the phase trace. Muted metadata only.
- **When allowed:** Beside an open panel, or on the host preview, strictly as a display of counters the shell already holds.
- **Must not:** Must not be buttons. Must not be a way to issue a grant or sign a frame. A rising counter is a report, not a control.
- **Shell / toolbar:** Reports shell counters. Not toolbar buttons and not dock glyphs.
- **Intent:** May name which recall lanes ran. That is provenance, not a choice of what to run.
- **Policy:** Displays counts the shell and the PDP already incremented. Does not increment them.

## Host-demo transport

The running `vela-glass` binary is a loopback page in front of `ShellRuntime`.
That transport is allowed as a demonstration. It is not the product shell, and
the §7 ISO does not serve it.

### Loopback bridge

- **Layer:** host-demo transport
- **Verdict:** allow
- **Purpose:** `GET /` serves the page and injects a per-run token. `GET /state` returns the view model. `POST /submit` and `POST /click` are the only mutations. The listener binds `127.0.0.1`. Host is pinned to that port. A foreign Origin is refused. The body must be `application/json` and at most 16 KiB.
- **When allowed:** Only as an untrusted host demonstration in front of `ShellRuntime`. Not as the desktop, and not inside the ISO image.
- **Must not:** Must not be called the shell. Must not bind anything but loopback. Must not add `/admit`, `/grant`, `/select`, `/selection`, `/execute`, `/api/admit`, or `/click/0`. Must not accept a body field outside `token` + `utterance` on submit, or `token` + `action` + `candidate` on click.
- **Shell / toolbar:** Talks to the shell through `submit` and `click`. Is not the toolbar. Is not a browser shell. The page is untrusted; principle 8 of the project applies.
- **Intent:** Carries the utterance in and the candidate view out. Does not rank by itself and does not add authority to a ranking.
- **Policy:** Cannot admit. The PDP stays private behind the shell click. Unknown routes are 404. Forged fields are 400. Wrong state is 409.

## Inventory of `poc/product/glass`

What the crate contains today, classified. Allow rows are the overlay and the
transport. Deny rows are either banned paths or host-demo stand-ins that must
not grow. Ephemeral panels in `assets/glass.html` consume the Art tokens (Inter, ephemeral frost, teal on Avvia only).

| Path | Symbol | Verdict | Component | Note |
| --- | --- | --- | --- | --- |
| `src/lib.rs` | `GlassApp::view` | allow | Proposal panel | View model of shell state. Handles are hex for display only. |
| `src/lib.rs` | `POST /submit` | allow | Loopback bridge | Host-demo transport of the utterance only. Product composition input is the Shell top-bar search. Does not authorize. |
| `src/lib.rs` | `POST /click` | allow | Gesture row | `avvia`, `rivedi`, `annulla`, `chiudi`. Index of a shown row, or no index. |
| `src/lib.rs` | `other routes` | deny | Loopback bridge | Anything other than `/`, `/state`, `/submit`, `/click` is 404, including admit, grant, select, and execute. |
| `src/lib.rs` | `serve` | allow | Loopback bridge | Loopback HTTP for the host demo. Not a shell and not a browser. |
| `src/lib.rs` | `Lanes` | allow | Audit strip | Recall-lane status. Not a choice and not a grant. |
| `src/lib.rs` | `demo` | deny | Loopback bridge | Demo keys, corpus, and PCB seed. A host fixture, not a surface and not production authority. |
| `src/main.rs` | `main` | allow | Loopback bridge | Binds `127.0.0.1` only. |
| `src/catalog.rs` | `COMPONENTS` | allow | — | These rules. Not a surface. |
| `assets/glass.html` | `#q / #go` | deny | Objective surface | Host-demo stand-in of the Shell top-bar search. Not product Glass chrome. Do not grow it into a toolbar. |
| `assets/glass.html` | `header.glass` | deny | Top system bar | Persistent branded header. Stand-in of a toolbar. Do not grow it into the §7 bar. |
| `assets/glass.html` | `main#panel candidates` | allow | Proposal panel | Candidate rows in the glass phase. Objective lead is the utterance. Handle hex is display only. |
| `assets/glass.html` | `gesture buttons` | allow | Gesture row | Avvia, Rivedi piano, Annulla, Chiudi. |
| `assets/glass.html` | `.note` | allow | Rationale note | Unverified model text. `textContent` only. No effect. |
| `assets/glass.html` | `result/error` | allow | Outcome panel | Renders the shell result or error. Does not mint the grant it displays. |
| `assets/glass.html` | `footer#foot` | allow | Audit strip | Lanes, M0, selection count, grant count, frame count, trace. Display only. |
| `assets/glass.html` | `nav.dock` | deny | Dock | A row of theme buttons. Not the six-glyph dock and not the teal underline. |
| `assets/glass.html` | `[data-theme]` | deny | Theme choice | Sets a body class for the preview. The product switcher is Welcome and Settings. |
| `assets/glass.html` | `backdrop-filter` | allow | Frost material | Ephemeral blur 20px and saturate 140% on the open panel. Not Shell permanent frost, and not a redraw of bar, dock, or side panel. |
| `assets/glass.html` | `body theme classes` | allow | Theme materials | Named slots Light, Dawn, Deep. Preview colours are not the ISO palette. |
| `examples/m0_request.rs` | `m0_request` | deny | — | Dev fixture that prints an M0 request. Not a Glass surface. |

`tests/glass_http.rs` is not a surface. It locks the same bans: forged fields
are 400, forbidden routes are 404, and an AI-only path is shown but does not act.

## Anti-patterns

These are failures, not unfinished features. A patch that does one of them is
outside Glass even if the pixels look frosted.

### shell-equals-browser

**Shell is not a browser.**

- **Rule:** Never `shell=browser`. The loopback page is an untrusted host demo. The §7 desktop has no browser and no HTTP server. Do not promote the page to a shell, and do not render the desktop inside a browser as the product.
- **Why:** Authority lives in the shell and the PDP. HTML is on the untrusted side of that boundary. A browser tab that looks like a desktop is a second shell, which this catalog forbids.

### glass-as-authority

**Glass is not an authority.**

- **Rule:** Never authority. Glass does not admit, grant, sign, select, or execute. There is no admit, grant, select, or execute route. Showing candidates authorizes nothing.
- **Why:** The only path to the PDP is a shell click on a shown candidate. The page cannot hold a key, a biscuit, or a frame.

### replace-toolbar

**Do not replace the toolbar.**

- **Rule:** Never replace the familiar toolbar. The §7 top bar and the dock belong to Vela Shell. Glass panels sit above that chrome. They do not remove it or substitute a header for it.
- **Why:** The header in the host page (`header.glass`) is already a deny stand-in. Extending it with menus, a clock, Spaces, or status would steal the toolbar from the shell.

### chat-as-os

**Chat is not the OS.**

- **Rule:** No chat-as-OS. No thread, no assistant that is the desktop, no conversation as the workspace. One objective, a proposal, a gesture, a result. Dismiss returns to the base.
- **Why:** The product is a goal, a proposal, and a selection. An endless transcript would hide the shell and make the model the place where work happens.

### llm-as-authority

**The model is not an authority.**

- **Rule:** No LLM-as-authority. Rankings and rationale are proposals. Prompt text that claims the user approved, or that demands auto-resolve, is unverified text with no effect. Policy denies an AI-only path.
- **Why:** Intent may add a row and a note. It cannot emit `UserSelection`, and it cannot flip `auto_resolve`. The rationale note exists so that text stays visible and powerless.

### forge-authority

**Do not forge grants, selections, or authority.**

- **Rule:** No forging grants, selections, or authority. The client cannot send a handle, version, selection, frame, grant, or ttl. The shell re-derives `UserSelection` from the shown row. Unknown JSON fields are rejected.
- **Why:** If the page could name the object, the user would not be the one selecting it, and the PDP would be admitting a story instead of a gesture.

### second-desktop

**Glass is not a second desktop.**

- **Rule:** Glass is temporary panels above the §7 base, not a second desktop. It does not draw window controls, the dock, the side panel, Spaces, Focus, Welcome, or notifications.
- **Why:** Those pieces are the operating system the user already has. A frosted copy of them is a second desktop with none of the shell's authority boundaries, which is worse than no glass at all.
