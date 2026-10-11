//! Regenerates `poc/m5_executor/fixtures/*.wasm` from `*.wat` and the padded
//! size fixtures. Run:
//! `cargo run --example gen_fixtures --manifest-path poc/m5_executor/executor-host/Cargo.toml`
#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub const PAGE_SIZE: usize = 4096;
pub const PAYLOAD_OFFSET: usize = 20 + 32 + 64;
pub const MAX_PAYLOAD_BYTES: usize = PAGE_SIZE - PAYLOAD_OFFSET;

/// Same bytes as `bridge::RETURN_7_MODULE`.
pub const RETURN_7: &[u8] = &[
    0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 127, 3, 2, 1, 0, 7, 7, 1, 3, b'r', b'u', b'n',
    0, 0, 10, 6, 1, 4, 0, 65, 7, 11,
];

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures")
}

fn leb(mut v: usize, out: &mut Vec<u8>) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

/// return-7 module + custom section "pad", total length exactly `total`.
pub fn padded_return7(total: usize) -> Vec<u8> {
    for pad in 0..total {
        let mut content = vec![3, b'p', b'a', b'd'];
        content.resize(4 + pad, 0);
        let mut module = RETURN_7.to_vec();
        module.push(0);
        leb(content.len(), &mut module);
        module.extend_from_slice(&content);
        if module.len() == total {
            return module;
        }
        if module.len() > total {
            break;
        }
    }
    panic!("cannot pad return-7 module to {total} bytes");
}

/// All generated fixtures as (file name, bytes).
pub fn generate() -> Vec<(String, Vec<u8>)> {
    let dir = fixtures_dir();
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .expect("fixtures dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "wat"))
        .collect();
    names.sort();
    let mut out = Vec::new();
    for path in names {
        let source = std::fs::read_to_string(&path).expect("read wat");
        let bytes = wat::parse_str(&source).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = path.with_extension("wasm").file_name().unwrap().to_string_lossy().into_owned();
        out.push((name, bytes));
    }
    out.push(("max_payload.wasm".into(), padded_return7(MAX_PAYLOAD_BYTES)));
    out.push(("oversized.wasm".into(), padded_return7(MAX_PAYLOAD_BYTES + 1)));
    out
}

fn main() {
    let dir = fixtures_dir();
    for (name, bytes) in generate() {
        std::fs::write(dir.join(&name), &bytes).expect("write fixture");
        println!("{name}: {} bytes", bytes.len());
    }
}
