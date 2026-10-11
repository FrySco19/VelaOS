//! `cargo run --offline --release -- [--port 7878] [--m0 <dir con bridge_recall.py, .venv, .model-cache>]`
//! then open http://127.0.0.1:7878/ . Loopback only.

use std::net::TcpListener;
use std::path::PathBuf;
use vela_glass::{GlassApp, demo, serve};
use vela_m0_recall::M0Config;

fn main() {
    let mut port: u16 = 7878;
    let mut m0: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" => port = args.next().and_then(|p| p.parse().ok()).expect("--port <n>"),
            "--m0" => m0 = args.next().map(PathBuf::from),
            other => panic!("argomento sconosciuto: {other}"),
        }
    }
    let cfg = m0.map(M0Config::in_dir).or_else(M0Config::from_env);
    let (engine, lanes) = demo::engine(cfg);
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("bind 127.0.0.1");
    let port = listener.local_addr().expect("addr").port();
    let mut app = GlassApp::new(demo::shell(engine), lanes, port);
    println!("VelaOS Glass (host demo) su http://127.0.0.1:{port}/");
    println!("lanes: {}", app.view()["lanes"]["label"]);
    println!("m0: {}", app.view()["lanes"]["m0"]);
    serve(&listener, &mut app, None);
}
