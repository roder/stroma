// Q14: Chunk Communication Protocol Spike
//
// Tests how bots transmit chunks to holders. Evaluates:
// - Option A: Freenet contract-based distribution
// - Option C: Hybrid approach (P2P + attestation)
// - Cost and latency comparison

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sha2::{Digest, Sha256};

/// Simulated chunk data
type ChunkData = Vec<u8>;
type BotId = [u8; 32];
type ContractHash = [u8; 32];

/// Distribution attestation proving chunk was sent
#[derive(Clone, Debug, PartialEq)]
pub struct DistributionAttestation {
    sender: BotId,
    receiver: BotId,
    chunk_hash: [u8; 32],
    timestamp: u64,
}

impl DistributionAttestation {
    pub fn new(sender: BotId, receiver: BotId, chunk: &ChunkData) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(chunk);
        let chunk_hash: [u8; 32] = hasher.finalize().into();

        Self {
            sender,
            receiver,
            chunk_hash,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn is_valid(&self) -> bool {
        // Verify attestation is recent (within 1 hour)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now - self.timestamp < 3600
    }

    pub fn verify_chunk(&self, chunk: &ChunkData) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(chunk);
        let computed_hash: [u8; 32] = hasher.finalize().into();
        computed_hash == self.chunk_hash
    }
}

/// Simulated Freenet contract storage
pub struct FreenetContractStore {
    /// Maps contract hash to stored data
    contracts: Arc<std::sync::Mutex<HashMap<ContractHash, Vec<ChunkData>>>>,
    /// Simulated write latency (milliseconds)
    write_latency_ms: u64,
    /// Simulated write cost (arbitrary units)
    write_cost: u64,
}

impl FreenetContractStore {
    pub fn new(write_latency_ms: u64, write_cost: u64) -> Self {
        Self {
            contracts: Arc::new(std::sync::Mutex::new(HashMap::new())),
            write_latency_ms,
            write_cost,
        }
    }

    pub async fn write_chunk(&self, contract: ContractHash, chunk: ChunkData) -> Result<(), String> {
        // Simulate network latency
        tokio::time::sleep(Duration::from_millis(self.write_latency_ms)).await;

        let mut contracts = self.contracts.lock().unwrap();
        contracts.entry(contract).or_insert_with(Vec::new).push(chunk);

        Ok(())
    }

    pub async fn read_chunks(&self, contract: ContractHash) -> Vec<ChunkData> {
        let contracts = self.contracts.lock().unwrap();
        contracts.get(&contract).cloned().unwrap_or_default()
    }

    pub fn get_write_cost(&self) -> u64 {
        self.write_cost
    }
}

/// Simulated P2P network layer
pub struct P2PNetwork {
    /// Maps bot ID to their stored chunks
    peers: Arc<std::sync::Mutex<HashMap<BotId, Vec<ChunkData>>>>,
    /// Simulated transfer latency (milliseconds)
    transfer_latency_ms: u64,
    /// Simulated transfer cost (arbitrary units)
    transfer_cost: u64,
}

impl P2PNetwork {
    pub fn new(transfer_latency_ms: u64, transfer_cost: u64) -> Self {
        Self {
            peers: Arc::new(std::sync::Mutex::new(HashMap::new())),
            transfer_latency_ms,
            transfer_cost,
        }
    }

    pub async fn send_chunk(&self, to: BotId, chunk: ChunkData) -> Result<(), String> {
        // Simulate network latency (faster than Freenet contracts)
        tokio::time::sleep(Duration::from_millis(self.transfer_latency_ms)).await;

        let mut peers = self.peers.lock().unwrap();
        peers.entry(to).or_insert_with(Vec::new).push(chunk);

        Ok(())
    }

    pub async fn get_chunks(&self, bot_id: BotId) -> Vec<ChunkData> {
        let peers = self.peers.lock().unwrap();
        peers.get(&bot_id).cloned().unwrap_or_default()
    }

    pub fn get_transfer_cost(&self) -> u64 {
        self.transfer_cost
    }
}

