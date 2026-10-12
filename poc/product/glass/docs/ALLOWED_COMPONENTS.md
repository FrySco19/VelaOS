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
- Frost materials. Palettes Light, Dawn, and Deep only.
- Permitted components below, and no others.
- Never `shell=browser`. Never authority. Never replace the familiar toolbar.
- No chat-as-OS. No LLM-as-authority. No forging grants, selections, or authority.

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
| Objective surface | ephemeral overlay | allow |
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
- **Purpose:** Frosted top bar of the desktop: sail, VelaOS wordmark, app menus, Spaces dots, search, network, volume, battery, live date, and clock.
- **When allowed:** Never as a Glass component. The §7 shell draws it.
- **Must not:** Must not replace, hide, or restyle this bar. Must not move the clock, Spaces, or system status into a Glass panel.
- **Shell / toolbar:** This is the familiar toolbar. Vela Shell owns it. Glass sits above it and leaves it in place.
- **Intent:** No role.
- **Policy:** No role. The bar does not admit a task.

### Dock

- **Layer:** §7 base chrome
- **Verdict:** deny
- **Purpose:** Floating bottom-centre dock. Six monochrome glyphs, in order: Vela, Find, Spaces, Notes, Media, Settings (Italian labels Trova, Note, Impostazioni where the shell translates them). Frosted glass. The active app is a frosted tile plus a teal underline (`TEAL_BRIGHT` `0x34919B` in `draw_dock`).
- **When allowed:** Never as a Glass component.
- **Must not:** Must not add a second dock, a theme dock, or a glyph launcher. Must not paint the teal underline. The host page's `nav.dock` is a deny stand-in, not this dock.
- **Shell / toolbar:** Shell chrome next to the top bar. Not a Glass toolbar.
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
- **Purpose:** Hides the top bar and the dock. Entered with F11, the View menu, or `focus`. Esc exits.
- **When allowed:** Never toggled by Glass.
- **Must not:** Must not enter or exit Focus, and must not treat an open panel as the focused desktop.
- **Shell / toolbar:** Shell mode that hides the toolbar and the dock. Glass does not become the thing left on screen.
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
holds, they use frost, they follow Light / Dawn / Deep, and they go away.
Dismiss (Annulla or Chiudi) returns to the base. Nothing in this list persists
as chrome.

### Frost material

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** Translucent blurred surface. The §7 shell uses `glass_rrect` (blurred wallpaper plus tint) for the bar, the dock, and notification cards. An open panel uses the same idea — frost — so it reads as glass above the desktop rather than as a new opaque shell.
- **When allowed:** On a temporary overlay only, for as long as that panel is open.
- **Must not:** Must not frost a fake toolbar, a fake dock, a window frame, or a full-screen desktop. Frost is a material, not a license to redraw §7 chrome.
- **Shell / toolbar:** Borrows the shell's material. Does not become the toolbar.
- **Intent:** No role.
- **Policy:** No role. Frost carries no authority.

### Theme materials

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** Light, Dawn, and Deep are the only palettes an overlay may follow. The names are the §7 `theme_t` records (Light, Dawn, Deep).
- **When allowed:** When the shell has already selected one of the three, the open panel renders in that material.
- **Must not:** Must not invent a fourth theme. Must not treat the host page's preview hex values (`#2f6df6`, `#c2508a`, `#7aa2ff`) as the ISO palette. Those classes are named slots only.
- **Shell / toolbar:** The shell selects the theme. The panel does not own a switcher and does not put one on the toolbar.
- **Intent:** No role.
- **Policy:** No role.

### Objective surface

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** One field for the current objective. The host control is the input plus Interpreta. The shell's idle text is "Chiedi qualcosa...". Submit sends `{ token, utterance }` and nothing else.
- **When allowed:** While the shell is idle, as a temporary field above the base. It is not a resident bar. It closes when the panel is dismissed.
- **Must not:** Must not persist as a replacement toolbar. Must not grow into a chat transcript. Must not attach candidates, handles, versions, or grants to the submit body.
- **Shell / toolbar:** Sits above the toolbar. The toolbar stays the shell's. `ShellRuntime::submit` is the only sink.
- **Intent:** The utterance is input to Intent. Intent returns ranked candidates and does not authorize.
- **Policy:** Submit does not reach the PDP. No grant is issued for typing or for interpreting.

### Proposal panel

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** The shell's glass phase, shown as data: candidate index, title, version, score, sources, and `auto_resolve` (it stays false). A handle may be shown in hex so a person can audit the row. The page never sends that handle back.
- **When allowed:** Only while the shell is in the glass phase, and only for candidates the shell is actually showing.
- **Must not:** Must not let the page post a handle or version. Must not style the list as a policy decision. Must not hide that showing the rows authorizes nothing. Counters stay at zero until a gesture.
- **Shell / toolbar:** Rendered above the shell from shell state. Not an application dock and not a menu on the toolbar.
- **Intent:** Rows are Intent candidates (BM25, M0, RRF). Data only. The model does not pick the row.
- **Policy:** Showing the panel issues no grant, no frame, and no `UserSelection`. Admission happens only after a gesture, inside the shell.

