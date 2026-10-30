// Copyright (c) 2026 Harllan He. Licensed under MIT.
use super::super::helpers::{replayed_reasoning, thinking_loss};
use crate::anthropic::converter::{HistoryReasoningReplay, HistoryThinkingLoss};
use crate::kiro::provider::SignatureRepair;
use crate::model::usage::{ReplayedReasoning, ThinkingLoss};

const HISTORY_REPLAY: HistoryReasoningReplay = HistoryReasoningReplay {
    signature_chars: 300,
    blocks: 3,
};

fn targeted_repair() -> SignatureRepair {
    SignatureRepair {
        rejected: vec!["a".repeat(40), "b".repeat(60)],
        stripped_all: false,
    }
}

#[test]
fn replayed_reasoning_excludes_rejected_signatures() {
    assert_eq!(
        replayed_reasoning(HISTORY_REPLAY, &targeted_repair()),
        ReplayedReasoning {
            signature_chars: 200,
            blocks: 1,
        }
    );
}

#[test]
fn replayed_reasoning_is_empty_after_stripping_all() {
    let repair = SignatureRepair {
        stripped_all: true,
        ..targeted_repair()
    };
    assert_eq!(
        replayed_reasoning(HISTORY_REPLAY, &repair),
        ReplayedReasoning::default()
    );
}

#[test]
fn thinking_loss_counts_cached_and_named_rejections() {
    let history = HistoryThinkingLoss {
        invalid_turns: 1,
        ..Default::default()
    };
    assert_eq!(
        thinking_loss(history, &targeted_repair()),
        Some(ThinkingLoss {
            invalid_turns: 3,
            ..Default::default()
        })
    );
    assert_eq!(
        thinking_loss(HistoryThinkingLoss::default(), &SignatureRepair::default()),
        None
    );
}
