/**
 * Vision Node Wallet - Client-Side Signing Example (JavaScript/Node.js)
 * 
 * This example demonstrates how to:
 * 1. Generate Ed25519 keypairs
 * 2. Construct canonical transfer messages
 * 3. Sign transfers with Ed25519 (MANDATORY ON MAINNET - do not send private keys to servers)
 * 4. Submit signed transfers to the Vision node
 * 
 * Dependencies:
 *   npm install @noble/ed25519 node-fetch
 */

const ed25519 = require('@noble/ed25519');
const fetch = require('node-fetch');

// Configuration
const VISION_NODE_URL = 'http://127.0.0.1:7070';

/**
 * Convert u128 to 16-byte little-endian buffer
 */
function uint128ToLE(value) {
    const buf = Buffer.alloc(16);
    const bigValue = BigInt(value);
    
    // Write lower 64 bits
    const lower = bigValue & 0xFFFFFFFFFFFFFFFFn;
    buf.writeBigUInt64LE(lower, 0);
    
    // Write upper 64 bits
    const upper = bigValue >> 64n;
    buf.writeBigUInt64LE(upper, 8);
    
    return buf;
}

/**
 * Convert u64 to 8-byte little-endian buffer
 */
function uint64ToLE(value) {
    const buf = Buffer.alloc(8);
    buf.writeBigUInt64LE(BigInt(value), 0);
    return buf;
}

/**
 * Construct canonical message for signing a transfer
 * 
 * Format:
 *   from (32 bytes raw) +
 *   to (32 bytes raw) +
 *   amount (16 bytes LE u128) +
 *   fee (16 bytes LE u128) +
 *   nonce (8 bytes LE u64) +
 *   memo (optional UTF-8)
 */
function constructTransferMessage(transfer) {
    const parts = [
        Buffer.from(transfer.from, 'hex'),           // 32 bytes
        Buffer.from(transfer.to, 'hex'),             // 32 bytes
        uint128ToLE(transfer.amount),                // 16 bytes
        uint128ToLE(transfer.fee || 0),              // 16 bytes
        uint64ToLE(transfer.nonce),                  // 8 bytes
        Buffer.from(transfer.memo || '', 'utf8')     // optional
    ];
    
    return Buffer.concat(parts);
}

/**
 * Generate a new Ed25519 keypair
 * 
 * Returns: { privateKey, publicKey, address }
 */
async function generateKeypair() {
    const privateKey = ed25519.utils.randomPrivateKey();
    const publicKey = await ed25519.getPublicKey(privateKey);
    const address = Buffer.from(publicKey).toString('hex');
    
    return {
        privateKey,
        publicKey,
        address
    };
}

/**
 * Query the balance for an address
 */
async function getBalance(address) {
    const response = await fetch(`${VISION_NODE_URL}/wallet/${address}/balance`);
    const data = await response.json();
    return data;
}

/**
 * Query the current nonce for an address
 */
async function getNonce(address) {
    const response = await fetch(`${VISION_NODE_URL}/wallet/${address}/nonce`);
    const data = await response.json();
    return data.nonce;
}

/**
 * Sign and submit a transfer
 */
async function signAndSubmitTransfer(privateKey, publicKey, transfer) {
    // 1. Construct canonical message
    const message = constructTransferMessage(transfer);
    
    // 2. Sign with Ed25519
    const signature = await ed25519.sign(message, privateKey);
    
    // 3. Prepare request
    const request = {
        from: transfer.from,
        to: transfer.to,
        amount: transfer.amount.toString(),
        fee: (transfer.fee || 0).toString(),
        memo: transfer.memo || null,
        signature: Buffer.from(signature).toString('hex'),
        nonce: transfer.nonce,
        public_key: Buffer.from(publicKey).toString('hex')
    };
    
    // 4. Submit to node
    const response = await fetch(`${VISION_NODE_URL}/wallet/transfer`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request)
    });
    
    const result = await response.json();
    
    if (!response.ok) {
        throw new Error(`Transfer failed: ${JSON.stringify(result)}`);
    }
    
    return result;
}

// ===== USAGE EXAMPLES =====

async function example1_generateKeypair() {
    console.log('=== Example 1: Generate Keypair ===');
    const keypair = await generateKeypair();
    
    console.log('Private Key:', Buffer.from(keypair.privateKey).toString('hex'));
    console.log('Public Key: ', Buffer.from(keypair.publicKey).toString('hex'));
    console.log('Address:    ', keypair.address);
    console.log();
    
    return keypair;
}

async function example2_queryBalance(address) {
    console.log('=== Example 2: Query Balance ===');
    const balance = await getBalance(address);
    
    console.log('Address:', balance.address);
    console.log('Balance:', balance.balance);
    console.log();
    
    return balance;
}

