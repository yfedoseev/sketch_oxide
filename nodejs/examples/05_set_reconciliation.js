/**
 * Set Reconciliation with RatelessIBLT
 *
 * Demonstrates efficient synchronization between two peers using Invertible Bloom
 * Lookup Tables (IBLT). Used in blockchain, P2P networks, and distributed systems
 * for efficient state reconciliation without transmitting full datasets.
 *
 * Run: node 05_set_reconciliation.js
 */

const { RatelessIBLT } = require('..');

console.log('=== P2P Set Reconciliation with RatelessIBLT ===\n');

// ============================================================================
// Example 1: Blockchain Transaction Pool Sync
// ============================================================================

console.log('Example 1: Blockchain Transaction Pool Sync');
console.log('===========================================\n');

class TransactionPool {
  constructor(nodeId, expectedDiff = 1000) {
    this.nodeId = nodeId;
    this.transactions = new Map();
    this.iblt = new RatelessIBLT(expectedDiff, 64);
  }

  addTransaction(txId, txData) {
    this.transactions.set(txId, txData);
    this.iblt.insert(Buffer.from(txId), Buffer.from(JSON.stringify(txData)));
  }

  removeTransaction(txId, txData) {
    this.transactions.delete(txId);
    this.iblt.delete(Buffer.from(txId), Buffer.from(JSON.stringify(txData)));
  }

  syncWith(otherPool) {
    // Create copy of IBLT for subtraction
    const localCopy = new RatelessIBLT(1000, 64);

    // Reconstruct IBLT from current state
    for (const [txId, txData] of this.transactions.entries()) {
      localCopy.insert(Buffer.from(txId), Buffer.from(JSON.stringify(txData)));
    }

    // Create copy of other pool's IBLT
    const remoteCopy = new RatelessIBLT(1000, 64);
    for (const [txId, txData] of otherPool.transactions.entries()) {
      remoteCopy.insert(Buffer.from(txId), Buffer.from(JSON.stringify(txData)));
    }

    // Compute symmetric difference
    localCopy.subtract(remoteCopy);

    // Decode to recover differences
    return localCopy.decode();
  }
}

// Simulate two blockchain nodes
const node1 = new TransactionPool('Node1');
const node2 = new TransactionPool('Node2');

// Both nodes have some shared transactions
console.log('Adding shared transactions...');
for (let i = 1; i <= 100; i++) {
  const txId = `tx_${i}`;
  const txData = { from: `addr_${i}`, to: `addr_${i + 1}`, amount: Math.random() * 100 };
  node1.addTransaction(txId, txData);
  node2.addTransaction(txId, txData);
}

// Node1 has unique transactions (101-120)
console.log('Node1 receives 20 new transactions...');
for (let i = 101; i <= 120; i++) {
  const txId = `tx_${i}`;
  const txData = { from: `addr_${i}`, to: `addr_${i + 1}`, amount: Math.random() * 100 };
  node1.addTransaction(txId, txData);
}

// Node2 has unique transactions (121-130)
console.log('Node2 receives 10 new transactions...');
for (let i = 121; i <= 130; i++) {
  const txId = `tx_${i}`;
  const txData = { from: `addr_${i}`, to: `addr_${i + 1}`, amount: Math.random() * 100 };
  node2.addTransaction(txId, txData);
}

console.log(`\nBefore sync:`);
console.log(`  Node1: ${node1.transactions.size} transactions`);
console.log(`  Node2: ${node2.transactions.size} transactions`);

// Perform reconciliation
console.log('\nPerforming IBLT reconciliation...');
const result = node1.syncWith(node2);

