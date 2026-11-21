//! SigmaOS tu-bylaws component.
//!
//! Rust implementation of `archlinux/tu-bylaws`: the Trusted User Bylaws
//! document and its governance rules — most importantly the Standard
//! Voting Procedure (SVP) state machine.
//!
//! Ports the document model (AsciiDoc sections) plus the SVP lifecycle:
//! proposal → discussion period → voting period → tally → accept/reject,
//! including the acceptance thresholds from the bylaws text.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod document;
pub mod error;
pub mod svp;

pub use document::{BylawsDocument, Section, load_document, parse_document};
pub use error::BylawsError;
pub use svp::{Proposal, ProposalKind, ProposalState, SvpConfig, SvpEngine, Vote, VoteTally};
