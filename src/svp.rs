//! Standard Voting Procedure (SVP) engine.
//!
//! From the bylaws: SVP = proposal posted on aur-general → 5-day discussion
//! period → 7-day voting period → tally → accept/reject.
//!
//! Acceptance: YES votes > half the TU count, OR (for amendments) 2/3
//! supermajority — the bylaws specify per-proposal-kind thresholds. This
//! engine encodes the lifecycle and threshold math.

use std::collections::BTreeMap;

use crate::BylawsError;

/// Kind of proposal (determines voting threshold).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalKind {
    /// New TU addition — requires YES > half of TUs.
    NewTu,
    /// TU removal — requires YES > half.
    RemoveTu,
    /// Bylaws amendment — requires 2/3 supermajority of votes cast.
    Amendment,
    /// Other business — simple majority of votes cast.
    Other,
}

/// SVP lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProposalState {
    /// Proposal posted, discussion ongoing.
    Discussion,
    /// Discussion ended, voting open.
    Voting,
    /// Voting closed, awaiting tally.
    Tallying,
    /// Proposal accepted.
    Accepted,
    /// Proposal rejected.
    Rejected,
    /// Withdrawn by proposer.
    Withdrawn,
}

/// A single vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    /// Yes.
    Yes,
    /// No.
    No,
    /// Abstain (counted in participation, not in majority math).
    Abstain,
}

/// Vote tally.
#[derive(Debug, Default)]
pub struct VoteTally {
    /// yes/no/abstain counts.
    pub yes: u32,
    /// no votes.
    pub no: u32,
    /// abstentions.
    pub abstain: u32,
}

impl VoteTally {
    /// Total votes cast (excluding abstentions).
    pub fn cast(&self) -> u32 {
        self.yes + self.no
    }
    /// Whether YES votes exceed half of `total_voters` (TU count).
    pub fn yes_exceeds_half(&self, total_voters: u32) -> bool {
        self.yes as u64 * 2 > total_voters as u64
    }
    /// Whether YES votes are ≥ 2/3 of votes cast.
    pub fn yes_two_thirds(&self) -> bool {
        self.cast() > 0 && (self.yes as u64) * 3 >= (self.cast() as u64) * 2
    }
}

/// SVP timing configuration (days).
#[derive(Debug, Clone)]
pub struct SvpConfig {
    /// Discussion period length in days (default 5).
    pub discussion_days: u64,
    /// Voting period length in days (default 7).
    pub voting_days: u64,
}

impl Default for SvpConfig {
    fn default() -> Self {
        Self { discussion_days: 5, voting_days: 7 }
    }
}

/// A proposal under SVP.
#[derive(Debug)]
pub struct Proposal {
    /// Proposal ID.
    pub id: u64,
    /// Title/summary.
    pub title: String,
    /// Kind (drives threshold).
    pub kind: ProposalKind,
    /// Current state.
    pub state: ProposalState,
    /// Unix timestamp when discussion started.
    pub discussion_start: i64,
    /// Unix timestamp when voting started.
    pub voting_start: i64,
    /// Votes by voter ID.
    pub votes: BTreeMap<String, Vote>,
}

/// SVP engine managing proposals.
pub struct SvpEngine {
    /// Timing config.
    pub config: SvpConfig,
    /// Total TU count (for threshold math).
    pub tu_count: u32,
    proposals: BTreeMap<u64, Proposal>,
    next_id: u64,
}

