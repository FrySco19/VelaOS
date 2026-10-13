//! USB HID in production; explicit interactive virtual-touch development mode.
use authenticator::{
    StatusUpdate,
    authenticatorservice::{AuthenticatorService, RegisterArgs, SignArgs},
    crypto::{COSEAlgorithm, COSEKeyType, Curve},
    ctap2::{
        attestation::AuthenticatorDataFlags,
        server::{
            AuthenticationExtensionsClientInputs, PublicKeyCredentialDescriptor,
            PublicKeyCredentialParameters, PublicKeyCredentialUserEntity, RelyingParty,
            ResidentKeyRequirement, Transport, UserVerificationRequirement,
        },
    },
    statecallback::StateCallback,
};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};
const RP: &str = "vela.local";
const ORIGIN: &str = "https://vela.local";
const TIMEOUT: u64 = 60_000;
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Credential {
    version: u8,
    rp: String,
    id: Vec<u8>,
    public: Vec<u8>,
    counter: u32,
    ctap1: bool,
}
fn err(e: impl std::fmt::Debug) -> String {
    format!("FIDO2: {e:?}")
}
fn service() -> Result<AuthenticatorService, String> {
    let mut s = AuthenticatorService::new().map_err(err)?;
    s.add_u2f_usb_hid_platform_transports();
    Ok(s)
}
fn statuses() -> mpsc::Sender<StatusUpdate> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        while let Ok(status) = rx.recv() {
            match status {
                StatusUpdate::PresenceRequired | StatusUpdate::SelectDeviceNotice => {
                    println!("[SECURITY] Tocca la chiave USB registrata.")
                }
                StatusUpdate::PinUvError(_) => eprintln!(
                    "[SECURITY] La chiave richiede PIN/UV: questa policy touch-only rifiuta la richiesta."
                ),
                _ => {}
            }
        }
    });
    tx
}
fn wait<T>(
    s: &mut AuthenticatorService,
    rx: mpsc::Receiver<authenticator::Result<T>>,
) -> Result<T, String> {
    let result = rx
        .recv_timeout(Duration::from_millis(TIMEOUT + 2000))
        .map_err(|_| "FIDO2 timeout o canale chiuso".to_string())
        .and_then(|v| v.map_err(err));
    let _ = s.cancel();
    result
}
fn assertion(c: &Credential, hash: [u8; 32]) -> Result<authenticator::SignResult, String> {
    let mut s = service()?;
    let (tx, rx) = mpsc::channel();
    let args = SignArgs {
        client_data_hash: hash,
        origin: ORIGIN.into(),
        relying_party_id: RP.into(),
        allow_list: vec![PublicKeyCredentialDescriptor {
            id: c.id.clone(),
            transports: vec![Transport::USB],
        }],
        user_verification_req: UserVerificationRequirement::Discouraged,
        user_presence_req: true,
        extensions: AuthenticationExtensionsClientInputs::default(),
        pin: None,
        use_ctap1_fallback: c.ctap1,
    };
    s.sign(
        TIMEOUT,
        args,
        statuses(),
        StateCallback::new(Box::new(move |r| {
            let _ = tx.send(r);
        })),
    )
    .map_err(err)?;
    wait(&mut s, rx)
}
fn check(
    c: &Credential,
    hash: &[u8; 32],
    result: &authenticator::SignResult,
    last: u32,
) -> Result<u32, String> {
    let a = &result.assertion;
    let data = a.auth_data.to_vec();
    if c.version != 1
        || c.rp != RP
        || c.id.is_empty()
        || c.id.len() > 1024
        || a.credentials.as_ref().is_some_and(|v| v.id != c.id)
        || !a
            .auth_data
            .flags
            .contains(AuthenticatorDataFlags::USER_PRESENT)
        || a.auth_data.credential_data.is_some()
        || a.auth_data.rp_id_hash.0 != <[u8; 32]>::from(Sha256::digest(RP.as_bytes()))
    {
        return Err("FIDO2 credential/RP/User Presence mismatch".into());
    }
    // Zero counters are supported by some authenticators; challenge uniqueness
    // still prevents replay. Never accept zero/decrease after a nonzero counter.
    let next = a.auth_data.counter;
    if (last != 0 || next != 0) && next <= last {
        return Err("FIDO2 contatore ripetuto o regressivo".into());
    }
    let key = VerifyingKey::from_sec1_bytes(&c.public).map_err(err)?;
    let signature = Signature::from_der(&a.signature).map_err(err)?;
    let mut signed = data;
    signed.extend_from_slice(hash);
    key.verify(&signed, &signature).map_err(err)?;
    Ok(next)
}
fn journal(path: &Path) -> PathBuf {
    path.with_extension("counter")
}
fn load(path: &Path) -> Result<(Credential, u32), String> {
    let bytes =
        fs::read(path).map_err(|_| "Chiave FIDO2 non registrata: usare enroll-fido".to_string())?;
    if bytes.len() > 4096 {
        return Err("Credential troppo grande".into());
    }
    let c: Credential = serde_json::from_slice(&bytes).map_err(err)?;
    if c.version != 1 || c.rp != RP || c.id.is_empty() || c.id.len() > 1024 || c.public.len() != 65
    {
        return Err("Credential FIDO2 non valida".into());
    }
    VerifyingKey::from_sec1_bytes(&c.public).map_err(err)?;
    let bytes = fs::read(journal(path)).map_err(err)?;
    if bytes.len() % 4 != 0 || bytes.len() > 1024 * 1024 {
        return Err("Journal FIDO2 incompleto o pieno".into());
    }
    let mut last = c.counter;
    for b in bytes.chunks_exact(4) {
        let n = u32::from_le_bytes(b.try_into().unwrap());
        if (last != 0 || n != 0) && n <= last {
            return Err("Journal FIDO2 regressivo".into());
        }
        last = n;
    }
    Ok((c, last))
}
pub struct HumanPresence {
    plan: [u8; 32],
    virtual_touch: bool,
}
impl HumanPresence {
    pub fn consent(&self) -> &'static str {
        if self.virtual_touch {
            "DevelopmentVirtualFido"
        } else {
            "HardwareFido2Verified"
        }
    }
    pub fn permits(&self, plan: &[u8; 32]) -> bool {
        &self.plan == plan
    }
}
/// A fresh assertion is verified before this opaque, non-cloneable permit exists.
pub fn verify(path: &Path, plan: [u8; 32]) -> Result<HumanPresence, String> {
    let (c, last) = load(path)?;
    let mut nonce = [0; 32];
    getrandom::getrandom(&mut nonce).map_err(err)?;
    let mut body = b"Vela.HardwareApproval.v1\0".to_vec();
    body.extend_from_slice(&nonce);
    body.extend_from_slice(&plan);
    let challenge: [u8; 32] = Sha256::digest(body).into();
    println!(
        "[SECURITY] Richiesta tocco su token hardware FIDO2 per autorizzare l'hash: {}",
        plan.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    let result = assertion(&c, challenge)?;
    let next = check(&c, &challenge, &result, last)?;
    let mut file = OpenOptions::new()
        .append(true)
        .open(journal(path))
        .map_err(err)?;
    file.write_all(&next.to_le_bytes()).map_err(err)?;
    file.sync_all().map_err(err)?;
    Ok(HumanPresence {
        plan,
        virtual_touch: false,
    })
}
/// CLI mode only. No environment switch, automatic success or piped approval.
pub fn authorize(path: &Path, plan: [u8; 32], dev_virtual: bool) -> Result<HumanPresence, String> {
    if !dev_virtual {
        return verify(path, plan);
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(
            "Dev virtual FIDO requires an interactive terminal; redirected input is denied".into(),
        );
    }
    println!(
        "[DEV VIRTUAL FIDO — NOT HARDWARE] Piano SHA-256: {}",
        plan.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    print!("Premi [INVIO] per simulare il tocco; qualsiasi altro testo rifiuta: ");
    io::stdout().flush().map_err(err)?;
    let mut line = String::new();
    let count = io::stdin().read_line(&mut line).map_err(err)?;
    if count == 0 || !matches!(line.as_str(), "\n" | "\r\n") {
        return Err("Dev virtual touch denied (input or EOF)".into());
    }
    Ok(HumanPresence {
        plan,
        virtual_touch: true,
    })
}
pub fn dev_warning() {
    eprintln!(
        "\x1b[1;31m\n============================================================\n!!! DEV VIRTUAL FIDO ATTIVO — NESSUNA AUTENTICAZIONE HARDWARE !!!\nINVIO SIMULA IL TOCCO. SOLO SVILUPPO. NON USARE IN PRODUZIONE.\nFIRME Ed25519 REALI; CONSENSO FIDO2 SIMULATO.\n============================================================\n\x1b[0m"
    );
}
pub fn enroll(path: &Path, ctap1: bool) -> Result<(), String> {
    if path.exists() || journal(path).exists() {
        return Err("Registrazione esistente: nessuna sovrascrittura automatica".into());
    }
    let mut nonce = [0; 32];
    getrandom::getrandom(&mut nonce).map_err(err)?;
    let mut s = service()?;
    let (tx, rx) = mpsc::channel();
    println!(
        "[SECURITY] Registrazione locale: tocca la chiave USB. Serve un secondo tocco per provarne il possesso."
    );
    let args = RegisterArgs {
        client_data_hash: nonce,
        relying_party: RelyingParty {
            id: RP.into(),
            name: Some("Vela Soft".into()),
        },
        origin: ORIGIN.into(),
        user: PublicKeyCredentialUserEntity {
            id: nonce.to_vec(),
            name: Some("Vela local user".into()),
            display_name: None,
        },
        pub_cred_params: vec![PublicKeyCredentialParameters {
            alg: COSEAlgorithm::ES256,
        }],
        exclude_list: vec![],
        user_verification_req: UserVerificationRequirement::Discouraged,
        resident_key_req: ResidentKeyRequirement::Discouraged,
        extensions: AuthenticationExtensionsClientInputs::default(),
        pin: None,
        use_ctap1_fallback: ctap1,
    };
    s.register(
        TIMEOUT,
        args,
        statuses(),
        StateCallback::new(Box::new(move |r| {
            let _ = tx.send(r);
        })),
    )
    .map_err(err)?;
    let result = wait(&mut s, rx)?;
    let auth = result.att_obj.auth_data;
    if !auth.flags.contains(AuthenticatorDataFlags::USER_PRESENT)
        || auth.rp_id_hash.0 != <[u8; 32]>::from(Sha256::digest(RP.as_bytes()))
    {
        return Err("Registration RP/presence mismatch".into());
    }
    let credential = auth
        .credential_data
        .ok_or("Missing registered credential")?;
    if credential.credential_public_key.alg != COSEAlgorithm::ES256 {
        return Err("Only ES256 hardware keys are enrolled".into());
    }
    let COSEKeyType::EC2(key) = credential.credential_public_key.key else {
        return Err("Not an EC2 key".into());
    };
    if key.curve != Curve::SECP256R1 || key.x.len() != 32 || key.y.len() != 32 {
        return Err("Invalid P256 key".into());
    }
    let mut public = vec![4];
    public.extend_from_slice(&key.x);
    public.extend_from_slice(&key.y);
    let mut c = Credential {
        version: 1,
        rp: RP.into(),
        id: credential.credential_id,
        public,
        counter: auth.counter,
        ctap1,
    };
    getrandom::getrandom(&mut nonce).map_err(err)?;
    let result = assertion(&c, nonce)?;
    c.counter = check(&c, &nonce, &result, c.counter)?;
    let mut counter = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(journal(path))
        .map_err(err)?;
    counter.flush().map_err(err)?;
    counter.sync_all().map_err(err)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(err)?;
    file.write_all(&serde_json::to_vec(&c).map_err(err)?)
        .map_err(err)?;
    file.sync_all().map_err(err)?;
    println!("[SECURITY] Credential USB registrata e firma verificata.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use authenticator::ctap2::{
        attestation::AuthenticatorData,
        server::{AuthenticatorAttachment, RpIdHash},
    };
    use p256::ecdsa::{SigningKey, signature::Signer};
    fn fixture(
        flags: AuthenticatorDataFlags,
        counter: u32,
    ) -> (Credential, authenticator::SignResult, [u8; 32]) {
        let key = SigningKey::from_bytes((&[7u8; 32]).into()).unwrap();
        let c = Credential {
            version: 1,
            rp: RP.into(),
            id: vec![1, 2, 3],
            public: key
                .verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .to_vec(),
            counter: 1,
            ctap1: false,
        };
        let hash = [8; 32];
        let auth = AuthenticatorData {
            rp_id_hash: RpIdHash(Sha256::digest(RP.as_bytes()).into()),
            flags,
            counter,
            credential_data: None,
            extensions: Default::default(),
        };
        let mut bytes = auth.to_vec();
        bytes.extend_from_slice(&hash);
        let sig: Signature = key.sign(&bytes);
        let result = authenticator::SignResult {
            assertion: authenticator::ctap2::commands::get_assertion::Assertion {
                credentials: Some(PublicKeyCredentialDescriptor {
                    id: c.id.clone(),
                    transports: vec![Transport::USB],
                }),
                auth_data: auth,
                signature: sig.to_der().as_bytes().to_vec(),
                user: None,
            },
            attachment: AuthenticatorAttachment::Unknown,
            extensions: Default::default(),
        };
        (c, result, hash)
    }
    #[test]
    fn assertion_requires_signature_presence_challenge_and_counter() {
        let (c, mut result, hash) = fixture(AuthenticatorDataFlags::USER_PRESENT, 2);
        assert_eq!(check(&c, &hash, &result, 1).unwrap(), 2);
        assert!(check(&c, &[9; 32], &result, 1).is_err());
        assert!(check(&c, &hash, &result, 2).is_err());
        result.assertion.signature[0] ^= 1;
        assert!(check(&c, &hash, &result, 1).is_err());
        let (c, result, hash) = fixture(AuthenticatorDataFlags::empty(), 2);
        assert!(check(&c, &hash, &result, 1).is_err());
    }
    #[test]
    fn wrong_rp_or_credential_and_counter_downgrade_are_denied() {
        let (c, mut r, h) = fixture(AuthenticatorDataFlags::USER_PRESENT, 2);
        r.assertion.credentials.as_mut().unwrap().id.push(4);
        assert!(check(&c, &h, &r, 1).is_err());
        let (c, mut r, h) = fixture(AuthenticatorDataFlags::USER_PRESENT, 2);
        r.assertion.auth_data.rp_id_hash.0[0] ^= 1;
        assert!(check(&c, &h, &r, 1).is_err());
        let (c, r, h) = fixture(AuthenticatorDataFlags::USER_PRESENT, 0);
        assert!(check(&c, &h, &r, 1).is_err());
        assert!(check(&c, &h, &r, 0).is_ok());
    }
    #[test]
    fn missing_enrollment_cannot_create_a_presence_permit() {
        let p = std::env::temp_dir().join(format!("vela-no-fido-{}", std::process::id()));
        assert!(verify(&p, [3; 32]).is_err());
    }
}
