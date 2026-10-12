//! Allowed Glass surfaces and the bans that keep them ephemeral.
//!
//! Human catalog: `docs/ALLOWED_COMPONENTS.md`.
//! Glass draws temporary panels above the ISO VB §7 desktop
//! (commit `60a5b8b9ec1e566f20c450366e8fa16c10ea34dd`). It does not own
//! that chrome, and it is not an authority.
//!
//! [`SUBMIT_FIELDS`], [`CLICK_FIELDS`], and [`ALLOWED_CLICK_ACTIONS`] are the
//! wire allowlists [`crate::GlassApp::handle`] enforces.

/// Where a component lives relative to the desktop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// ISO VB §7 chrome. The desktop shell owns it. Glass must not draw it.
    BaseChrome,
    /// Temporary panel above the base, rendered from shell / intent context.
    EphemeralOverlay,
    /// Untrusted host-demo transport. Not chrome and not a shell.
    HostDemoTransport,
}

/// Whether this crate may treat the row as a Glass surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Permitted Glass surface or untrusted transport.
    Allow,
    /// Not a Glass surface. Base chrome, a host-demo stand-in, or a banned path.
    Deny,
}

/// One catalog row. The markdown file restates the same name.
#[derive(Clone, Copy, Debug)]
pub struct Component {
    pub name: &'static str,
    pub purpose: &'static str,
    pub when_allowed: &'static str,
    pub must_not: &'static str,
    pub shell_toolbar: &'static str,
    pub intent: &'static str,
    pub policy: &'static str,
    pub layer: Layer,
    pub verdict: Verdict,
}

/// A ban. The id is the markdown heading.
#[derive(Clone, Copy, Debug)]
pub struct AntiPattern {
    pub id: &'static str,
    pub name: &'static str,
    pub rule: &'static str,
}

/// One existing file or symbol under `poc/product/glass`, classified.
///
/// `component` is a [`Component::name`], or `""` when the row is not a surface.
#[derive(Clone, Copy, Debug)]
pub struct InventoryItem {
    pub path: &'static str,
    pub symbol: &'static str,
    pub verdict: Verdict,
    pub component: &'static str,
    pub note: &'static str,
}

/// JSON keys `POST /submit` may carry. Anything else is rejected.
pub const SUBMIT_FIELDS: &[&str] = &["token", "utterance"];

/// JSON keys `POST /click` may carry. Anything else is rejected.
pub const CLICK_FIELDS: &[&str] = &["token", "action", "candidate"];

/// Click actions the shell accepts. The wire name `rivedi` is the
/// "Rivedi piano" gesture. No other action is a Glass gesture.
pub const ALLOWED_CLICK_ACTIONS: &[&str] = &["avvia", "rivedi", "annulla", "chiudi"];

/// Paths that must stay absent. They would skip the shell click.
pub const FORBIDDEN_ROUTES: &[&str] = &[
    "/admit",
    "/grant",
    "/select",
    "/selection",
    "/execute",
    "/api/admit",
    "/click/0",
];

/// Body fields that would forge a selection, grant, or frame.
pub const FORBIDDEN_FIELDS: &[&str] = &[
    "handle",
    "expected_version",
    "selection",
    "version",
    "frame",
    "grant",
    "ttl",
];

const BASE: Layer = Layer::BaseChrome;
const OVER: Layer = Layer::EphemeralOverlay;
const HOST: Layer = Layer::HostDemoTransport;
const ALLOW: Verdict = Verdict::Allow;
const DENY: Verdict = Verdict::Deny;

