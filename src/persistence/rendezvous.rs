//! Rendezvous hashing for deterministic chunk holder selection.
//!
//! This module implements the rendezvous hashing algorithm (also known as
//! highest random weight hashing) for selecting which bots should hold
//! replicas of each chunk.
//!
//! ## Design
//!
//! - **Deterministic**: Same inputs always produce same outputs
//! - **Uniform**: Load distributed evenly across all holders
//! - **Stable**: Minimal reassignment when bots join/leave
//! - **Unbiased**: Owner cannot influence holder selection
//!
//! ## Security Properties
//!
//! - **No Owner Influence**: Owner cannot game the algorithm
//! - **Adversarial Holders**: All holders are untrusted adversaries
//! - **Chunk Privacy**: Holders cannot read encrypted chunks
//! - **Recovery Resistance**: Need ALL chunks + ACI key to reconstruct
//!
//! ## References
//!
//! - Algorithm: docs/PERSISTENCE.md § Distribution
//! - Validation: Q11 spike (.beads/persistence-model.bead)

use sha2::{Digest, Sha256};

/// Compute the replica holders for a specific chunk using rendezvous hashing.
///
/// # Algorithm
///
/// For each candidate bot (excluding owner):
/// 1. Compute score = SHA256(owner || chunk_index || candidate || epoch)
/// 2. Sort all candidates by score (descending)
/// 3. Return top 2 candidates as replica holders
///
/// # Properties
///
/// - **Deterministic**: Same (owner, chunk_index, bots, epoch) → same holders
/// - **Uniform**: Each bot gets roughly equal number of chunks to hold
/// - **Stable**: When bots join/leave, only affected chunks reassign
/// - **Collision Resistant**: SHA256 ensures different chunks → different holders
///
/// # Arguments
///
/// * `owner_contract` - Owner's contract hash (whose chunk this is)
/// * `chunk_index` - Index of this chunk (0-based)
/// * `registered_bots` - All bots currently in the persistence network
/// * `epoch` - Current epoch (changes with >10% network churn)
/// * `num_replicas` - Number of replicas to select (default 2)
///
/// # Returns
///
/// Vector of contract hashes for replica holders (up to `num_replicas` bots)
///
/// # Panics
///
/// Panics if there are fewer than `num_replicas` eligible bots (excluding owner).
/// Caller should check network size before calling.
///
/// # Example
///
/// ```ignore
/// let holders = compute_chunk_holders(
///     "owner-bot-123",
///     0,  // First chunk
///     &["bot-a", "bot-b", "bot-c", "bot-d"],
///     1,  // Epoch 1
///     2,  // 2 replicas
/// );
/// assert_eq!(holders.len(), 2);
/// assert!(!holders.contains(&"owner-bot-123".to_string()));
/// ```
pub fn compute_chunk_holders(
    owner_contract: &str,
    chunk_index: u32,
    registered_bots: &[String],
    epoch: u64,
    num_replicas: usize,
) -> Vec<String> {
    // Filter out owner (can't hold own chunks)
    let candidates: Vec<&String> = registered_bots
        .iter()
        .filter(|bot| *bot != owner_contract)
        .collect();

    if candidates.len() < num_replicas {
        panic!(
            "Not enough bots for replication: need {}, have {} (excluding owner)",
            num_replicas,
            candidates.len()
        );
    }

    // Compute score for each candidate
    let mut scored_candidates: Vec<(String, Vec<u8>)> = candidates
        .iter()
        .map(|bot| {
            let score = compute_rendezvous_score(owner_contract, chunk_index, bot, epoch);
            ((*bot).clone(), score)
        })
        .collect();

    // Sort by score (descending - highest score first)
    scored_candidates.sort_by(|a, b| b.1.cmp(&a.1));

    // Return top N candidates
    scored_candidates
        .into_iter()
        .take(num_replicas)
        .map(|(bot, _score)| bot)
        .collect()
}