async function example3_queryNonce(address) {
    console.log('=== Example 3: Query Nonce ===');
    const nonce = await getNonce(address);
    
    console.log('Address:', address);
    console.log('Nonce:  ', nonce);
    console.log();
    
    return nonce;
}

async function example4_signedTransfer() {
    console.log('=== Example 4: Signed Transfer ===');
    
    // Generate sender and recipient keypairs
    const sender = await generateKeypair();
    const recipient = await generateKeypair();
    
    console.log('Sender:   ', sender.address);
    console.log('Recipient:', recipient.address);
    
    // NOTE: In production, you would fund the sender address first
    // For this example, assume the sender has balance
    
    // Get current nonce
    const currentNonce = await getNonce(sender.address);
    console.log('Current Nonce:', currentNonce);
    
    // Prepare transfer
    const transfer = {
        from: sender.address,
        to: recipient.address,
        amount: 5000,
        fee: 50,
        memo: 'Test transfer',
        nonce: currentNonce + 1
    };
    
    console.log('\nTransfer Details:');
    console.log('  Amount:', transfer.amount);
    console.log('  Fee:   ', transfer.fee);
    console.log('  Nonce: ', transfer.nonce);
    console.log('  Memo:  ', transfer.memo);
    
    // Sign and submit
    try {
        const result = await signAndSubmitTransfer(
            sender.privateKey,
            sender.publicKey,
            transfer
        );
        
        console.log('\n✓ Transfer successful!');
        console.log('Result:', result);
    } catch (error) {
        console.error('\n✗ Transfer failed:', error.message);
    }
    
    console.log();
}

async function example5_messageConstruction() {
    console.log('=== Example 5: Message Construction ===');
    
    const transfer = {
        from: '0000000000000000000000000000000000000000000000000000000000000001',
        to: '0000000000000000000000000000000000000000000000000000000000000002',
        amount: 5000,
        fee: 50,
        nonce: 1,
        memo: 'test'
    };
    
    const message = constructTransferMessage(transfer);
    
    console.log('Transfer:');
    console.log('  from:  ', transfer.from);
    console.log('  to:    ', transfer.to);
    console.log('  amount:', transfer.amount);
    console.log('  fee:   ', transfer.fee);
    console.log('  nonce: ', transfer.nonce);
    console.log('  memo:  ', transfer.memo);
    console.log('\nCanonical Message (hex):');
    console.log(message.toString('hex'));
    console.log('\nMessage Length:', message.length, 'bytes');
    console.log('Expected: 32 (from) + 32 (to) + 16 (amount) + 16 (fee) + 8 (nonce) + 4 (memo) = 108');
    console.log();
}

async function example6_multipleTransfers() {
    console.log('=== Example 6: Multiple Sequential Transfers ===');
    
    const sender = await generateKeypair();
    const recipient = await generateKeypair();
    
    console.log('Sender:   ', sender.address);
    console.log('Recipient:', recipient.address);
    
    // Get initial nonce
    let nonce = await getNonce(sender.address);
    console.log('Initial Nonce:', nonce);
    
    // Submit 3 transfers sequentially
    for (let i = 1; i <= 3; i++) {
        console.log(`\nTransfer ${i}:`);
        
        const transfer = {
            from: sender.address,
            to: recipient.address,
            amount: 1000 * i,
            fee: 10,
            memo: `Transfer #${i}`,
            nonce: nonce + 1
        };
        
        try {
            const result = await signAndSubmitTransfer(
                sender.privateKey,
                sender.publicKey,
                transfer
            );
            
            console.log(`  ✓ Success (nonce ${transfer.nonce})`);
            nonce = transfer.nonce; // Update for next transfer
        } catch (error) {
            console.log(`  ✗ Failed: ${error.message}`);
            break; // Stop on error
        }
    }
    
    console.log();
}

// ===== RUN EXAMPLES =====

async function main() {
    try {
        // Run examples
        await example1_generateKeypair();
        
        // Example 2-4 require a running Vision node
        // Uncomment to test with real node:
        
        // const keypair = await example1_generateKeypair();
        // await example2_queryBalance(keypair.address);
        // await example3_queryNonce(keypair.address);
        // await example4_signedTransfer();
        
        await example5_messageConstruction();
        
        // await example6_multipleTransfers();
        
        console.log('✓ All examples completed');
    } catch (error) {
        console.error('Error:', error);
        process.exit(1);
    }
}

// Export functions for use as library
module.exports = {
    generateKeypair,
    getBalance,
    getNonce,
    signAndSubmitTransfer,
    constructTransferMessage,
    uint128ToLE,
    uint64ToLE
};

// Run examples if called directly
if (require.main === module) {
    main();
}