/// §7 base chrome (deny) and the ephemeral surfaces Glass may draw (allow).
pub static COMPONENTS: &[Component] = &[
    Component {
        name: "Top system bar",
        purpose: "Frosted top bar: sail, VelaOS wordmark, app menus, Spaces dots, search, network, volume, battery, date, and clock.",
        when_allowed: "Never drawn by Glass. The §7 shell draws it on the base.",
        must_not: "Must not replace, hide, or restyle this bar, and must not move the clock, Spaces, or status into a panel.",
        shell_toolbar: "This is the familiar toolbar. Vela Shell owns it.",
        intent: "No role.",
        policy: "No role. The bar does not admit a task.",
        layer: BASE,
        verdict: DENY,
    },
    Component {
        name: "Dock",
        purpose: "Floating bottom-centre dock. Six monochrome glyphs (Vela, Find, Spaces, Notes, Media, Settings) on frost. The active app is a frosted tile plus a teal underline (0x34919B).",
        when_allowed: "Never drawn by Glass.",
        must_not: "Must not add a second dock, a theme dock, or a glyph launcher, and must not paint the teal underline.",
        shell_toolbar: "Shell chrome beside the top bar. Not a Glass toolbar.",
        intent: "No role.",
        policy: "No role.",
        layer: BASE,
        verdict: DENY,
    },
    Component {
        name: "Window controls",
        purpose: "Right-aligned minimise (–), maximise (□), and close (×) on each window title row.",
        when_allowed: "Never drawn by Glass. Windows belong to the §7 shell.",
        must_not: "Must not frame a panel as a managed window or invent its own –□×.",
        shell_toolbar: "Shell window manager. Not the toolbar and not Glass.",
        intent: "No role.",
        policy: "No role. Closing a window is not a policy decision.",
        layer: BASE,
        verdict: DENY,
    },
    Component {
        name: "Focus mode",
        purpose: "Hides the top bar and the dock (F11, View, or `focus`; Esc exits).",
        when_allowed: "Never toggled by Glass.",
        must_not: "Must not enter or exit Focus, and must not treat a panel as the focused desktop.",
        shell_toolbar: "Shell mode that hides the toolbar and the dock.",
        intent: "No role.",
        policy: "No role.",
        layer: BASE,
        verdict: DENY,
    },
    Component {
        name: "Notifications",
        purpose: "Frosted notices stacked at the top-right. After Welcome the shell posts \"Welcome aboard\" / \"Benvenuto a bordo\".",
        when_allowed: "Never posted by Glass. The shell notifier owns that corner.",
        must_not: "Must not draw a notification stack or toast. A policy denial is an outcome panel, not a system notification.",
        shell_toolbar: "Shell chrome, separate from the toolbar.",
        intent: "No role.",
        policy: "No role. A notification is not an admission.",
        layer: BASE,
        verdict: DENY,
    },
    Component {
        name: "Spaces",
        purpose: "Virtual desktops. Dots on the top bar, Ctrl+Alt+arrows, and the Spaces app.",
        when_allowed: "Never managed by Glass.",
        must_not: "Must not create desktops, move windows between spaces, or present a panel as a space.",
        shell_toolbar: "Shell owns Spaces. The dots live on the toolbar.",
        intent: "No role.",
        policy: "No role.",
        layer: BASE,
        verdict: DENY,
    },
    Component {
        name: "Welcome",
        purpose: "First-boot card. English by default, Italiano selectable. Appearance is Light / Dawn / Deep. Title: \"Welcome to VelaOS\" / \"Benvenuto in VelaOS\".",
        when_allowed: "Never shown by Glass. First boot belongs to the §7 shell.",
        must_not: "Must not replace Welcome, choose the locale, or present onboarding as a chat.",
        shell_toolbar: "Shell first-boot surface, shown before the desktop is the workspace.",
        intent: "No role.",
        policy: "No role. Continuing Welcome grants nothing.",
        layer: BASE,
        verdict: DENY,
    },
    Component {
        name: "Theme choice",
        purpose: "The active theme is Light, Dawn, or Deep, chosen on Welcome or in Settings.",
        when_allowed: "Never switched by a Glass control. Glass may only follow the choice the shell already made.",
        must_not: "Must not ship a theme dock, a settings window, or a second palette editor.",
        shell_toolbar: "Welcome and Settings own the choice. The dock is not a theme switcher.",
        intent: "No role.",
        policy: "No role.",
        layer: BASE,
        verdict: DENY,
    },
    Component {
        name: "Frost material",
        purpose: "Translucent blurred surface. §7 uses it for the bar, dock, and notification cards. An open panel uses the same material so it reads as glass above the desktop.",
        when_allowed: "On a temporary overlay only, while that panel is open.",
        must_not: "Must not frost a fake toolbar, dock, window frame, or full-screen desktop.",
        shell_toolbar: "Borrows the shell material. Does not become the toolbar.",
        intent: "No role.",
        policy: "No role. Frost carries no authority.",
        layer: OVER,
        verdict: ALLOW,
    },
    Component {
        name: "Theme materials",
        purpose: "Light, Dawn, and Deep are the only palettes an overlay may follow. The names match the §7 theme records.",
        when_allowed: "When the shell has already selected one of the three, the open panel renders in that material.",
        must_not: "Must not invent a fourth theme, and must not treat the host page's preview colours as the ISO palette.",
        shell_toolbar: "The shell selects the theme. The panel does not own the control.",
        intent: "No role.",
        policy: "No role.",
        layer: OVER,
        verdict: ALLOW,
    },
    Component {
        name: "Objective surface",
        purpose: "One field for the current objective. Submit sends the utterance and nothing else.",
        when_allowed: "While the shell is idle, as a temporary field above the base. It closes when the panel is dismissed.",
        must_not: "Must not persist as a replacement toolbar, become a chat transcript, or attach candidates, handles, or grants to the submit body.",
        shell_toolbar: "Sits above the toolbar. The toolbar stays the shell's. ShellRuntime::submit is the only sink.",
        intent: "The utterance is input to Intent. Intent returns candidates and does not authorize.",
        policy: "Submit does not reach the PDP. No grant is issued.",
        layer: OVER,
        verdict: ALLOW,
    },
    Component {
        name: "Proposal panel",
        purpose: "The shell's glass phase: candidate index, title, version, score, sources, and auto_resolve (false). Handles may be shown in hex for audit.",
        when_allowed: "Only while the shell is in the glass phase, for candidates the shell is showing.",
        must_not: "Must not let the page post a handle or version back, style the list as a policy decision, or hide that the rows authorize nothing.",
        shell_toolbar: "Rendered above the shell from shell state. Not an application dock.",
        intent: "Rows are Intent candidates, data only.",
        policy: "Showing the panel issues no grant, frame, or selection.",
        layer: OVER,
        verdict: ALLOW,
    },
    Component {
        name: "Gesture row",
        purpose: "Avvia and Rivedi piano on a shown index, Annulla (undo, nothing authorized), and Chiudi (dismiss a result or error).",
        when_allowed: "Avvia and Rivedi piano only in the glass phase on a shown index. Annulla only from that phase. Chiudi only from result or error.",
        must_not: "Must not add actions, click a row the panel is not showing, or skip ShellRuntime::click.",
        shell_toolbar: "Gestures are shell clicks. The toolbar does not gain these buttons.",
        intent: "The index picks a shown Intent row. Intent does not choose.",
        policy: "The shell re-derives UserSelection and asks the PDP. The gesture is not itself a grant. Annulla authorizes nothing.",
        layer: OVER,
        verdict: ALLOW,
    },
    Component {
        name: "Rationale note",
        purpose: "Planner or model text shown as an unverified note with no effect.",
        when_allowed: "Only as a label on the open proposal, when the shell supplies a rationale.",
        must_not: "Must not treat the note as a decision, parse it as commands, or let it select a row or call the PDP.",
        shell_toolbar: "Not chrome. The shell forwards it only as planner_note.",
        intent: "Intent or model output. Untrusted.",
        policy: "Ignored as authority. An AI-only path is denied.",
        layer: OVER,
        verdict: ALLOW,
    },
    Component {
        name: "Outcome panel",
        purpose: "What the shell recorded after a gesture: executed, reviewed, denied with a stable code, nothing found, or an executor status.",
        when_allowed: "After the shell enters result or error, until Chiudi.",
        must_not: "Must not mint a grant, extend a TTL, retry a denial, or present generated text as the original object.",
        shell_toolbar: "Displays shell state above the base. Does not replace the toolbar.",
        intent: "No further proposal.",
        policy: "The text is the outcome the shell already produced. Glass does not admit.",
        layer: OVER,
        verdict: ALLOW,
    },
    Component {
        name: "Audit strip",
        purpose: "Read-only honesty: recall lane, M0 status, selections emitted, grants issued, frames signed, and the phase trace.",
        when_allowed: "Beside an open panel, as a display of shell counters.",
        must_not: "Must not be a control, and must not be a way to issue a grant or sign a frame.",
        shell_toolbar: "Reports shell counters. Not toolbar buttons.",
        intent: "May name which recall lanes ran. That is provenance, not a choice.",
        policy: "Displays counts the shell and PDP already incremented. Does not increment them.",
        layer: OVER,
        verdict: ALLOW,
    },
    Component {
        name: "Loopback bridge",
        purpose: "Host-demo transport. GET / serves the page with a per-run token. GET /state returns the view model. POST /submit and POST /click are the only mutations. Bind is loopback.",
        when_allowed: "Only as an untrusted host demonstration in front of ShellRuntime. The ISO desktop does not serve this page.",
        must_not: "Must not be called the shell, bind a non-loopback address, or add an admit, grant, select, or execute route.",
        shell_toolbar: "Talks to the shell. Is not the toolbar and is not a browser shell.",
        intent: "Carries the utterance in and the candidate view out.",
        policy: "Cannot admit. The PDP stays behind the shell click.",
        layer: HOST,
        verdict: ALLOW,
    },
];

