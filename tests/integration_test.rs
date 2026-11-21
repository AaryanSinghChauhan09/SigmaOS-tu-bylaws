//! Integration tests for sigmaos-tu-bylaws.

use std::io::Write;
use tu_bylaws::{BylawsDocument, ProposalKind, ProposalState, SvpEngine, Vote, parse_document, load_document};

const ADOC: &str = "Trusted User Bylaws\n===================\nTrusted Users <aur-general@archlinux.org>\n1.3, 2013-08-07\n\nSummary\n-------\nThis document describes the bylaws.\n\nStandard Voting Procedure\n-------------------------\nSVP begins with a proposal.\n";

#[test]
fn test_document_parse() {
    let doc = parse_document(ADOC);
    assert_eq!(doc.title, "Trusted User Bylaws");
    assert_eq!(doc.version, "1.3, 2013-08-07");
    assert!(doc.find_section("voting").is_some());
    assert_eq!(doc.titles().len(), 2);
}

#[test]
fn test_load_document_file() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("tu-bylaws.adoc");
    let mut f = std::fs::File::create(&p).unwrap();
    write!(f, "{}", ADOC).unwrap();
    let doc = load_document(&p).unwrap();
    assert_eq!(doc.sections.len(), 2);
}

#[test]
fn test_svp_engine_lifecycle() {
    let mut eng = SvpEngine::new(8);
    let id = eng.propose("Test proposal", ProposalKind::NewTu, 0);
    // Discussion → voting at day 5+.
    assert_eq!(eng.advance(id, 5 * 86400).unwrap(), ProposalState::Voting);
    // Votes only during voting.
    eng.vote(id, "tu1", Vote::Yes, 5 * 86400).unwrap();
    eng.vote(id, "tu2", Vote::Yes, 5 * 86400).unwrap();
    eng.vote(id, "tu3", Vote::Yes, 5 * 86400).unwrap();
    eng.vote(id, "tu4", Vote::Yes, 5 * 86400).unwrap();
    // 4*2=8 not >8 → reject (need strictly greater than half of 8 = >4... wait half of 8 is 4, yes must be >4 → 4 fails? yes_exceeds_half: yes*2 > total → 8 > 8 false → rejected)
    let (st, t) = eng.tally(id, 12 * 86400).unwrap();
    assert_eq!(t.yes, 4);
    assert_eq!(st, ProposalState::Rejected);
}

#[test]
fn test_document_titles() {
    let doc: BylawsDocument = parse_document(ADOC);
    assert!(doc.titles().contains(&"Summary"));
    assert!(doc.titles().contains(&"Standard Voting Procedure"));
}
