//! `cargo run --example demo -- "riassumi le note progetto VelaOS"`
//! Walks the Zero-App Shell on the host: idle -> glass -> Avvia -> result.

use biscuit_auth::{KeyPair as RootKeys, builder::BiscuitBuilder};
use ed25519_compact::{KeyPair as EdKeys, PublicKey, Seed};
use vela_executor_host::HostExecutor;
use vela_executor_policy::pipeline::SelectionGatedPdp;
use vela_intent::{Corpus, Doc, IntentEngine};
use vela_shell::{Click, ShellRuntime};

const USER: &str = "vela-user";

fn delegation(root: &RootKeys, resource: &str, op: &str) -> Vec<u8> {
    BiscuitBuilder::new()
        .code(format!("right(\"{USER}\", \"{resource}\", \"{op}\");"))
        .unwrap()
        .build(root)
        .unwrap()
        .to_vec()
        .unwrap()
}

fn main() {
    let utterance = std::env::args().nth(1).unwrap_or_else(|| "riassumi le note progetto VelaOS".into());
    let corpus = Corpus::new(vec![
        Doc { handle: [0xA1; 32], version: 3, oid: "oid-notes".into(), title: "Note progetto VelaOS (bozza)".into(), body: "architettura microkernel note riunione".into() },
        Doc { handle: [0xB2; 32], version: 1, oid: "oid-photos".into(), title: "Foto vacanze note".into(), body: "mare".into() },
        Doc { handle: [0xC3; 32], version: 9, oid: "oid-keys".into(), title: "Chiavi private".into(), body: "seed".into() },
    ]);
    let root = RootKeys::new();
    let pcb = EdKeys::from_seed(Seed::new([41; 32])); // demo key, NOT a production key
    let pdp = SelectionGatedPdp::new(root.public(), delegation(&root, "lfs", "read"), delegation(&root, "executor", "execute"), pcb.sk).unwrap();
    let mut shell = ShellRuntime::new(USER, IntentEngine::new(corpus), pdp, HostExecutor::new(PublicKey::new(*pcb.pk)));
    println!("{}\n", shell.render());
    shell.submit(&utterance).unwrap();
    println!("{}\n", shell.render());
    if shell.click(Click::Avvia { candidate: 0 }).is_ok() {
        println!("{}\n", shell.render());
    }
    println!("trace: {:?}", shell.trace());
    println!("UserSelection emesse: {}", shell.emitted_selections().len());
    println!("grant: {}  frame firmati: {}", shell.grants_issued(), shell.frames_signed());
}