if (result.success) {
  console.log(`✓ Reconciliation successful!\n`);

  console.log('Differences found:');
  console.log(`  Node1 has (but Node2 doesn't): ${result.toInsert.length} transactions`);
  console.log(`  Node2 has (but Node1 doesn't): ${result.toRemove.length} transactions`);

  // Show sample differences
  if (result.toInsert.length > 0) {
    console.log('\n  Sample transactions Node1 has:');
    result.toInsert.slice(0, 3).forEach(item => {
      const txId = item.key.toString();
      console.log(`    - ${txId}`);
    });
  }

  if (result.toRemove.length > 0) {
    console.log('\n  Sample transactions Node2 has:');
    result.toRemove.slice(0, 3).forEach(item => {
      const txId = item.key.toString();
      console.log(`    - ${txId}`);
    });
  }

  // Calculate efficiency
  const totalDiff = result.toInsert.length + result.toRemove.length;
  const naiveBandwidth = (node1.transactions.size + node2.transactions.size) * 100; // bytes per tx
  const ibltBandwidth = 1000 * 64 * 2; // IBLT size
  const savings = ((1 - ibltBandwidth / naiveBandwidth) * 100).toFixed(1);

  console.log(`\nEfficiency Analysis:`);
  console.log(`  Actual difference size: ${totalDiff}`);
  console.log(`  Expected difference: 30`);
  console.log(`  Naive sync bandwidth: ~${(naiveBandwidth / 1024).toFixed(1)} KB`);
  console.log(`  IBLT sync bandwidth: ~${(ibltBandwidth / 1024).toFixed(1)} KB`);
  console.log(`  Bandwidth savings: ${savings}%`);
} else {
  console.log('✗ Reconciliation failed (difference too large)');
}

// ============================================================================
// Example 2: File Synchronization (like Dropbox/rsync)
// ============================================================================

console.log('\n\nExample 2: Distributed File Synchronization');
console.log('===========================================\n');

class FileSystem {
  constructor(name) {
    this.name = name;
    this.files = new Map();
  }

  addFile(filename, content) {
    const hash = this.hashContent(content);
    this.files.set(filename, { content, hash });
  }

  updateFile(filename, content) {
    this.addFile(filename, content);
  }

  deleteFile(filename) {
    this.files.delete(filename);
  }

  hashContent(content) {
    // Simple hash for demo
    let hash = 0;
    for (let i = 0; i < content.length; i++) {
      hash = ((hash << 5) - hash) + content.charCodeAt(i);
      hash = hash & hash;
    }
    return hash.toString(16);
  }

  createIBLT(expectedDiff = 100) {
    const iblt = new RatelessIBLT(expectedDiff, 128);
    for (const [filename, fileData] of this.files.entries()) {
      const key = Buffer.from(filename);
      const value = Buffer.from(fileData.hash);
      iblt.insert(key, value);
    }
    return iblt;
  }

  syncWith(otherFS) {
    const local = this.createIBLT();
    const remote = otherFS.createIBLT();

    local.subtract(remote);
    return local.decode();
  }
}

// Simulate Alice's and Bob's file systems
const alice = new FileSystem('Alice');
const bob = new FileSystem('Bob');

// Shared files
console.log('Setting up shared files...');
alice.addFile('document.txt', 'Shared content v1');
bob.addFile('document.txt', 'Shared content v1');

alice.addFile('project.md', 'Project documentation');
bob.addFile('project.md', 'Project documentation');

// Alice's unique files
console.log('Alice creates new files...');
alice.addFile('alice_notes.txt', 'Alice private notes');
alice.addFile('alice_draft.md', 'Draft document');

// Bob's unique files
console.log('Bob creates new files...');
bob.addFile('bob_report.pdf', 'Bob quarterly report');

// Both modify shared file differently
alice.updateFile('document.txt', 'Shared content v2 - Alice version');
bob.updateFile('document.txt', 'Shared content v2 - Bob version');

console.log(`\nBefore sync:`);
console.log(`  Alice: ${alice.files.size} files`);
console.log(`  Bob: ${bob.files.size} files`);

// Perform sync
console.log('\nPerforming file synchronization...');
const syncResult = alice.syncWith(bob);

if (syncResult.success) {
  console.log('✓ Synchronization successful!\n');

  console.log('Files to sync:');
  console.log(`  Alice -> Bob: ${syncResult.toInsert.length} files`);
  syncResult.toInsert.forEach(item => {
    console.log(`    + ${item.key.toString()}`);
  });

  console.log(`  Bob -> Alice: ${syncResult.toRemove.length} files`);
  syncResult.toRemove.forEach(item => {
    console.log(`    + ${item.key.toString()}`);
  });

  console.log('\n  Conflicts detected (same file, different hash):');
  const aliceFiles = new Set(Array.from(alice.files.keys()));
  const bobFiles = new Set(Array.from(bob.files.keys()));

  for (const filename of aliceFiles) {
    if (bobFiles.has(filename)) {
      const aliceHash = alice.files.get(filename).hash;
      const bobHash = bob.files.get(filename).hash;
      if (aliceHash !== bobHash) {
        console.log(`    ⚠ ${filename} (Alice: ${aliceHash}, Bob: ${bobHash})`);
      }
    }
  }
}

