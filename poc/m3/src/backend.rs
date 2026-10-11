//! Private loopback development adapter. Production inference must expose the Neural Bus manifest.
use anyhow::{Result, anyhow, ensure};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};
use zeroize::Zeroizing;

pub struct LocalCompiler {
    address: SocketAddr,
    key: Zeroizing<String>,
}
impl LocalCompiler {
    pub fn new(address: SocketAddr, key: String) -> Result<Self> {
        ensure!(
            address.ip().is_loopback(),
            "inference endpoint must be local"
        );
        ensure!(
            !key.is_empty() && key.len() <= 128 && key.bytes().all(|b| b.is_ascii_alphanumeric()),
            "invalid endpoint credential"
        );
        Ok(Self {
            address,
            key: Zeroizing::new(key),
        })
    }
    pub fn compile(&self, request: &str, focus: bool) -> Result<serde_json::Value> {
        ensure!(request.len() <= 8192, "request exceeds bound");
        let input =
            serde_json::to_string(&serde_json::json!({"request":request,"focus_netconfig":focus}))?;
        let context = if focus {
            "Trusted host context: a current network configuration is selected."
        } else {
            "Trusted host context: NO current network configuration is selected. For ANY request to save the current network, output clarify. Searching or restoring an existing saved network may still be classified."
        };
        let body = serde_json::to_vec(
            &serde_json::json!({"messages":[{"role":"system","content":crate::PROMPT},
            {"role":"system","content":context},
            {"role":"user","content":input}],"temperature":0,"seed":7,"max_tokens":32,
            "stream":false,"cache_prompt":true,"chat_template_kwargs":{"enable_thinking":false},
            "grammar": "root ::= \"{\\\"intent\\\":\\\"\" (\"save\" | \"recall\" | \"restore\" | \"clarify\" | \"unsupported\") \"\\\"}\""}),
        )?;
        let mut stream = TcpStream::connect_timeout(&self.address, Duration::from_secs(2))?;
        stream.set_read_timeout(Some(Duration::from_secs(90)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        write!(
            stream,
            "POST /v1/chat/completions HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
            self.address,
            &*self.key,
            body.len()
        )?;
        stream.write_all(&body)?;
        let mut reply = Vec::new();
        stream.take(128 * 1024 + 1).read_to_end(&mut reply)?;
        ensure!(reply.len() <= 128 * 1024, "inference reply exceeds bound");
        let split = reply
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or_else(|| anyhow!("missing HTTP headers"))?;
        ensure!(split <= 16 * 1024, "HTTP headers exceed bound");
        let header = std::str::from_utf8(&reply[..split])?;
        ensure!(
            header.starts_with("HTTP/1.1 200 ") || header.starts_with("HTTP/1.0 200 "),
            "local compiler request failed"
        );
        ensure!(
            !header.to_ascii_lowercase().contains("transfer-encoding:"),
            "unsupported HTTP framing"
        );
        Ok(serde_json::from_slice(&reply[split + 4..])?)
    }
}