impl SvpEngine {
    /// New engine with TU count.
    pub fn new(tu_count: u32) -> Self {
        Self {
            config: SvpConfig::default(),
            tu_count,
            proposals: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Post a new proposal (enters Discussion state).
    pub fn propose(&mut self, title: &str, kind: ProposalKind, now: i64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.proposals.insert(id, Proposal {
            id,
            title: title.to_string(),
            kind,
            state: ProposalState::Discussion,
            discussion_start: now,
            voting_start: 0,
            votes: BTreeMap::new(),
        });
        id
    }

    /// Advance proposal state based on current time.
    ///
    /// `now` is unix timestamp. Discussion ends after `discussion_days`,
    /// voting ends after `voting_days` from voting_start.
    pub fn advance(&mut self, id: u64, now: i64) -> Result<ProposalState, BylawsError> {
        let p = self.proposals.get_mut(&id)
            .ok_or_else(|| BylawsError::Svp(format!("unknown proposal {id}")))?;
        let day = 86400i64;
        match p.state {
            ProposalState::Discussion => {
                if now >= p.discussion_start + self.config.discussion_days as i64 * day {
                    p.state = ProposalState::Voting;
                    p.voting_start = now;
                }
            }
            ProposalState::Voting => {
                if now >= p.voting_start + self.config.voting_days as i64 * day {
                    p.state = ProposalState::Tallying;
                }
            }
            _ => {}
        }
        Ok(p.state)
    }

    /// Cast a vote — only allowed during Voting state.
    pub fn vote(&mut self, id: u64, voter: &str, vote: Vote, now: i64) -> Result<(), BylawsError> {
        self.advance(id, now)?;
        let p = self.proposals.get_mut(&id).unwrap();
        if p.state != ProposalState::Voting {
            return Err(BylawsError::Svp(format!(
                "cannot vote in state {:?}", p.state
            )));
        }
        p.votes.insert(voter.to_string(), vote);
        Ok(())
    }

    /// Tally votes and decide accept/reject per bylaws thresholds.
    pub fn tally(&mut self, id: u64, now: i64) -> Result<(ProposalState, VoteTally), BylawsError> {
        self.advance(id, now)?;
        let p = self.proposals.get_mut(&id).unwrap();
        if p.state != ProposalState::Tallying {
            return Err(BylawsError::Svp(format!(
                "cannot tally in state {:?}", p.state
            )));
        }
        let mut t = VoteTally::default();
        for v in p.votes.values() {
            match v {
                Vote::Yes => t.yes += 1,
                Vote::No => t.no += 1,
                Vote::Abstain => t.abstain += 1,
            }
        }
        // Acceptance rules per bylaws:
        // - New TU / removal: YES > half of total TUs.
        // - Amendment: YES >= 2/3 of votes cast.
        // - Other: YES > half of votes cast.
        let accepted = match p.kind {
            ProposalKind::NewTu | ProposalKind::RemoveTu => {
                t.yes_exceeds_half(self.tu_count)
            }
            ProposalKind::Amendment => t.yes_two_thirds(),
            ProposalKind::Other => t.cast() > 0 && t.yes * 2 > t.cast(),
        };
        p.state = if accepted { ProposalState::Accepted } else { ProposalState::Rejected };
        Ok((p.state, t))
    }

    /// Get a proposal by ID.
    pub fn get(&self, id: u64) -> Option<&Proposal> {
        self.proposals.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86400;

    #[test]
    fn test_full_svp_cycle() {
        let mut eng = SvpEngine::new(10);
        let id = eng.propose("Add new TU", ProposalKind::NewTu, 0);
        // Day 0: discussion. Vote should fail.
        assert!(eng.vote(id, "tu1", Vote::Yes, 0).is_err());
        // Day 6: discussion over → voting.
        let st = eng.advance(id, 6 * DAY).unwrap();
        assert_eq!(st, ProposalState::Voting);
        // Vote 6 YES out of 10 TUs → 6*2=12 > 10 → accept.
        for i in 0..6 {
            eng.vote(id, &format!("tu{i}"), Vote::Yes, 6 * DAY).unwrap();
        }
        eng.vote(id, "tu6", Vote::No, 6 * DAY).unwrap();
        // Day 14: voting over → tally.
        let (st, t) = eng.tally(id, 14 * DAY).unwrap();
        assert_eq!(st, ProposalState::Accepted);
        assert_eq!(t.yes, 6);
    }

    #[test]
    fn test_rejection_below_threshold() {
        let mut eng = SvpEngine::new(10);
        let id = eng.propose("Remove TU", ProposalKind::RemoveTu, 0);
        eng.advance(id, 6 * DAY).unwrap();
        for i in 0..4 {
            eng.vote(id, &format!("tu{i}"), Vote::Yes, 6 * DAY).unwrap();
        }
        let (st, _) = eng.tally(id, 14 * DAY).unwrap();
        assert_eq!(st, ProposalState::Rejected); // 4*2=8 < 10
    }

    #[test]
    fn test_amendment_needs_two_thirds() {
        let mut eng = SvpEngine::new(100);
        let id = eng.propose("Amend bylaws", ProposalKind::Amendment, 0);
        eng.advance(id, 6 * DAY).unwrap();
        // 7 yes, 3 no → 7/10 = 70% ≥ 66.7% → accept
        for i in 0..7 {
            eng.vote(id, &format!("tu{i}"), Vote::Yes, 6 * DAY).unwrap();
        }
        for i in 7..10 {
            eng.vote(id, &format!("tu{i}"), Vote::No, 6 * DAY).unwrap();
        }
        let (st, _) = eng.tally(id, 14 * DAY).unwrap();
        assert_eq!(st, ProposalState::Accepted);
    }

    #[test]
    fn test_abstain_not_counted() {
        let mut eng = SvpEngine::new(10);
        let id = eng.propose("Other", ProposalKind::Other, 0);
        eng.advance(id, 6 * DAY).unwrap();
        eng.vote(id, "a", Vote::Yes, 6 * DAY).unwrap();
        eng.vote(id, "b", Vote::Abstain, 6 * DAY).unwrap();
        let (_, t) = eng.tally(id, 14 * DAY).unwrap();
        assert_eq!(t.cast(), 1);
        assert_eq!(t.abstain, 1);
    }
}
