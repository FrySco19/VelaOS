//! Never collect a Windows PIN: the OS owns that input.
use std::io::{self, IsTerminal, Write};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consent {
    WindowsVerified,
    ConsoleDevelopment,
    MockUnavailableDevelopment,
}
// Explicitly authorized development mode. This is synthetic consent, never a
// human attestation. Available verifiers and user denial cannot be bypassed.
pub fn verify_mock_unavailable(message: &str) -> Result<Consent, String> {
    match native(message) {
        Ok(()) => Ok(Consent::WindowsVerified),
        Err(NativeError::Denied(reason)) => Err(reason),
        Err(NativeError::Unavailable(reason)) => {
            println!(
                "CONSENT: MOCK_UNAVAILABLE_DEVELOPMENT; no human authentication\n{reason}\n{message}"
            );
            Ok(Consent::MockUnavailableDevelopment)
        }
    }
}
pub fn verify(message: &str, allow_console: bool) -> Result<Consent, String> {
    match native(message) {
        Ok(()) => Ok(Consent::WindowsVerified),
        Err(NativeError::Denied(reason)) => Err(reason),
        Err(NativeError::Unavailable(reason)) if allow_console => {
            if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
                return Err(format!("{reason}; interactive console required"));
            }
            println!(
                "\nDEVELOPMENT ONLY — MOCK HUMAN APPROVAL\n{reason}\n{message}\nType APPROVE for these exact bytes; anything else denies."
            );
            io::stdout().flush().map_err(|e| e.to_string())?;
            let mut answer = String::new();
            io::stdin()
                .read_line(&mut answer)
                .map_err(|e| e.to_string())?;
            if answer.trim() != "APPROVE" {
                return Err("Development approval denied".into());
            }
            println!("CONSENT: CONSOLE_DEVELOPMENT (not Windows Hello)");
            Ok(Consent::ConsoleDevelopment)
        }
        Err(NativeError::Unavailable(reason)) => Err(reason),
    }
}
enum NativeError {
    Unavailable(String),
    Denied(String),
}
#[cfg(windows)]
fn native(message: &str) -> Result<(), NativeError> {
    use windows::{
        Security::Credentials::UI::{
            UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
        },
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        core::HSTRING,
    };
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
        .map_err(|e| NativeError::Unavailable(e.to_string()))?;
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                RoUninitialize();
            }
        }
    }
    let _apartment = Apartment;
    let availability = UserConsentVerifier::CheckAvailabilityAsync()
        .and_then(|op| op.get())
        .map_err(|e| NativeError::Unavailable(e.to_string()))?;
    if availability != UserConsentVerifierAvailability::Available {
        let reason = format!("Windows verifier unavailable: {}", availability.0);
        return Err(
            if availability == UserConsentVerifierAvailability::DeviceNotPresent
                || availability == UserConsentVerifierAvailability::NotConfiguredForUser
            {
                NativeError::Unavailable(reason)
            } else {
                NativeError::Denied(reason)
            },
        );
    }
    // Win10 supports the API, but desktop processes lack a UWP CoreWindow.
    // Activation errors are reported; they are never successful authentication.
    let result = UserConsentVerifier::RequestVerificationAsync(&HSTRING::from(message))
        .and_then(|op| op.get())
        .map_err(|e| NativeError::Unavailable(e.to_string()))?;
    if result == UserConsentVerificationResult::Verified {
        return Ok(());
    }
    Err(NativeError::Denied(format!(
        "Windows verification denied: {}",
        result.0
    )))
}
#[cfg(not(windows))]
fn native(_: &str) -> Result<(), NativeError> {
    Err(NativeError::Unavailable(
        "Windows verification unavailable on this platform".into(),
    ))
}