/// Bot that can distribute chunks using different protocols
pub struct ChunkDistributor {
    bot_id: BotId,
    contract_store: Arc<FreenetContractStore>,
    p2p_network: Arc<P2PNetwork>,
}

impl ChunkDistributor {
    pub fn new(
        bot_id: BotId,
        contract_store: Arc<FreenetContractStore>,
        p2p_network: Arc<P2PNetwork>,
    ) -> Self {
        Self {
            bot_id,
            contract_store,
            p2p_network,
        }
    }

    /// Option A: Distribute chunk via Freenet contract
    pub async fn distribute_via_contract(
        &self,
        receiver: BotId,
        chunk: ChunkData,
    ) -> Result<DistributionAttestation, String> {
        // Write chunk to receiver's storage contract
        self.contract_store.write_chunk(receiver, chunk.clone()).await?;

        // Create attestation
        let attestation = DistributionAttestation::new(self.bot_id, receiver, &chunk);

        Ok(attestation)
    }

    /// Option C: Hybrid distribution (P2P + attestation)
    pub async fn distribute_hybrid(
        &self,
        receiver: BotId,
        chunk: ChunkData,
    ) -> Result<DistributionAttestation, String> {
        // Send chunk via P2P (fast, low cost)
        self.p2p_network.send_chunk(receiver, chunk.clone()).await?;

        // Create attestation (small, can be batched)
        let attestation = DistributionAttestation::new(self.bot_id, receiver, &chunk);

        // In real implementation, would write attestation to contract
        // For simulation, we just return it

        Ok(attestation)
    }

    /// Distribute chunks to multiple holders and measure performance
    pub async fn distribute_state_update(
        &self,
        holders: Vec<BotId>,
        chunks: Vec<ChunkData>,
        use_hybrid: bool,
    ) -> (Duration, u64) {
        let start = Instant::now();
        let mut total_cost = 0u64;

        // Distribute each chunk to its holders
        for chunk in chunks {
            for holder in &holders {
                if use_hybrid {
                    self.distribute_hybrid(*holder, chunk.clone()).await.unwrap();
                    total_cost += self.p2p_network.get_transfer_cost();
                } else {
                    self.distribute_via_contract(*holder, chunk.clone()).await.unwrap();
                    total_cost += self.contract_store.get_write_cost();
                }
            }
        }

        (start.elapsed(), total_cost)
    }
}

/// Test utilities
fn create_test_chunk(size: usize) -> ChunkData {
    vec![0xAB; size]
}

fn create_test_bot_id(id: u8) -> BotId {
    let mut bot_id = [0u8; 32];
    bot_id[0] = id;
    bot_id
}

// ==================== TESTS ====================

#[tokio::test]
async fn test_contract_distribution() {
    println!("\n=== Test 1: Contract-Based Distribution ===");

    // Setup: Simulate Freenet with 100ms write latency, cost 10 units
    let contract_store = Arc::new(FreenetContractStore::new(100, 10));
    let p2p_network = Arc::new(P2PNetwork::new(20, 1));

    let bot_a = create_test_bot_id(1);
    let bot_b = create_test_bot_id(2);

    let distributor = ChunkDistributor::new(bot_a, contract_store.clone(), p2p_network);

    // Test: Distribute chunk via contract
    let chunk = create_test_chunk(64 * 1024); // 64KB chunk
    let start = Instant::now();
    let attestation = distributor
        .distribute_via_contract(bot_b, chunk.clone())
        .await
        .unwrap();
    let latency = start.elapsed();

    println!("Latency: {:?}", latency);
    println!("Cost: {} units", contract_store.get_write_cost());

    // Verify attestation
    assert!(attestation.is_valid());
    assert!(attestation.verify_chunk(&chunk));

    // Verify chunk was stored
    let stored_chunks = contract_store.read_chunks(bot_b).await;
    assert_eq!(stored_chunks.len(), 1);
    assert_eq!(stored_chunks[0], chunk);

    // Success criteria: < 5s (easily met with 100ms latency)
    assert!(latency < Duration::from_secs(5));

    println!("✅ Contract distribution successful");
}