/// Bans. Glass is a panel above the base, not a second desktop and not an authority.
pub static ANTI_PATTERNS: &[AntiPattern] = &[
    AntiPattern {
        id: "shell-equals-browser",
        name: "Shell is not a browser",
        rule: "Never shell=browser. The loopback page is an untrusted host demo. The §7 desktop has no browser and no HTTP server. Do not promote the page to a shell, and do not render the desktop inside a browser as the product.",
    },
    AntiPattern {
        id: "glass-as-authority",
        name: "Glass is not an authority",
        rule: "Never authority. Glass does not admit, grant, sign, select, or execute. There is no admit, grant, select, or execute route. Showing candidates authorizes nothing.",
    },
    AntiPattern {
        id: "replace-toolbar",
        name: "Do not replace the toolbar",
        rule: "Never replace the familiar toolbar. The §7 top bar and the dock belong to Vela Shell. Glass panels sit above that chrome. They do not remove it or substitute a header for it.",
    },
    AntiPattern {
        id: "chat-as-os",
        name: "Chat is not the OS",
        rule: "No chat-as-OS. No thread, no assistant that is the desktop, no conversation as the workspace. One objective, a proposal, a gesture, a result. Dismiss returns to the base.",
    },
    AntiPattern {
        id: "llm-as-authority",
        name: "The model is not an authority",
        rule: "No LLM-as-authority. Rankings and rationale are proposals. Prompt text that claims the user approved, or that demands auto-resolve, is unverified text with no effect. Policy denies an AI-only path.",
    },
    AntiPattern {
        id: "forge-authority",
        name: "Do not forge grants, selections, or authority",
        rule: "No forging grants, selections, or authority. The client cannot send a handle, version, selection, frame, grant, or ttl. The shell re-derives UserSelection from the shown row. Unknown JSON fields are rejected.",
    },
    AntiPattern {
        id: "second-desktop",
        name: "Glass is not a second desktop",
        rule: "Glass is temporary panels above the §7 base, not a second desktop. It does not draw window controls, the dock, Spaces, Focus, Welcome, or notifications.",
    },
];

