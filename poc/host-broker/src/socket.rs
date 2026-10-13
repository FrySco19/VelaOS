//! Development host adapter. It never treats TCP delivery as an LFS receipt.
use crate::hello;
use ed25519_compact::{KeyPair, Seed};
use ed25519_dalek::VerifyingKey;
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tungstenite::{
    Message,
    handshake::server::{ErrorResponse, Request, Response},
};
use vela_note_action::{CAP_SIZE, Capability, MAX_NOTE, Operation, RECEIPT_SIZE, Receipt, digest};
use vela_protocol::{Frame, FrameRef, HEADER_SIZE, Header, MAX_FRAME, MessageType};
use vela_selection_contract::UserSelection;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|v| format!("{v:02x}")).collect()
}
fn key(path: &Path) -> Result<[u8; 32], String> {
    fs::read(path)
        .map_err(|e| e.to_string())?
        .try_into()
        .map_err(|_| "Expected 32-byte key".into())
}
fn send(stream: &mut TcpStream, kind: MessageType, id: u64, payload: &[u8]) -> Result<(), String> {
    let frame = Frame::new(kind, id, 0, payload).map_err(|e| format!("{e:?}"))?;
    let mut bytes = [0; MAX_FRAME];
    let wire = frame.encode(&mut bytes).map_err(|e| format!("{e:?}"))?;
    stream.write_all(wire).map_err(|e| e.to_string())
}
fn receive(stream: &mut TcpStream) -> Result<([u8; MAX_FRAME], usize), String> {
    let mut bytes = [0; MAX_FRAME];
    stream
        .read_exact(&mut bytes[..HEADER_SIZE])
        .map_err(|e| e.to_string())?;
    let header = Header::decode(&bytes[..HEADER_SIZE]).map_err(|e| format!("{e:?}"))?;
    let length = HEADER_SIZE + header.payload_len as usize;
    if length > MAX_FRAME {
        return Err("Oversize guest frame".into());
    }
    stream
        .read_exact(&mut bytes[HEADER_SIZE..length])
        .map_err(|e| e.to_string())?;
    Ok((bytes, length))
}
pub fn serve(
    seed: &Path,
    pcb_public: &Path,
    lfs_public: &Path,
    allow_console: bool,
    mock_unavailable: bool,
) -> Result<(), String> {
    let pair = KeyPair::from_seed(Seed::new(key(seed)?));
    let pcb = VerifyingKey::from_bytes(&key(pcb_public)?).map_err(|e| e.to_string())?;
    let lfs = VerifyingKey::from_bytes(&key(lfs_public)?).map_err(|e| e.to_string())?;
    let listener = TcpListener::bind("127.0.0.1:7990").map_err(|e| e.to_string())?;
    println!(
        "BROKER: ws://127.0.0.1:7990 ; guest=127.0.0.1:7991 ; console-development={allow_console}"
    );
    let mut last_id = 0u64;
    for connection in listener.incoming() {
        let stream = connection.map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| e.to_string())?;
        let callback = |request: &Request, response: Response| -> Result<Response, ErrorResponse> {
            let origin = request
                .headers()
                .get("origin")
                .and_then(|v| v.to_str().ok());
            let host = request.headers().get("host").and_then(|v| v.to_str().ok());
            if !matches!(
                origin,
                Some("http://127.0.0.1:7882" | "http://localhost:7882")
            ) || host != Some("127.0.0.1:7990")
                || request.uri().path() != "/"
            {
                return Err(tungstenite::http::Response::builder()
                    .status(403)
                    .body(Some("Origin/Host rejected".into()))
                    .unwrap());
            }
            Ok(response)
        };
        let config = tungstenite::protocol::WebSocketConfig::default()
            .max_message_size(Some(8192))
            .max_frame_size(Some(8192));
        let mut ws = match tungstenite::accept_hdr_with_config(stream, callback, Some(config)) {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("HANDSHAKE denied: {e}");
                continue;
            }
        };
        let result = (|| -> Result<Value, String> {
            let message = ws.read().map_err(|e| e.to_string())?;
            let Message::Text(text) = message else {
                return Err("Expected JSON Proposal".into());
            };
            let request: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            if request["type"] != "Proposal" || request["operation"] != "write-note" {
                return Err("Unsupported operation".into());
            }
            let note = request["note"].as_str().ok_or("Missing note")?;
            if note.len() > MAX_NOTE {
                return Err("Note exceeds 1024 UTF-8 bytes".into());
            }
            let intent = request["intent"].as_str().ok_or("Missing intent")?;
            if intent.len() > 512 {
                return Err("Intent too long".into());
            }
            if request["note_sha256"].as_str() != Some(&hex(&digest(note.as_bytes()))) {
                return Err("Proposal hash mismatch".into());
            }
            let id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_millis()
                .try_into()
                .map_err(|_| "Clock overflow")?;
            let id = u64::max(id, last_id.checked_add(1).ok_or("Sequence exhausted")?);
            last_id = id;
            // Durable replay counter is enforced by LFS. Epoch 1 is the current
            // deployment generation, not a fabricated fresh guest boot nonce.
            let op = Operation::new(1, id, note.as_bytes()).map_err(|e| format!("{e:?}"))?;
            let mut session = [0; 32];
            getrandom::getrandom(&mut session).map_err(|e| e.to_string())?;
            let prompt = format!(
                "Write /lfs/notes/note1.txt\nIntent: {intent}\nSHA-256: {}\nExact note:\n{note}",
                hex(&digest(note.as_bytes()))
            );
            let consent = if mock_unavailable {
                hello::verify_mock_unavailable(&prompt)?
            } else {
                hello::verify(&prompt, allow_console)?
            };
            // Signature happens only after verified OS consent or explicitly
            // enabled, labelled, interactive DEVELOPMENT approval.
            let mut approval = UserSelection {
                session_binding: session,
                revision: id,
                plan_sha256: digest(&op.manifest()),
                signature: [0; 64],
            };
            let signature = pair.sk.sign(approval.signing_bytes(), None);
            approval.signature.copy_from_slice(signature.as_ref());
            let address: SocketAddr = "127.0.0.1:7991".parse().unwrap();
            let mut guest = TcpStream::connect_timeout(&address, Duration::from_secs(5))
                .map_err(|e| e.to_string())?;
            guest
                .set_read_timeout(Some(Duration::from_secs(15)))
                .map_err(|e| e.to_string())?;
            guest
                .set_write_timeout(Some(Duration::from_secs(5)))
                .map_err(|e| e.to_string())?;
            let mut proposal = [0; 50 + MAX_NOTE];
            proposal[..8].copy_from_slice(&op.epoch.to_le_bytes());
            proposal[8..16].copy_from_slice(&id.to_le_bytes());
            proposal[16..48].copy_from_slice(&session);
            proposal[48..50].copy_from_slice(&(note.len() as u16).to_le_bytes());
            proposal[50..50 + note.len()].copy_from_slice(note.as_bytes());
            send(
                &mut guest,
                MessageType::Proposal,
                id,
                &proposal[..50 + note.len()],
            )?;
            send(&mut guest, MessageType::Approval, id, &approval.encode())?;
            let (bytes, length) = receive(&mut guest)?;
            let frame = FrameRef::decode(&bytes[..length]).map_err(|e| format!("{e:?}"))?;
            if frame.header.kind != MessageType::Receipt
                || frame.header.transaction != id
                || frame.header.sequence != 0
                || frame.payload.len() != CAP_SIZE + RECEIPT_SIZE
            {
                return Err("Missing/mismatched durable receipt".into());
            }
            let cap = Capability(frame.payload[..CAP_SIZE].try_into().unwrap());
            // Signature and operation binding, independent of host wall clock.
            cap.verify_binding(&op, &pcb)
                .map_err(|e| format!("Capability: {e:?}"))?;
            let receipt = Receipt(frame.payload[CAP_SIZE..].try_into().unwrap());
            receipt
                .verify(&op, &cap, &lfs)
                .map_err(|e| format!("Receipt: {e:?}"))?;
            println!("BROKER: VERIFIED LFS RECEIPT request={id} consent={consent:?}");
            Ok(
                json!({"type":"Receipt","request":id.to_string(),"note_sha256":hex(&digest(note.as_bytes())),"receipt_hex":hex(&receipt.0),"verified":true,"consent":format!("{consent:?}")}),
            )
        })();
        let response = match result {
            Ok(receipt) => receipt,
            Err(reason) => {
                eprintln!("BROKER denied: {reason}");
                json!({"type":"Denied","reason":reason})
            }
        };
        let _ = ws.send(Message::Text(response.to_string().into()));
        let _ = ws.close(None);
    }
    Ok(())
}