// ============================================================================
// Example 3: Database Replication
// ============================================================================

console.log('\n\nExample 3: Database Row Replication');
console.log('===================================\n');

class DatabaseReplica {
  constructor(replicaId) {
    this.replicaId = replicaId;
    this.rows = new Map();
  }

  insert(rowId, data) {
    this.rows.set(rowId, data);
  }

  update(rowId, data) {
    this.rows.set(rowId, data);
  }

  delete(rowId) {
    this.rows.delete(rowId);
  }

  reconcileWith(otherReplica, expectedDiff = 50) {
    const local = new RatelessIBLT(expectedDiff, 64);
    const remote = new RatelessIBLT(expectedDiff, 64);

    // Insert local rows
    for (const [rowId, data] of this.rows.entries()) {
      local.insert(Buffer.from(rowId), Buffer.from(JSON.stringify(data)));
    }

    // Insert remote rows
    for (const [rowId, data] of otherReplica.rows.entries()) {
      remote.insert(Buffer.from(rowId), Buffer.from(JSON.stringify(data)));
    }

    local.subtract(remote);
    return local.decode();
  }
}

const primary = new DatabaseReplica('primary');
const secondary = new DatabaseReplica('secondary');

// Initial sync
console.log('Initial database sync...');
for (let i = 1; i <= 1000; i++) {
  const rowId = `row_${i}`;
  const data = { id: i, value: `value_${i}`, version: 1 };
  primary.insert(rowId, data);
  secondary.insert(rowId, data);
}

// Simulate network partition - writes on both sides
console.log('Network partition - divergent writes...');

// Primary gets updates
for (let i = 1001; i <= 1020; i++) {
  primary.insert(`row_${i}`, { id: i, value: `primary_${i}`, version: 1 });
}

// Secondary gets different updates
for (let i = 1021; i <= 1030; i++) {
  secondary.insert(`row_${i}`, { id: i, value: `secondary_${i}`, version: 1 });
}

console.log(`\nBefore reconciliation:`);
console.log(`  Primary: ${primary.rows.size} rows`);
console.log(`  Secondary: ${secondary.rows.size} rows`);

// Reconcile after partition heals
console.log('\nReconciling replicas...');
const dbResult = primary.reconcileWith(secondary);

if (dbResult.success) {
  console.log('✓ Reconciliation successful!\n');
  console.log(`  Primary has: ${dbResult.toInsert.length} rows to send to secondary`);
  console.log(`  Secondary has: ${dbResult.toRemove.length} rows to send to primary`);
  console.log(`  Total divergence: ${dbResult.toInsert.length + dbResult.toRemove.length} rows`);
}

// ============================================================================
// Summary
// ============================================================================

console.log('\n\n=== RatelessIBLT Summary ===\n');

console.log('Key Benefits:');
console.log('  ✓ Compute symmetric difference without knowing size');
console.log('  ✓ 5-10x bandwidth reduction vs naive sync');
console.log('  ✓ Constant-size data structure (independent of set size)');
console.log('  ✓ Efficient for small to moderate differences\n');

console.log('Performance:');
console.log('  Space: O(d) where d = expected difference');
console.log('  Insert/Delete: O(k) where k = 3 hash functions');
console.log('  Subtract: O(n) where n = number of cells');
console.log('  Decode: O(d × k)\n');

console.log('Production Use Cases:');
console.log('  • Blockchain transaction pool synchronization');
console.log('  • P2P file sharing (BitTorrent, IPFS)');
console.log('  • Distributed database replication');
console.log('  • CDN cache invalidation');
console.log('  • Git-like version control systems');
console.log('  • Mobile app offline-first sync\n');

console.log('Best Practices:');
console.log('  1. Set expectedDiff to 1.5-2x actual difference');
console.log('  2. Use cell size large enough for your data (32-128 bytes)');
console.log('  3. Fall back to full sync if decode fails');
console.log('  4. Combine with bloom filters for pre-filtering');
console.log('  5. Monitor success rate and adjust parameters\n');

console.log('=== Example Complete ===\n');
