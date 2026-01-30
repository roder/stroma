//! Merkle Tree Implementation for Stroma
//!
//! Implements a binary Merkle tree from a BTreeSet of member hashes,
//! supporting root calculation, proof generation, and proof verification.

use std::collections::BTreeSet;

/// 32-byte hash (SHA-256 output)
pub type Hash = [u8; 32];

/// A Merkle tree node
#[derive(Debug, Clone)]
pub enum MerkleNode {
    Leaf(Hash),
    Internal {
        hash: Hash,
        left: Box<MerkleNode>,
        right: Box<MerkleNode>,
    },
}

impl MerkleNode {
    pub fn hash(&self) -> &Hash {
        match self {
            MerkleNode::Leaf(h) => h,
            MerkleNode::Internal { hash, .. } => hash,
        }
    }
}

/// A complete Merkle tree
#[derive(Debug, Clone)]
pub struct MerkleTree {
    root: MerkleNode,
    leaves: Vec<Hash>,
}

/// A Merkle proof for a single element
#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub leaf: Hash,
    pub path: Vec<(Hash, bool)>, // (sibling_hash, is_left)
}

impl MerkleTree {
    /// Build a Merkle tree from a BTreeSet of hashes
    pub fn from_btreeset(members: &BTreeSet<Hash>) -> Option<Self> {
        if members.is_empty() {
            return None;
        }

        let leaves: Vec<Hash> = members.iter().cloned().collect();
        let root = Self::build_tree(&leaves);

        Some(Self { root, leaves })
    }

    /// Build tree recursively from bottom up
    fn build_tree(hashes: &[Hash]) -> MerkleNode {
        if hashes.len() == 1 {
            return MerkleNode::Leaf(hashes[0]);
        }

        // Split in half (if odd, left side gets extra)
        let mid = (hashes.len() + 1) / 2;
        let (left_hashes, right_hashes) = hashes.split_at(mid);

        // Handle odd number of leaves by duplicating last
        let right_hashes = if right_hashes.is_empty() {
            left_hashes
        } else {
            right_hashes
        };

        let left = Box::new(Self::build_tree(left_hashes));
        let right = Box::new(Self::build_tree(right_hashes));

        let hash = hash_pair(left.hash(), right.hash());

        MerkleNode::Internal { hash, left, right }
    }

    /// Get the root hash
    pub fn root(&self) -> &Hash {
        self.root.hash()
    }

    /// Generate a proof for a leaf
    pub fn generate_proof(&self, leaf: &Hash) -> Option<MerkleProof> {
        let index = self.leaves.iter().position(|h| h == leaf)?;
        let path = self.collect_path(&self.root, index, self.leaves.len());
        Some(MerkleProof { leaf: *leaf, path })
    }

    /// Collect the path from leaf to root
    fn collect_path(&self, node: &MerkleNode, index: usize, size: usize) -> Vec<(Hash, bool)> {
        match node {
            MerkleNode::Leaf(_) => vec![],
            MerkleNode::Internal { left, right, .. } => {
                let left_size = (size + 1) / 2;

                if index < left_size {
                    // Go left
                    let mut path = self.collect_path(left, index, left_size);
                    // Sibling is on the right
                    path.push((*right.hash(), false)); // false = sibling is on right
                    path
                } else {
                    // Go right
                    let right_size = size - left_size;
                    let right_index = index - left_size;
                    let mut path = self.collect_path(right, right_index, right_size);
                    // Sibling is on the left
                    path.push((*left.hash(), true)); // true = sibling is on left
                    path
                }
            }
        }
    }

    /// Number of leaves
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }
}

/// Verify a Merkle proof against a root
pub fn verify_proof(proof: &MerkleProof, root: &Hash) -> bool {
    let mut current = proof.leaf;

    for (sibling, is_left) in &proof.path {
        current = if *is_left {
            hash_pair(sibling, &current)
        } else {
            hash_pair(&current, sibling)
        };
    }

    &current == root
}

