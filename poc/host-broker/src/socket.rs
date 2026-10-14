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

fn unhex16(value: &str) -> Result<[u8; 16], String> {
    if value.len() != 32 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid object ID".into());
    }
    let mut out = [0; 16];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(|_| "Invalid ID")?;
    }
    Ok(out)
}
fn guest_address() -> Result<SocketAddr, String> {
    let address: SocketAddr = std::env::var("VELA_GUEST_ENDPOINT")
        .unwrap_or_else(|_| "127.0.0.1:7991".into())
        .parse()
        .map_err(|_| "Invalid guest endpoint")?;
    if !address.ip().is_loopback() {
        return Err("Guest endpoint must be loopback".into());
    }
    Ok(address)
}
fn query(
    pair: &KeyPair,
    lfs: &VerifyingKey,
    read: bool,
    target: [u8; 16],
) -> Result<(u64, Option<Value>), String> {
    let mut nonce = [0; 32];
    getrandom::getrandom(&mut nonce).map_err(|e| e.to_string())?;
    let id = u64::from_le_bytes(nonce[..8].try_into().unwrap()).max(1);
    let (signed, n) = vela_note_action::catalog::request_bytes(read, id, &nonce, &target);
    let mut payload = [0; vela_note_action::catalog::REQUEST_SIZE];
    payload[..32].copy_from_slice(&nonce);
    payload[32..48].copy_from_slice(&target);
    payload[48..].copy_from_slice(pair.sk.sign(&signed[..n], None).as_ref());
    let address = guest_address()?;
    let mut stream =
        TcpStream::connect_timeout(&address, Duration::from_secs(5)).map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .map_err(|e| e.to_string())?;
    send(
        &mut stream,
        if read {
            MessageType::ReadRequest
        } else {
            MessageType::ListRequest
        },
        id,
        &payload,
    )?;
    let (wire, n) = receive(&mut stream)?;
    let f = FrameRef::decode(&wire[..n]).map_err(|e| format!("{e:?}"))?;
    if f.header.transaction != id
        || f.header.sequence != 0
        || f.header.kind
            != if read {
                MessageType::ReadResponse
            } else {
                MessageType::ListResponse
            }
    {
        return Err("Mismatched catalog response".into());
    }
    let (next, entry) =
        vela_note_action::catalog::verify_response(read, id, &nonce, f.payload, lfs)
            .map_err(|e| format!("Catalog proof: {e:?}"))?;
    let value=entry.map(|e|{let r=e.receipt.0;let request=u64::from_le_bytes(r[10..18].try_into().unwrap()).to_string();
        json!({"id":hex(&r[90..106]),"document_id":hex(&e.root),"parent_id":hex(&e.parent),"request":request,"timestamp_ms":request,"preview":e.note.chars().take(120).collect::<String>(),"note":if read {Some(e.note)}else{None},"note_sha256":hex(&r[26..58]),"receipt_hex":hex(&r),"verified":true,"consent":"Unrecorded","operation":"write-note"})});
    if read
        && value
            .as_ref()
            .is_some_and(|v| v["id"].as_str() != Some(hex(&target).as_str()))
    {
        return Err("Wrong read object".into());
    }
    Ok((next, value))
}
fn list_current(pair: &KeyPair, lfs: &VerifyingKey) -> Result<Value, String> {
    let mut notes: Vec<Value> = Vec::new();
    let mut cursor = 0u64;
    for _ in 0..=64 {
        let mut target = [0; 16];
        target[..8].copy_from_slice(&cursor.to_le_bytes());
        let (next, entry) = query(pair, lfs, false, target)?;
        if let Some(e) = entry {
            if !notes.iter().any(|old| old["id"] == e["id"]) {
                notes.push(e);
            }
        }
        if next == 64 {
            return Ok(json!({"type":"ListResponse","verified":true,"notes":notes}));
        }
        if next <= cursor {
            return Err("Non-progressing index cursor".into());
        }
        cursor = next;
    }
    Err("Catalog exceeds bounded extent".into())
}
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
    credential: &Path,
    dev_virtual: bool,
) -> Result<(), String> {
    if dev_virtual {
        hello::dev_warning();
    }
    let pcb = VerifyingKey::from_bytes(&key(pcb_public)?).map_err(|e| e.to_string())?;
    let lfs = VerifyingKey::from_bytes(&key(lfs_public)?).map_err(|e| e.to_string())?;
    let listener = TcpListener::bind("127.0.0.1:7990").map_err(|e| e.to_string())?;
    println!(
        "BROKER: ws://127.0.0.1:7990 ; guest=127.0.0.1:7991 ; consent={}",
        if dev_virtual {
            "DEV VIRTUAL FIDO (interactive Enter)"
        } else {
            "FIDO2 USB HID required"
        }
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
            if matches!(message, Message::Close(_)) {
                return Err("SYNC_CLIENT_CLOSED".into());
            }
            let Message::Text(text) = message else {
                return Err("Expected JSON Proposal".into());
            };
            let request: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            if request["type"] == "IntentRequest" {
                return crate::intent::compile(request["input"].as_str().ok_or("Missing input")?);
            }
            if request["type"] == "ListRequest" {
                let plan = digest(b"Vela.List.AllImmutableNotes.v1");
                let presence = hello::authorize(credential, plan, dev_virtual)?;
                if !presence.permits(&plan) {
                    return Err("FIDO2 plan mismatch".into());
                }
                let pair = KeyPair::from_seed(Seed::new(key(seed)?));
                let snapshot = list_current(&pair, &lfs)?;
                if let Some(notes) = snapshot["notes"].as_array() {
                    for note in notes {
                        last_id = last_id.max(
                            note["request"]
                                .as_str()
                                .ok_or("Missing receipt request")?
                                .parse::<u64>()
                                .map_err(|_| "Invalid receipt request")?,
                        );
                    }
                }
                return Ok(snapshot);
            }
            if request["type"] == "ReadRequest" {
                let target = unhex16(request["id"].as_str().ok_or("Missing read ID")?)?;
                let mut plan_bytes = b"Vela.Read.ImmutableVersion.v1".to_vec();
                plan_bytes.extend_from_slice(&target);
                let plan = digest(&plan_bytes);
                let presence = hello::authorize(credential, plan, dev_virtual)?;
                if !presence.permits(&plan) {
                    return Err("FIDO2 plan mismatch".into());
                }
                let pair = KeyPair::from_seed(Seed::new(key(seed)?));
                let (_, entry) = query(&pair, &lfs, true, target)?;
                return Ok(
                    json!({"type":"ReadResponse","verified":true,"entry":entry.ok_or("Document not found")?}),
                );
            }
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
            let root = match request["document_id"].as_str() {
                Some(v) => unhex16(v)?,
                None => [0; 16],
            };
            let parent = match request["parent_id"].as_str() {
                Some(v) => unhex16(v)?,
                None => [0; 16],
            };
            let op = Operation::document(1, id, root, parent, note.as_bytes())
                .map_err(|e| format!("{e:?}"))?;
            let mut session = [0; 32];
            getrandom::getrandom(&mut session).map_err(|e| e.to_string())?;
            let mut approval = UserSelection {
                session_binding: session,
                revision: id,
                plan_sha256: digest(&op.manifest()),
                signature: [0; 64],
            };
            // FIDO covers session, revision, document, parent and exact bytes.
            // No Ed25519 signing key is loaded before successful verification.
            let plan = digest(&approval.signing_bytes());
            let presence = hello::authorize(credential, plan, dev_virtual)?;
            if !presence.permits(&plan) {
                return Err("FIDO2 plan mismatch".into());
            }
            let pair = KeyPair::from_seed(Seed::new(key(seed)?));
            let consent = presence.consent();
            let signature = pair.sk.sign(approval.signing_bytes(), None);
            approval.signature.copy_from_slice(signature.as_ref());
            let address = guest_address()?;
            let mut guest = TcpStream::connect_timeout(&address, Duration::from_secs(5))
                .map_err(|e| e.to_string())?;
            guest
                .set_read_timeout(Some(Duration::from_secs(15)))
                .map_err(|e| e.to_string())?;
            guest
                .set_write_timeout(Some(Duration::from_secs(5)))
                .map_err(|e| e.to_string())?;
            let mut proposal = [0; 82 + MAX_NOTE];
            proposal[..8].copy_from_slice(&op.epoch.to_le_bytes());
            proposal[8..16].copy_from_slice(&id.to_le_bytes());
            proposal[16..48].copy_from_slice(&session);
            proposal[48..50].copy_from_slice(&(note.len() as u16).to_le_bytes());
            proposal[50..66].copy_from_slice(&op.root);
            proposal[66..82].copy_from_slice(&op.parent);
            proposal[82..82 + note.len()].copy_from_slice(note.as_bytes());
            send(
                &mut guest,
                MessageType::Proposal,
                id,
                &proposal[..82 + note.len()],
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
                json!({"type":"Receipt","id":hex(&op.object()),"document_id":hex(&op.root),"parent_id":hex(&op.parent),"preview":note.chars().take(120).collect::<String>(),"request":id.to_string(),"note_sha256":hex(&digest(note.as_bytes())),"receipt_hex":hex(&receipt.0),"verified":true,"consent":consent}),
            )
        })();
        if result
            .as_ref()
            .is_err_and(|reason| reason == "SYNC_CLIENT_CLOSED")
        {
            continue;
        }
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
