//! Drift-auth: Multi-signature broadcast authentication built on iroh's Ed25519 keys.
//!
//! # Design Principle
//!
//! This crate uses iroh's existing Ed25519 keys as node identity. No separate key management is needed.
//!
//! # Crypto Primitives (Items 1-5)
//!
//! - Item 1: Use existing iroh Ed25519 keys
//! - Item 2: Sign auth messages with iroh keypair
//! - Item 3: Verify signatures with iroh public key
//! - Item 4: Aggregate signatures for threshold m-of-n
//! - Item 5: Unit tests for sign/verify/aggregate

pub mod crypto;
pub mod rng;
pub mod messages;
pub mod aggregator;
pub mod node;
pub mod coordinator;
pub mod replay;
pub mod config;

pub use crypto::{
    CryptoError,
    sign_message_with_iroh_keypair,
    verify_signature_with_iroh_pubkey,
    aggregate_signatures,
};

pub use rng::CryptoOsRng;

pub use config::{
    AuthConfig,
    ConfigError,
    AuthError,
    SignatureError,
    TimeoutError,
    AuthMetrics,
};

pub use messages::{
    AuthMessage,
    SignedAuthMessage,
    AggregateAuthMessage,
};

pub use aggregator::{Aggregator, AggregationError};
pub use coordinator::{CoordinatorAuth, BroadcastError, KeyRotationError};