/// Hash two children to create parent hash
fn hash_pair(left: &Hash, right: &Hash) -> Hash {
    use std::hash::Hasher;

    // Simple concatenation + hash
    let mut data = [0u8; 64];
    data[..32].copy_from_slice(left);
    data[32..].copy_from_slice(right);

    sha256(&data)
}

/// SHA-256 hash function (using standard library)
fn sha256(data: &[u8]) -> Hash {
    // Simple implementation using built-in hashing
    // In production, use ring or sha2 crate
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash as StdHash, Hasher};

    // This is NOT cryptographically secure - for spike testing only
    // Production should use: ring::digest::digest(&ring::digest::SHA256, data)
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let h1 = hasher.finish();

    // Run through hasher again for more spread
    hasher = DefaultHasher::new();
    h1.hash(&mut hasher);
    let h2 = hasher.finish();

    // Combine both hashes into 32 bytes
    let mut result = [0u8; 32];
    result[..8].copy_from_slice(&h1.to_le_bytes());
    result[8..16].copy_from_slice(&h2.to_le_bytes());
    result[16..24].copy_from_slice(&h1.to_be_bytes());
    result[24..32].copy_from_slice(&h2.to_be_bytes());

    result
}

/// Generate a test hash from an integer (for benchmarking)
pub fn test_hash(n: u64) -> Hash {
    let mut result = [0u8; 32];
    let bytes = n.to_le_bytes();
    result[..8].copy_from_slice(&bytes);

    // Fill rest with derived values
    for i in 1..4 {
        let derived = n.wrapping_mul(i as u64 + 1);
        result[i * 8..(i + 1) * 8].copy_from_slice(&derived.to_le_bytes());
    }

    sha256(&result)
}

/// Just calculate the root hash without building full tree
/// This is the most common operation
pub fn calculate_root(members: &BTreeSet<Hash>) -> Option<Hash> {
    if members.is_empty() {
        return None;
    }

    let leaves: Vec<Hash> = members.iter().cloned().collect();
    Some(calculate_root_from_leaves(&leaves))
}

/// Calculate root from a slice of leaves (recursive)
fn calculate_root_from_leaves(leaves: &[Hash]) -> Hash {
    if leaves.len() == 1 {
        return leaves[0];
    }

    // Pair up and hash
    let mut next_level: Vec<Hash> = Vec::with_capacity((leaves.len() + 1) / 2);

    let mut i = 0;
    while i < leaves.len() {
        if i + 1 < leaves.len() {
            next_level.push(hash_pair(&leaves[i], &leaves[i + 1]));
        } else {
            // Odd number: duplicate last
            next_level.push(hash_pair(&leaves[i], &leaves[i]));
        }
        i += 2;
    }

    calculate_root_from_leaves(&next_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_basic() {
        let mut members: BTreeSet<Hash> = BTreeSet::new();
        for i in 0..10 {
            members.insert(test_hash(i));
        }

        let tree = MerkleTree::from_btreeset(&members).unwrap();
        assert_eq!(tree.len(), 10);

        // Verify proof for each member
        for member in &members {
            let proof = tree.generate_proof(member).unwrap();
            assert!(verify_proof(&proof, tree.root()));
        }
    }

    #[test]
    fn test_root_only_calculation() {
        let mut members: BTreeSet<Hash> = BTreeSet::new();
        for i in 0..100 {
            members.insert(test_hash(i));
        }

        let tree = MerkleTree::from_btreeset(&members).unwrap();
        let root_only = calculate_root(&members).unwrap();

        assert_eq!(tree.root(), &root_only);
    }

    #[test]
    fn test_empty_set() {
        let members: BTreeSet<Hash> = BTreeSet::new();
        assert!(MerkleTree::from_btreeset(&members).is_none());
        assert!(calculate_root(&members).is_none());
    }

    #[test]
    fn test_single_member() {
        let mut members: BTreeSet<Hash> = BTreeSet::new();
        members.insert(test_hash(42));

        let tree = MerkleTree::from_btreeset(&members).unwrap();
        assert_eq!(tree.len(), 1);

        let proof = tree.generate_proof(&test_hash(42)).unwrap();
        assert!(verify_proof(&proof, tree.root()));
    }
}