### Gesture row

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** The only actions. Avvia and Rivedi piano take the index of a shown candidate. Annulla returns to idle and authorizes nothing. Chiudi dismisses a result or an error. Wire names: `avvia`, `rivedi`, `annulla`, `chiudi`.
- **When allowed:** Avvia and Rivedi piano only in the glass phase, and only on an index the panel is showing. Annulla only from that phase. Chiudi only from result or error. Any other action is rejected.
- **Must not:** Must not add actions (`approve_all`, grant, select, admit). Must not click a row that is not shown. Must not skip `ShellRuntime::click`. Must not send a handle with the click.
- **Shell / toolbar:** These are shell clicks. The toolbar does not gain Avvia or Rivedi piano. The shell re-derives `UserSelection { handle, expected_version }` from the displayed row.
- **Intent:** The index picks a shown Intent row. Intent does not choose and does not confirm.
- **Policy:** The shell asks the PDP with the selection it derived. The gesture is not itself a grant. Rivedi piano can admit without dispatch. Avvia can admit and dispatch. Annulla authorizes nothing. A wrong state or an unshown index is refused.

### Rationale note

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** Planner or model text, shown as an unverified note with no effect. The host page labels it as a note that does not execute. Untrusted text is inserted with `textContent`, never parsed as HTML.
- **When allowed:** Only as a label on the open proposal, and only when the shell supplies a rationale.
- **Must not:** Must not treat the note as a decision. Must not parse it as commands. Must not let it set `auto_resolve`, select a row, or call the PDP. A note that says the user already approved is still just a note.
- **Shell / toolbar:** Not chrome. The shell forwards the text only as `planner_note`.
- **Intent:** Intent or model output. Untrusted by construction.
- **Policy:** Ignored as authority. An AI-only path, with no user selection, is denied.

### Outcome panel

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** What the shell recorded after a gesture. Executed, reviewed, denied (stable code, stage, reason), nothing found, or an executor status. The selection the shell emitted may be shown for audit, including when the grant is valid until.
- **When allowed:** After the shell enters result or error, until Chiudi.
- **Must not:** Must not mint a grant, extend a TTL, or retry a denial. Must not restyle a denial as a model success. Must not present generated text as the original object. Must not offer a generic confirm that commits a further effect.
- **Shell / toolbar:** Displays shell state above the base. Does not replace the toolbar.
- **Intent:** No further proposal. The rationale is not rewritten into the decision.
- **Policy:** The text is the outcome the shell already produced. Glass does not admit, and it does not overturn a denial.

### Audit strip

- **Layer:** ephemeral overlay
- **Verdict:** allow
- **Purpose:** Read-only honesty next to the panel: recall-lane label, M0 status, how many selections were emitted, how many grants were issued, how many frames were signed, and the phase trace.
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
not grow. This pass does not restyle `assets/glass.html`.

| Path | Symbol | Verdict | Component | Note |
| --- | --- | --- | --- | --- |
| `src/lib.rs` | `GlassApp::view` | allow | Proposal panel | View model of shell state. Handles are hex for display only. |
| `src/lib.rs` | `POST /submit` | allow | Objective surface | Utterance only. Does not authorize. |
| `src/lib.rs` | `POST /click` | allow | Gesture row | `avvia`, `rivedi`, `annulla`, `chiudi`. Index of a shown row, or no index. |
| `src/lib.rs` | `other routes` | deny | Loopback bridge | Anything other than `/`, `/state`, `/submit`, `/click` is 404, including admit, grant, select, and execute. |
| `src/lib.rs` | `serve` | allow | Loopback bridge | Loopback HTTP for the host demo. Not a shell and not a browser. |
| `src/lib.rs` | `Lanes` | allow | Audit strip | Recall-lane status. Not a choice and not a grant. |
| `src/lib.rs` | `demo` | deny | Loopback bridge | Demo keys, corpus, and PCB seed. A host fixture, not a surface and not production authority. |
| `src/main.rs` | `main` | allow | Loopback bridge | Binds `127.0.0.1` only. |
| `src/catalog.rs` | `COMPONENTS` | allow | — | These rules. Not a surface. |
| `assets/glass.html` | `#q / #go` | allow | Objective surface | The field and Interpreta post `/submit`. |
| `assets/glass.html` | `header.glass` | deny | Top system bar | Persistent branded header. Stand-in of a toolbar. Do not grow it into the §7 bar. |
| `assets/glass.html` | `main#panel candidates` | allow | Proposal panel | Candidate rows while the shell is in the glass phase. |
| `assets/glass.html` | `gesture buttons` | allow | Gesture row | Avvia, Rivedi piano, Annulla, Chiudi. |
| `assets/glass.html` | `.note` | allow | Rationale note | Unverified model text. `textContent` only. No effect. |
| `assets/glass.html` | `result/error` | allow | Outcome panel | Renders the shell result or error. Does not mint the grant it displays. |
| `assets/glass.html` | `footer#foot` | allow | Audit strip | Lanes, M0, selection count, grant count, frame count, trace. Display only. |
| `assets/glass.html` | `nav.dock` | deny | Dock | A row of theme buttons. Not the six-glyph dock and not the teal underline. |
| `assets/glass.html` | `[data-theme]` | deny | Theme choice | Sets a body class for the preview. The product switcher is Welcome and Settings. |
| `assets/glass.html` | `backdrop-filter` | allow | Frost material | Blur on the open panel. Not a redraw of the bar or the dock. |
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

- **Rule:** Glass is temporary panels above the §7 base, not a second desktop. It does not draw window controls, the dock, Spaces, Focus, Welcome, or notifications.
- **Why:** Those pieces are the operating system the user already has. A frosted copy of them is a second desktop with none of the shell's authority boundaries, which is worse than no glass at all.