#[tokio::test]
async fn test_hybrid_distribution() {
    println!("\n=== Test 2: Hybrid Distribution (P2P + Attestation) ===");

    // Setup: P2P is faster (20ms) and cheaper (cost 1) than contracts
    let contract_store = Arc::new(FreenetContractStore::new(100, 10));
    let p2p_network = Arc::new(P2PNetwork::new(20, 1));

    let bot_a = create_test_bot_id(1);
    let bot_b = create_test_bot_id(2);

    let distributor = ChunkDistributor::new(bot_a, contract_store, p2p_network.clone());

    // Test: Distribute chunk via hybrid approach
    let chunk = create_test_chunk(64 * 1024); // 64KB chunk
    let start = Instant::now();
    let attestation = distributor
        .distribute_hybrid(bot_b, chunk.clone())
        .await
        .unwrap();
    let latency = start.elapsed();

    println!("Latency: {:?}", latency);
    println!("Cost: {} units", p2p_network.get_transfer_cost());

    // Verify attestation
    assert!(attestation.is_valid());
    assert!(attestation.verify_chunk(&chunk));

    // Verify chunk was sent via P2P
    let stored_chunks = p2p_network.get_chunks(bot_b).await;
    assert_eq!(stored_chunks.len(), 1);
    assert_eq!(stored_chunks[0], chunk);

    // Success criteria: < 2s (faster than contract-only)
    assert!(latency < Duration::from_secs(2));

    println!("✅ Hybrid distribution successful");
}

#[tokio::test]
async fn test_full_state_distribution_comparison() {
    println!("\n=== Test 3: Full State Distribution Comparison ===");

    // Setup: Typical state update scenario
    // 512KB state = 8 chunks × 64KB, 2 replicas each = 16 distributions
    let contract_store = Arc::new(FreenetContractStore::new(100, 10));
    let p2p_network = Arc::new(P2PNetwork::new(20, 1));

    let bot_a = create_test_bot_id(1);
    let holders = vec![
        create_test_bot_id(2),
        create_test_bot_id(3),
    ];

    let chunks: Vec<ChunkData> = (0..8)
        .map(|_| create_test_chunk(64 * 1024))
        .collect();

    let distributor = ChunkDistributor::new(bot_a, contract_store.clone(), p2p_network.clone());

    // Test Option A: Contract-based
    println!("\nOption A: Contract-based distribution");
    let (latency_contract, cost_contract) = distributor
        .distribute_state_update(holders.clone(), chunks.clone(), false)
        .await;

    println!("Total latency: {:?}", latency_contract);
    println!("Total cost: {} units", cost_contract);
    println!(
        "Per-chunk latency: {:?}",
        latency_contract / (chunks.len() as u32 * holders.len() as u32)
    );

    // Test Option C: Hybrid
    println!("\nOption C: Hybrid distribution");
    let (latency_hybrid, cost_hybrid) = distributor
        .distribute_state_update(holders.clone(), chunks.clone(), true)
        .await;

    println!("Total latency: {:?}", latency_hybrid);
    println!("Total cost: {} units", cost_hybrid);
    println!(
        "Per-chunk latency: {:?}",
        latency_hybrid / (chunks.len() as u32 * holders.len() as u32)
    );

    // Comparison
    println!("\n--- Comparison ---");
    println!("Latency improvement: {:.1}x faster",
        latency_contract.as_millis() as f64 / latency_hybrid.as_millis() as f64);
    println!("Cost reduction: {:.1}x cheaper",
        cost_contract as f64 / cost_hybrid as f64);

    // Success criteria: Distribution completes in < 10s
    assert!(latency_contract < Duration::from_secs(10));
    assert!(latency_hybrid < Duration::from_secs(10));

    // Hybrid should be faster and cheaper
    assert!(latency_hybrid < latency_contract);
    assert!(cost_hybrid < cost_contract);

    println!("✅ Full state distribution comparison complete");
}

