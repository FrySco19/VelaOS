use ed25519_compact::{KeyPair, Seed};
use rand::{RngCore, rngs::OsRng};
use std::{fs, path::Path};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "keygen PRIVATE_SEED PUBLIC_RUST_FILE");
    let path = Path::new(&args[1]);
    let seed: [u8; 32] = if path.exists() {
        fs::read(path)
            .expect("read protected seed")
            .try_into()
            .expect("32-byte seed")
    } else {
        let mut seed = [0; 32];
        OsRng.fill_bytes(&mut seed);
        fs::write(path, seed).expect("write protected seed");
        seed
    };
    let keys = KeyPair::from_seed(Seed::new(seed));
    fs::write(&args[2], format!("// Public trust anchor; private seed is a local build secret.\npub const TRUSTED_PUBLIC_KEY: [u8; 32] = {:?};\n", keys.pk.as_ref())).expect("write public anchor");
    println!("Generated/loaded PCB signing key; only the public anchor enters source control.");
}

#[cfg(test)]
#[path = "../../app/src/bridge.rs"]
mod bridge;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_boundaries_and_overlap_are_rejected() {
        let key = KeyPair::from_seed(Seed::new([31; 32]));
        let frame = bridge::signed_frame(&key.sk, 9, bridge::RETURN_7_MODULE, 1000);
        assert_eq!(bridge::parse_descriptor(&frame).unwrap().request_id, 9);
        for (offset, value) in [
            (4, 0),
            (6, 19),
            (12, 0),
            (12, 4095),
            (14, 0),
            (14, 95),
            (16, 0),
            (16, 20),
            (16, 4095),
            (18, 0),
            (18, 65535),
        ] {
            let mut malformed = frame;
            malformed[offset..offset + 2].copy_from_slice(&(value as u16).to_le_bytes());
            assert_eq!(
                bridge::parse_descriptor(&malformed).unwrap_err(),
                bridge::Status::Malformed
            );
        }
        // The last byte of the page is valid; moving it one byte beyond is not.
        let mut boundary = frame;
        boundary[16..18].copy_from_slice(&4095u16.to_le_bytes());
        boundary[18..20].copy_from_slice(&1u16.to_le_bytes());
        assert!(bridge::parse_descriptor(&boundary).is_ok());
        boundary[18..20].copy_from_slice(&2u16.to_le_bytes());
        assert!(bridge::parse_descriptor(&boundary).is_err());
    }
    #[test]
    fn signed_snapshot_rejects_tampering_scope_replay_and_bounds() {
        let keys = KeyPair::from_seed(Seed::new([27; 32]));
        let valid = bridge::signed_frame(&keys.sk, 1, bridge::RETURN_7_MODULE, 1000);
        let verified = bridge::verify_with_key(&valid, 0, keys.pk).unwrap();
        assert_eq!(verified.fuel, 1000);
        assert_eq!(
            verified.descriptor.payload_len,
            bridge::RETURN_7_MODULE.len()
        );
        let mut tampered = valid;
        tampered[bridge::PAYLOAD_OFFSET + 10] ^= 1;
        assert_eq!(
            bridge::verify_with_key(&tampered, 0, keys.pk).unwrap_err(),
            bridge::Status::Authentication
        );
        let mut header = valid;
        header[8] = 2;
        assert_eq!(
            bridge::verify_with_key(&header, 0, keys.pk).unwrap_err(),
            bridge::Status::Authentication
        );
        let wrong = KeyPair::from_seed(Seed::new([28; 32]));
        assert_eq!(
            bridge::verify_with_key(&valid, 0, wrong.pk).unwrap_err(),
            bridge::Status::Authentication
        );
        assert_eq!(
            bridge::verify_with_key(&valid, 1, keys.pk).unwrap_err(),
            bridge::Status::Replay
        );
        let excessive =
            bridge::signed_frame(&keys.sk, 2, bridge::RETURN_7_MODULE, bridge::MAX_FUEL + 1);
        assert_eq!(
            bridge::verify_with_key(&excessive, 0, keys.pk).unwrap_err(),
            bridge::Status::Scope
        );
        let mut bounds = valid;
        bounds[16..18].copy_from_slice(&4095u16.to_le_bytes());
        assert_eq!(
            bridge::verify_with_key(&bounds, 0, keys.pk).unwrap_err(),
            bridge::Status::Malformed
        );
        // Changing shared storage after capture cannot change authenticated bytes.
        let mut shared = valid;
        let captured = unsafe { bridge::snapshot(shared.as_ptr()) };
        shared[bridge::PAYLOAD_OFFSET + 10] ^= 1;
        assert!(bridge::verify_with_key(&captured, 0, keys.pk).is_ok());
        assert!(bridge::verify_with_key(&shared, 0, keys.pk).is_err());
        let mut hostile = [0; bridge::PAGE_SIZE];
        for index in 0..bridge::PAGE_SIZE {
            hostile[index] = 255;
            assert!(bridge::parse_descriptor(&hostile).is_err());
        }
    }
}
