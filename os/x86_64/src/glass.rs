//! Bounded, deterministic surface selection. This does not mint authority.
#![cfg_attr(not(test), no_std)]

pub const IDLE: u32 = 0;
pub const INTENT: u32 = 1;
pub const NOTE: u32 = 2;
pub const MEDIA: u32 = 3;
pub const NETWORK: u32 = 4;
pub const RETAINED: u32 = 5;

fn contains(text: &[u8], needle: &[u8]) -> bool {
    text.windows(needle.len()).any(|window| window.iter().zip(needle).all(|(a, b)| a.to_ascii_lowercase() == *b))
}

pub fn classify(text: &[u8]) -> u32 {
    if text.is_empty() || text.len() > 72 { return INTENT; }
    if contains(text, b"wifi") || contains(text, b"wi-fi") || contains(text, b"rete") || contains(text, b"network") { NETWORK }
    else if contains(text, b"appunta") || contains(text, b"note") || contains(text, b"write") { NOTE }
    else if contains(text, b"riproduci") || contains(text, b"play") || contains(text, b"red rain") || contains(text, b"audio") { MEDIA }
    else { INTENT }
}

#[unsafe(no_mangle)]
/// # Safety
/// `text` must point to `len` readable bytes owned by the caller for this call.
pub unsafe extern "C" fn vela_glass_intent(text: *const u8, len: usize) -> u32 {
    if text.is_null() || len == 0 || len > 72 { return INTENT; }
    // SAFETY: pointer lifetime and extent are the C caller's contract.
    classify(unsafe { core::slice::from_raw_parts(text, len) })
}

#[unsafe(no_mangle)]
pub extern "C" fn vela_glass_transition(current: u32, event: u32) -> u32 {
    match event {
        0 => IDLE, // close/cancel: release the visible surface
        1..=4 => event, // manual controls work without a model
        5 if current == NOTE => RETAINED, // RAM draft, not a durable receipt
        _ => current,
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { loop { core::hint::spin_loop(); } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_intents_do_not_claim_unknown_actions() {
        assert_eq!(classify(b"APPUNTA le idee per il bridge IPC"), NOTE);
        assert_eq!(classify(b"Riproduci Red Rain"), MEDIA);
        assert_eq!(classify(b"Connettiti al Wi-Fi dell'ufficio"), NETWORK);
        assert_eq!(classify(b"delete all objects"), INTENT);
        assert_eq!(classify(&[b'n'; 73]), INTENT);
    }
    #[test]
    fn cancel_releases_every_surface_and_only_notes_can_be_retained() {
        for surface in 0..=5 { assert_eq!(vela_glass_transition(surface, 0), IDLE); }
        assert_eq!(vela_glass_transition(NOTE, 5), RETAINED);
        assert_eq!(vela_glass_transition(NETWORK, 5), NETWORK);
        assert_eq!(vela_glass_transition(MEDIA, 5), MEDIA);
        assert_eq!(vela_glass_transition(IDLE, 2), NOTE);
    }
}