#[tokio::test]
async fn test_parallel_distribution() {
    println!("\n=== Test 4: Parallel Distribution Performance ===");

    // Setup: Test parallel distribution to multiple holders
    let contract_store = Arc::new(FreenetContractStore::new(100, 10));
    let p2p_network = Arc::new(P2PNetwork::new(20, 1));

    let bot_a = create_test_bot_id(1);
    let holders: Vec<BotId> = (2..18).map(create_test_bot_id).collect(); // 16 holders

    let distributor = ChunkDistributor::new(bot_a, contract_store, p2p_network);

    let chunk = create_test_chunk(64 * 1024);

    // Test: Parallel distribution (in real implementation, would use tokio::spawn)
    let start = Instant::now();
    for holder in &holders {
        // Simulating parallel with sequential for simplicity
        distributor
            .distribute_hybrid(*holder, chunk.clone())
            .await
            .unwrap();
    }
    let latency = start.elapsed();

    println!("Distributed to {} holders", holders.len());
    println!("Total latency: {:?}", latency);
    println!("Per-holder latency: {:?}", latency / holders.len() as u32);

    // With real parallel execution, should approach single-request latency
    // For sequential simulation, should still be < 10s
    assert!(latency < Duration::from_secs(10));

    println!("✅ Parallel distribution test complete");
}

#[tokio::test]
async fn test_attestation_verification() {
    println!("\n=== Test 5: Attestation Verification ===");

    let bot_a = create_test_bot_id(1);
    let bot_b = create_test_bot_id(2);
    let chunk = create_test_chunk(64 * 1024);

    // Create attestation
    let attestation = DistributionAttestation::new(bot_a, bot_b, &chunk);

    // Test: Valid attestation
    assert!(attestation.is_valid());
    assert!(attestation.verify_chunk(&chunk));

    // Test: Tampered chunk detection
    let tampered_chunk = create_test_chunk(64 * 1024 + 1);
    assert!(!attestation.verify_chunk(&tampered_chunk));

    println!("✅ Attestation verification works correctly");
}

#[tokio::test]
async fn test_scalability() {
    println!("\n=== Test 6: Scalability Analysis ===");

    // Test different network sizes and state sizes
    let scenarios: Vec<(&str, usize, usize)> = vec![
        ("Small: 50KB, 10 holders", 50 * 1024, 10),
        ("Medium: 512KB, 16 holders", 512 * 1024, 16),
        ("Large: 2MB, 32 holders", 2 * 1024 * 1024, 32),
    ];

    for (name, state_size, num_holders) in scenarios {
        println!("\n{}", name);

        let contract_store = Arc::new(FreenetContractStore::new(100, 10));
        let p2p_network = Arc::new(P2PNetwork::new(20, 1));

        let bot_a = create_test_bot_id(1);
        let holders: Vec<BotId> = (2..2 + num_holders).map(|id| create_test_bot_id(id as u8)).collect();

        let distributor = ChunkDistributor::new(bot_a, contract_store, p2p_network);

        // Calculate chunks (64KB each, minimum 1)
        let num_chunks = (state_size / (64 * 1024)).max(1);
        let chunks: Vec<ChunkData> = (0..num_chunks)
            .map(|_| create_test_chunk(64 * 1024))
            .collect();

        let (latency, cost) = distributor
            .distribute_state_update(holders, chunks, true)
            .await;

        let total_distributions = num_chunks * num_holders;

        println!("  Chunks: {}", num_chunks);
        println!("  Latency: {:?}", latency);
        println!("  Cost: {} units", cost);
        if total_distributions > 0 {
            println!(
                "  Per-distribution: {:?}",
                latency / (total_distributions as u32)
            );
        }
    }

    println!("\n✅ Scalability analysis complete");
}

fn main() {
    println!("Q14: Chunk Communication Protocol - Spike Tests");
    println!("Run with: cargo test --bin spike-q14");
}