/// Compute rendezvous hash score for a candidate holder.
///
/// # Algorithm
///
/// score = SHA256(owner || chunk_index || candidate || epoch)
///
/// Uses SHA256 for:
/// - Cryptographic collision resistance
/// - Uniform distribution of scores
/// - Deterministic output
///
/// # Arguments
///
/// * `owner` - Owner's contract hash
/// * `chunk_index` - Chunk index
/// * `candidate` - Candidate holder's contract hash
/// * `epoch` - Current epoch
///
/// # Returns
///
/// 32-byte hash used as score
fn compute_rendezvous_score(owner: &str, chunk_index: u32, candidate: &str, epoch: u64) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(owner.as_bytes());
    hasher.update(chunk_index.to_le_bytes());
    hasher.update(candidate.as_bytes());
    hasher.update(epoch.to_le_bytes());
    hasher.finalize().to_vec()
}

/// Compute all chunk holders for a bot's state.
///
/// Convenience function that computes holders for all chunks at once.
///
/// # Arguments
///
/// * `owner_contract` - Owner's contract hash
/// * `num_chunks` - Total number of chunks for this bot's state
/// * `registered_bots` - All bots in the persistence network
/// * `epoch` - Current epoch
/// * `num_replicas` - Number of replicas per chunk (default 2)
///
/// # Returns
///
/// Vector of holder lists, one per chunk. Index i contains holders for chunk i.
///
/// # Example
///
/// ```ignore
/// let all_holders = compute_all_chunk_holders(
///     "owner-bot",
///     3,  // 3 chunks
///     &["bot-a", "bot-b", "bot-c", "bot-d"],
///     1,  // Epoch 1
///     2,  // 2 replicas per chunk
/// );
/// assert_eq!(all_holders.len(), 3);
/// assert_eq!(all_holders[0].len(), 2);
/// ```
pub fn compute_all_chunk_holders(
    owner_contract: &str,
    num_chunks: u32,
    registered_bots: &[String],
    epoch: u64,
    num_replicas: usize,
) -> Vec<Vec<String>> {
    (0..num_chunks)
        .map(|chunk_index| {
            compute_chunk_holders(
                owner_contract,
                chunk_index,
                registered_bots,
                epoch,
                num_replicas,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    fn test_bots() -> Vec<String> {
        vec![
            "bot-a".to_string(),
            "bot-b".to_string(),
            "bot-c".to_string(),
            "bot-d".to_string(),
            "bot-e".to_string(),
        ]
    }

    #[test]
    fn test_compute_chunk_holders_basic() {
        let owner = "owner-bot";
        let bots = test_bots();

        let holders = compute_chunk_holders(owner, 0, &bots, 1, 2);

        assert_eq!(holders.len(), 2);
        assert!(!holders.contains(&owner.to_string()));
    }

    #[test]
    fn test_deterministic_selection() {
        let owner = "owner-bot";
        let bots = test_bots();

        let holders1 = compute_chunk_holders(owner, 0, &bots, 1, 2);
        let holders2 = compute_chunk_holders(owner, 0, &bots, 1, 2);

        assert_eq!(holders1, holders2, "Should produce same holders");
    }

    #[test]
    fn test_different_chunks_different_holders() {
        let owner = "owner-bot";
        let bots = test_bots();

        let holders_chunk0 = compute_chunk_holders(owner, 0, &bots, 1, 2);
        let holders_chunk1 = compute_chunk_holders(owner, 1, &bots, 1, 2);

        // Different chunks should (probably) have different holders
        // Not guaranteed, but highly likely with 5 bots
        assert_ne!(
            holders_chunk0, holders_chunk1,
            "Different chunks should distribute to different holders"
        );
    }

    #[test]
    fn test_owner_never_selected() {
        let owner = "owner-bot";
        let mut bots = test_bots();
        bots.push(owner.to_string());

        for chunk_index in 0..10 {
            let holders = compute_chunk_holders(owner, chunk_index, &bots, 1, 2);
            assert!(!holders.contains(&owner.to_string()));
        }
    }

    #[test]
    fn test_epoch_change_affects_selection() {
        let owner = "owner-bot";
        let bots = test_bots();

        let holders_epoch1 = compute_chunk_holders(owner, 0, &bots, 1, 2);
        let holders_epoch2 = compute_chunk_holders(owner, 0, &bots, 2, 2);

        // Different epochs should (probably) produce different holders
        // Ensures holder redistribution when network changes
        assert_ne!(
            holders_epoch1, holders_epoch2,
            "Different epochs should change holder selection"
        );
    }

    #[test]
    fn test_uniform_distribution() {
        let owner = "owner-bot";
        let bots = test_bots();
        let num_chunks = 100;

        // Count how many chunks each bot holds
        let mut holder_counts: HashMap<String, usize> = HashMap::new();

        for chunk_index in 0..num_chunks {
            let holders = compute_chunk_holders(owner, chunk_index, &bots, 1, 2);
            for holder in holders {
                *holder_counts.entry(holder).or_insert(0) += 1;
            }
        }

        // With 100 chunks, 2 replicas each = 200 total placements
        // 5 bots (excluding owner) should each get ~40 placements
        // Allow 20% variance (32-48 placements per bot)
        let expected = 40;
        let min_allowed = 32;
        let max_allowed = 48;

        for (bot, count) in holder_counts {
            assert!(
                count >= min_allowed && count <= max_allowed,
                "Bot {} has {} placements, expected {} ±20%",
                bot,
                count,
                expected
            );
        }
    }

    #[test]
    fn test_compute_all_chunk_holders() {
        let owner = "owner-bot";
        let bots = test_bots();
        let num_chunks = 3;

        let all_holders = compute_all_chunk_holders(owner, num_chunks, &bots, 1, 2);

        assert_eq!(all_holders.len(), 3);
        for holders in all_holders {
            assert_eq!(holders.len(), 2);
            assert!(!holders.contains(&owner.to_string()));
        }
    }

    #[test]
    fn test_stability_under_bot_addition() {
        let owner = "owner-bot";
        let mut bots = test_bots();
        let num_chunks = 20;

        // Compute holders with original bot set
        let original_holders: Vec<_> = (0..num_chunks)
            .map(|i| compute_chunk_holders(owner, i, &bots, 1, 2))
            .collect();

        // Add new bot
        bots.push("bot-f".to_string());

        // Compute holders with expanded bot set
        let new_holders: Vec<_> = (0..num_chunks)
            .map(|i| compute_chunk_holders(owner, i, &bots, 1, 2))
            .collect();

        // Count how many chunks changed holders
        let mut changes = 0;
        for (orig, new) in original_holders.iter().zip(new_holders.iter()) {
            if orig != new {
                changes += 1;
            }
        }

        // With rendezvous hashing, adding 1 bot to 5 bots (~20% increase)
        // should cause minimal reassignment. Expect <50% of chunks to move.
        assert!(
            changes < num_chunks / 2,
            "Too many chunks reassigned: {}/{}",
            changes,
            num_chunks
        );
    }

    #[test]
    #[should_panic(expected = "Not enough bots for replication")]
    fn test_insufficient_bots() {
        let owner = "owner-bot";
        let bots = vec!["bot-a".to_string()]; // Only 1 bot, need 2 replicas

        compute_chunk_holders(owner, 0, &bots, 1, 2);
    }

    #[test]
    fn test_no_duplicate_holders() {
        let owner = "owner-bot";
        let bots = test_bots();

        for chunk_index in 0..10 {
            let holders = compute_chunk_holders(owner, chunk_index, &bots, 1, 2);
            let unique: HashSet<_> = holders.iter().collect();
            assert_eq!(
                holders.len(),
                unique.len(),
                "Holders should be unique for chunk {}",
                chunk_index
            );
        }
    }

    #[test]
    fn test_different_owners_same_chunk_index() {
        let bots = test_bots();

        let holders_owner_a = compute_chunk_holders("owner-a", 0, &bots, 1, 2);
        let holders_owner_b = compute_chunk_holders("owner-b", 0, &bots, 1, 2);

        // Different owners should get different holders (even for same chunk index)
        assert_ne!(
            holders_owner_a, holders_owner_b,
            "Different owners should have different holder assignments"
        );
    }

    #[test]
    fn test_score_collision_resistance() {
        let owner = "owner-bot";
        let bots = test_bots();

        // Compute scores for multiple chunks
        let mut scores = HashSet::new();
        for chunk_index in 0..100 {
            for bot in &bots {
                let score = compute_rendezvous_score(owner, chunk_index, bot, 1);
                scores.insert(score);
            }
        }

        // With 100 chunks × 5 bots = 500 scores, should have no collisions
        // (SHA256 has 2^256 possible outputs)
        assert_eq!(scores.len(), 500, "Should have no score collisions");
    }
}