/// Classification of what `poc/product/glass` already contains.
///
/// The host page still paints a header and a theme dock. Those are stand-ins
/// for chrome this catalog denies. They are not grown into a desktop here.
pub static INVENTORY: &[InventoryItem] = &[
    InventoryItem {
        path: "src/lib.rs",
        symbol: "GlassApp::view",
        verdict: ALLOW,
        component: "Proposal panel",
        note: "View model of shell state. Handles are hex for display only.",
    },
    InventoryItem {
        path: "src/lib.rs",
        symbol: "POST /submit",
        verdict: ALLOW,
        component: "Objective surface",
        note: "Utterance only. Does not authorize.",
    },
    InventoryItem {
        path: "src/lib.rs",
        symbol: "POST /click",
        verdict: ALLOW,
        component: "Gesture row",
        note: "avvia, rivedi, annulla, chiudi. Index of a shown row, or no index.",
    },
    InventoryItem {
        path: "src/lib.rs",
        symbol: "other routes",
        verdict: DENY,
        component: "Loopback bridge",
        note: "Anything other than /, /state, /submit, /click is 404, including admit, grant, select, and execute.",
    },
    InventoryItem {
        path: "src/lib.rs",
        symbol: "serve",
        verdict: ALLOW,
        component: "Loopback bridge",
        note: "Loopback HTTP for the host demo. Not a shell and not a browser.",
    },
    InventoryItem {
        path: "src/lib.rs",
        symbol: "Lanes",
        verdict: ALLOW,
        component: "Audit strip",
        note: "Recall-lane status for the strip. Not a choice and not a grant.",
    },
    InventoryItem {
        path: "src/lib.rs",
        symbol: "demo",
        verdict: DENY,
        component: "Loopback bridge",
        note: "Demo keys, corpus, and PCB seed. A host fixture, not a surface and not production authority.",
    },
    InventoryItem {
        path: "src/main.rs",
        symbol: "main",
        verdict: ALLOW,
        component: "Loopback bridge",
        note: "Binds 127.0.0.1 only.",
    },
    InventoryItem {
        path: "src/catalog.rs",
        symbol: "COMPONENTS",
        verdict: ALLOW,
        component: "",
        note: "These rules. Not a surface.",
    },
    InventoryItem {
        path: "assets/glass.html",
        symbol: "#q / #go",
        verdict: ALLOW,
        component: "Objective surface",
        note: "The field and Interpreta post /submit. Allowed as the objective gesture.",
    },
    InventoryItem {
        path: "assets/glass.html",
        symbol: "header.glass",
        verdict: DENY,
        component: "Top system bar",
        note: "Persistent branded header. Host-demo stand-in of a toolbar. Do not grow it into the §7 bar.",
    },
    InventoryItem {
        path: "assets/glass.html",
        symbol: "main#panel candidates",
        verdict: ALLOW,
        component: "Proposal panel",
        note: "Candidate rows while the shell is in the glass phase.",
    },
    InventoryItem {
        path: "assets/glass.html",
        symbol: "gesture buttons",
        verdict: ALLOW,
        component: "Gesture row",
        note: "Avvia, Rivedi piano, Annulla, Chiudi. They post an action and, when needed, an index.",
    },
    InventoryItem {
        path: "assets/glass.html",
        symbol: ".note",
        verdict: ALLOW,
        component: "Rationale note",
        note: "Unverified model text. textContent only. No effect.",
    },
    InventoryItem {
        path: "assets/glass.html",
        symbol: "result/error",
        verdict: ALLOW,
        component: "Outcome panel",
        note: "Renders the shell result or error. Does not mint the grant it displays.",
    },
    InventoryItem {
        path: "assets/glass.html",
        symbol: "footer#foot",
        verdict: ALLOW,
        component: "Audit strip",
        note: "Lanes, M0, selection count, grant count, frame count, trace. Display only.",
    },
    InventoryItem {
        path: "assets/glass.html",
        symbol: "nav.dock",
        verdict: DENY,
        component: "Dock",
        note: "A row of theme buttons. Not the six-glyph dock and not the teal underline. Do not extend it.",
    },
    InventoryItem {
        path: "assets/glass.html",
        symbol: "[data-theme]",
        verdict: DENY,
        component: "Theme choice",
        note: "Host preview that sets body class. The product switcher is Welcome and Settings, not Glass.",
    },
    InventoryItem {
        path: "assets/glass.html",
        symbol: "backdrop-filter",
        verdict: ALLOW,
        component: "Frost material",
        note: "Blur on the open panel. Frost for the overlay, not a redraw of bar or dock.",
    },
    InventoryItem {
        path: "assets/glass.html",
        symbol: "body theme classes",
        verdict: ALLOW,
        component: "Theme materials",
        note: "Named slots Light, Dawn, Deep. Preview colours are not the ISO palette.",
    },
    InventoryItem {
        path: "examples/m0_request.rs",
        symbol: "m0_request",
        verdict: DENY,
        component: "",
        note: "Dev fixture that prints an M0 request. Not a Glass surface.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = include_str!("../docs/ALLOWED_COMPONENTS.md");

    fn section<'a>(doc: &'a str, title: &str) -> &'a str {
        let marker = format!("\n### {title}\n");
        let start = doc
            .find(&marker)
            .unwrap_or_else(|| panic!("missing section {title}"));
        let body = &doc[start + marker.len()..];
        match body.find("\n### ") {
            Some(end) => &body[..end],
            None => body,
        }
    }

    #[test]
    fn components_are_complete_and_base_chrome_is_denied() {
        let mut names = Vec::new();
        for c in COMPONENTS {
            assert!(!names.contains(&c.name), "duplicate {}", c.name);
            names.push(c.name);
            assert!(!c.purpose.is_empty() && !c.when_allowed.is_empty() && !c.must_not.is_empty());
            assert!(!c.shell_toolbar.is_empty() && !c.intent.is_empty() && !c.policy.is_empty());
            if c.layer == Layer::BaseChrome {
                assert_eq!(c.verdict, Verdict::Deny, "{}", c.name);
            }
            if c.verdict == Verdict::Allow {
                assert_ne!(c.layer, Layer::BaseChrome, "{}", c.name);
            }
        }
        let base: Vec<_> = COMPONENTS
            .iter()
            .filter(|c| c.layer == Layer::BaseChrome)
            .map(|c| c.name)
            .collect();
        assert_eq!(
            base,
            vec![
                "Top system bar",
                "Dock",
                "Window controls",
                "Focus mode",
                "Notifications",
                "Spaces",
                "Welcome",
                "Theme choice",
            ]
        );
    }

    #[test]
    fn anti_patterns_cover_the_bans() {
        let ids: Vec<_> = ANTI_PATTERNS.iter().map(|a| a.id).collect();
        assert_eq!(
            ids,
            vec![
                "shell-equals-browser",
                "glass-as-authority",
                "replace-toolbar",
                "chat-as-os",
                "llm-as-authority",
                "forge-authority",
                "second-desktop",
            ]
        );
        for a in ANTI_PATTERNS {
            assert!(!a.name.is_empty() && !a.rule.is_empty());
        }
    }

    #[test]
    fn wire_allowlists_match_the_shell_gestures() {
        assert_eq!(SUBMIT_FIELDS, ["token", "utterance"].as_slice());
        assert_eq!(CLICK_FIELDS, ["token", "action", "candidate"].as_slice());
        assert_eq!(
            ALLOWED_CLICK_ACTIONS,
            ["avvia", "rivedi", "annulla", "chiudi"].as_slice()
        );
        for banned in ["approve_all", "grant", "select", "admit"] {
            assert!(!ALLOWED_CLICK_ACTIONS.contains(&banned));
        }
        for path in [
            "/admit",
            "/grant",
            "/select",
            "/selection",
            "/execute",
            "/api/admit",
            "/click/0",
        ] {
            assert!(FORBIDDEN_ROUTES.contains(&path), "{path}");
        }
        for field in [
            "handle",
            "expected_version",
            "selection",
            "version",
            "frame",
            "grant",
            "ttl",
        ] {
            assert!(FORBIDDEN_FIELDS.contains(&field), "{field}");
        }
    }

    #[test]
    fn doc_restates_every_row() {
        assert!(DOC.contains("60a5b8b9ec1e566f20c450366e8fa16c10ea34dd"));
        assert!(DOC.contains("not a second desktop"));
        for c in COMPONENTS {
            let body = section(DOC, c.name);
            for label in [
                "**Layer:**",
                "**Verdict:**",
                "**Purpose:**",
                "**When allowed:**",
                "**Must not:**",
                "**Shell / toolbar:**",
                "**Intent:**",
                "**Policy:**",
            ] {
                assert!(body.contains(label), "{} missing {label}", c.name);
            }
            match c.layer {
                Layer::BaseChrome => assert!(body.contains("§7 base chrome"), "{}", c.name),
                Layer::EphemeralOverlay => {
                    assert!(body.contains("ephemeral overlay"), "{}", c.name)
                }
                Layer::HostDemoTransport => {
                    assert!(body.contains("host-demo transport"), "{}", c.name)
                }
            }
            match c.verdict {
                Verdict::Allow => assert!(body.contains("allow"), "{}", c.name),
                Verdict::Deny => assert!(body.contains("deny"), "{}", c.name),
            }
        }
        for a in ANTI_PATTERNS {
            let body = section(DOC, a.id);
            assert!(body.contains("**Rule:**"), "{}", a.id);
            assert!(body.contains(a.name), "{}", a.id);
        }
        for item in INVENTORY {
            assert!(DOC.contains(item.path), "{}", item.path);
            assert!(DOC.contains(item.symbol), "{}", item.symbol);
            if !item.component.is_empty() {
                assert!(
                    COMPONENTS.iter().any(|c| c.name == item.component),
                    "{}",
                    item.component
                );
            }
        }
    }

    #[test]
    fn inventory_does_not_allow_base_chrome() {
        let base: Vec<_> = COMPONENTS
            .iter()
            .filter(|c| c.layer == Layer::BaseChrome)
            .map(|c| c.name)
            .collect();
        for item in INVENTORY {
            if item.verdict == Verdict::Allow {
                assert!(
                    !base.contains(&item.component),
                    "{} allows {}",
                    item.symbol,
                    item.component
                );
            }
        }
        let paths = [
            "src/lib.rs",
            "src/main.rs",
            "src/catalog.rs",
            "assets/glass.html",
        ];
        for path in paths {
            assert!(INVENTORY.iter().any(|i| i.path == path), "{path}");
        }
    }

    #[test]
    fn host_page_standins_stay_denied_and_base_chrome_is_absent() {
        let page = crate::PAGE;
        if page.contains("<nav class=\"dock glass\">") {
            assert!(
                INVENTORY
                    .iter()
                    .any(|i| i.symbol == "nav.dock" && i.verdict == Verdict::Deny)
            );
        }
        if page.contains("<header class=\"glass\">") {
            assert!(
                INVENTORY
                    .iter()
                    .any(|i| i.symbol == "header.glass" && i.verdict == Verdict::Deny)
            );
        }
        for banned in [
            "Welcome to VelaOS",
            "Benvenuto in VelaOS",
            "Make yourself at home",
        ] {
            assert!(
                !page.contains(banned),
                "{banned} is §7 chrome, not a Glass panel"
            );
        }
    }
}
