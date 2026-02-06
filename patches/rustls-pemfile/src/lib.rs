//! Compatibility wrapper for unmaintained rustls-pemfile crate
//!
//! This is a local patch that provides the rustls-pemfile API using rustls-pki-types.
//! See: RUSTSEC-2025-0134
//!
//! The rustls-pemfile crate is no longer maintained. This wrapper provides the old API
//! on top of rustls-pki-types 1.9+, which includes PEM parsing functionality.

use std::io::{self, BufRead, Read};

// Re-export key types that rustls-pemfile used to provide
pub use rustls_pki_types::{
    CertificateDer, PrivateKeyDer, PrivatePkcs1KeyDer, PrivatePkcs8KeyDer, PrivateSec1KeyDer,
};

/// Represents a PEM item that has been parsed
#[derive(Debug, PartialEq)]
pub enum Item {
    X509Certificate(CertificateDer<'static>),
    Pkcs1Key(PrivatePkcs1KeyDer<'static>),
    Pkcs8Key(PrivatePkcs8KeyDer<'static>),
    Sec1Key(PrivateSec1KeyDer<'static>),
}

/// Extract all certificates from a PEM source
pub fn certs(input: &mut dyn BufRead) -> Result<Vec<CertificateDer<'static>>, io::Error> {
    let mut certs = Vec::new();
    let mut data = Vec::new();
    input.read_to_end(&mut data)?;

    for item in CertificateDer::pem_slice_iter(&data) {
        match item {
            Ok(cert) => certs.push(cert),
            Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid PEM certificate")),
        }
    }

    Ok(certs)
}

/// Extract PKCS8-encoded private keys from a PEM source
pub fn pkcs8_private_keys(input: &mut dyn BufRead) -> Result<Vec<PrivatePkcs8KeyDer<'static>>, io::Error> {
    let mut keys = Vec::new();
    let mut data = Vec::new();
    input.read_to_end(&mut data)?;

    for item in PrivatePkcs8KeyDer::pem_slice_iter(&data) {
        match item {
            Ok(key) => keys.push(key),
            Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid PEM PKCS8 key")),
        }
    }

    Ok(keys)
}

/// Extract RSA private keys in PKCS1 format from a PEM source
pub fn rsa_private_keys(input: &mut dyn BufRead) -> Result<Vec<PrivatePkcs1KeyDer<'static>>, io::Error> {
    let mut keys = Vec::new();
    let mut data = Vec::new();
    input.read_to_end(&mut data)?;

    for item in PrivatePkcs1KeyDer::pem_slice_iter(&data) {
        match item {
            Ok(key) => keys.push(key),
            Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid PEM PKCS1 key")),
        }
    }

    Ok(keys)
}

/// Extract EC private keys in SEC1 format from a PEM source
pub fn ec_private_keys(input: &mut dyn BufRead) -> Result<Vec<PrivateSec1KeyDer<'static>>, io::Error> {
    let mut keys = Vec::new();
    let mut data = Vec::new();
    input.read_to_end(&mut data)?;

    for item in PrivateSec1KeyDer::pem_slice_iter(&data) {
        match item {
            Ok(key) => keys.push(key),
            Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid PEM SEC1 key")),
        }
    }

    Ok(keys)
}

/// Read a single PEM item from input
pub fn read_one(input: &mut dyn BufRead) -> Result<Option<Item>, io::Error> {
    let mut data = Vec::new();
    input.read_to_end(&mut data)?;

    if data.is_empty() {
        return Ok(None);
    }

    // Try each type in order
    if let Some(Ok(cert)) = CertificateDer::pem_slice_iter(&data).next() {
        return Ok(Some(Item::X509Certificate(cert)));
    }

    if let Some(Ok(key)) = PrivatePkcs8KeyDer::pem_slice_iter(&data).next() {
        return Ok(Some(Item::Pkcs8Key(key)));
    }

    if let Some(Ok(key)) = PrivatePkcs1KeyDer::pem_slice_iter(&data).next() {
        return Ok(Some(Item::Pkcs1Key(key)));
    }

    if let Some(Ok(key)) = PrivateSec1KeyDer::pem_slice_iter(&data).next() {
        return Ok(Some(Item::Sec1Key(key)));
    }

    Err(io::Error::new(io::ErrorKind::InvalidData, "no valid PEM item found"))
}

/// Iterator-based reader for PEM items
pub fn read_all(input: &mut dyn BufRead) -> Result<Vec<Item>, io::Error> {
    let mut items = Vec::new();
    let mut data = Vec::new();
    input.read_to_end(&mut data)?;

    // Collect all certificates
    for cert in CertificateDer::pem_slice_iter(&data).flatten() {
        items.push(Item::X509Certificate(cert));
    }

    // Collect all keys
    for key in PrivatePkcs8KeyDer::pem_slice_iter(&data).flatten() {
        items.push(Item::Pkcs8Key(key));
    }

    for key in PrivatePkcs1KeyDer::pem_slice_iter(&data).flatten() {
        items.push(Item::Pkcs1Key(key));
    }

    for key in PrivateSec1KeyDer::pem_slice_iter(&data).flatten() {
        items.push(Item::Sec1Key(key));
    }

    Ok(items)
}
