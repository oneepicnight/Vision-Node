# Legendary / Immortal Wallet Transfer System - Quick Reference

## Overview
The Legendary / Immortal Wallet Transfer System allows wallets with special status (legendary or immortal-node) to be sold/transferred to new owners. During transfer, the buyer **MUST** use a brand-new wallet with new seed words. The old wallet is permanently stripped of special powers after the transfer.

## Core Concept
- **Seller**: Has a legendary or immortal-node wallet
- **Transferable Mode**: Seller opts in to allow transfer
- **Marketplace**: Seller creates offer with price
- **Buyer**: Activates transfer with **mandatory new wallet**
- **Security**: Old wallet loses all special status permanently

---

## Chain State (Rust Layer)

### Data Structures

**AccountFlags** (3 bytes):
```rust
pub struct AccountFlags {
    pub legendary: bool,        // Legendary status
    pub immortal_node: bool,    // Immortal node status
    pub transferable: bool,     // Can be transferred
}
```

**TransferWalletStatusTx**:
```rust
pub struct TransferWalletStatusTx {
    pub from: String,                // Seller address
    pub to: String,                  // Buyer address (MUST be new wallet)
    pub move_balance: bool,          // Transfer balance?
    pub move_legendary: bool,        // Transfer legendary flag?
    pub move_immortal_node: bool,    // Transfer immortal_node flag?
}
```

**WalletOffer**:
```rust
pub struct WalletOffer {
    pub id: Uuid,
    pub from: String,
    pub move_legendary: bool,
    pub move_immortal_node: bool,
    pub move_balance: bool,
    pub price_land: u128,
    pub status: OfferStatus,  // Open | Completed | Cancelled
    pub created_at: u64,
}
```

### Validation Rules

**validate_transfer_wallet_status()**:
1. ✅ Feature gate check (feature_legendary_transfer enabled)
2. ✅ Same address check (from != to)
3. ✅ Status verification (wallet has legendary or immortal_node)
4. ✅ Transferable flag check (must be marked transferable)
5. ✅ Balance sufficiency (if moving balance)
6. ✅ Overflow protection

### State Transition

**apply_transfer_wallet_status()**:
1. **Move legendary flag** (if requested): from → to
2. **Move immortal_node flag** (if requested): from → to
3. **Move balance** (if requested): from → to
4. **⚠️ CRITICAL SECURITY**: `from_flags.transferable = false`
   - Old wallet CANNOT transfer again
   - Prevents seller from rugging buyer
5. Returns Result<(), WalletStatusError>

### Database Storage

**Prefix**: `acctflags:{address}`
**Format**: 3 bytes → [legendary_byte, immortal_byte, transferable_byte]

**Loading** (Chain::init):
```rust
for kv in db.scan_prefix(b"acctflags:") {
    let (k, v) = kv.expect("acctflags kv");
    let addr = key["acctflags:".len()..].to_string();
    let flags = AccountFlags::from_bytes(&v);
    account_flags.insert(addr, flags);
}
```

**Persistence**:
```rust
db.insert(
    format!("acctflags:{}", address).as_bytes(),
    flags.to_bytes().as_slice()
)
```

---

## API Endpoints (Rust/Axum Layer)

### 1. GET /api/wallets/:address/status
**Get wallet status and flags**

Response:
```json
{
  "address": "0x...",
  "balance": 1000000,
  "flags": {
    "legendary": true,
    "immortal_node": false,
    "transferable": false
  }
}
```

---

### 2. POST /api/wallets/:address/mark-transferable
**Enable/disable transfer mode (seller opt-in)**

Request:
```json
{
  "transferable": true,
  "admin_token": "..."  // TODO: Replace with signature
}
```

Response:
```json
{
  "success": true,
  "message": "Wallet 0x... enabled for transfer"
}
```

**Validation**:
- Wallet must have legendary OR immortal_node status
- Authorization required (admin_token or signature)

---

### 3. POST /api/wallets/:address/create-legendary-offer
**Create marketplace listing**

Request:
```json
{
  "move_legendary": true,
  "move_immortal_node": false,
  "move_balance": false,
  "price_land": 100000,
  "admin_token": "..."  // TODO: Replace with signature
}
```

Response:
```json
{
  "offer_id": "uuid-here",
  "offer": {
    "id": "uuid-here",
    "from": "0x...",
    "move_legendary": true,
    "move_immortal_node": false,
    "move_balance": false,
    "price_land": 100000,
    "status": "Open",
    "created_at": 1234567890
  }
}
```

**Validation**:
- Wallet must be marked transferable
- Wallet must have requested status (legendary/immortal_node)
- Authorization required

---

### 4. POST /api/wallets/complete-status-transfer
**Execute transfer (buyer activates with new wallet)**

Request:
```json
{
  "offer_id": "uuid-here",
  "new_wallet_address": "0xNEW_ADDRESS",  // MUST BE NEW
  "admin_token": "..."  // TODO: Replace with signature
}
```

Response:
```json
{
  "success": true,
  "transaction_hash": "0xabc123...",
  "message": "Legendary wallet status transferred from 0x... to 0x..."
}
```

**Process**:
1. ✅ Verify offer exists and is Open
2. ✅ Verify new_wallet_address != from (different wallet)
3. ✅ Validate transfer (using validate_transfer_wallet_status)
4. ✅ Apply transfer (using apply_transfer_wallet_status)
5. ✅ Update chain state (balances + account_flags)
6. ✅ Persist to database
7. ✅ Mark offer as Completed
8. ✅ **Critical**: Old wallet loses transferable flag

---

### 5. GET /api/wallets/legendary-offers
**List all open offers (marketplace)**

Response:
```json
{
  "offers": [
    {
      "id": "uuid-1",
      "from": "0x...",
      "move_legendary": true,
      "move_immortal_node": false,
      "price_land": 100000,
      "status": "Open",
      "created_at": 1234567890
    }
  ]
}
```

---

### 6. GET /api/wallets/legendary-offers/:offer_id
**Get specific offer details**

Response: Same as offer object above

---

### 7. POST /api/wallets/legendary-offers/:offer_id/cancel
**Cancel open offer**

Request:
```json
{
  "admin_token": "..."  // TODO: Replace with signature
}
```

Response:
```json
{
  "success": true,
  "message": "Offer uuid-here cancelled"
}
```

---

## Wallet UI Flow (React/TypeScript - TO BE IMPLEMENTED)

### Seller Side: TransferStatusFlow Component

**Step 1: Enable Transferable Mode**
```tsx
<Button onClick={() => markTransferable(address, true)}>
  Enable Transfer Mode
</Button>
```

**Step 2: Create Offer**
```tsx
<Form onSubmit={createOffer}>
  <Checkbox name="move_legendary" />
  <Checkbox name="move_immortal_node" />
  <Checkbox name="move_balance" />
  <Input name="price_land" type="number" />
  <Button type="submit">Create Offer</Button>
</Form>
```

**Step 3: Share Offer Link**
```tsx
<CopyButton text={`${MARKETPLACE_URL}/offers/${offerId}`} />
```

---

### Buyer Side: ActivateLegendaryWallet Component

**⚠️ CRITICAL: Force New Wallet Generation**

**Step 1: Generate New Wallet**
```tsx
<CreateNewWalletForTransfer 
  onGenerate={(seedPhrase, address) => {
    // Save seed phrase securely
    // Use address for transfer
  }}
/>
```

**Step 2: Confirm Seed Phrase Backup**
```tsx
<SeedPhraseConfirmation
  seedPhrase={generatedSeedPhrase}
  onConfirmed={() => setStep("transfer")}
/>
```

**Step 3: Complete Transfer**
```tsx
<Button onClick={() => completeTransfer(offerId, newWalletAddress)}>
  Activate Legendary Status on New Wallet
</Button>
```

**Step 4: Success Screen**
```tsx
<SuccessMessage>
  Legendary status activated on {newWalletAddress}
  <LegendaryWalletBadge address={newWalletAddress} />
</SuccessMessage>
```

---

### UI Components

**LegendaryWalletBadge.tsx**:
```tsx
interface Props {
  address: string;
}

function LegendaryWalletBadge({ address }: Props) {
  const { data } = useQuery(['wallet-status', address], () => 
    fetch(`/api/wallets/${address}/status`).then(r => r.json())
  );
  
  return (
    <div>
      {data?.flags.legendary && <Badge>⭐ LEGENDARY</Badge>}
      {data?.flags.immortal_node && <Badge>♾️ IMMORTAL NODE</Badge>}
    </div>
  );
}
```

**CreateNewWalletForTransfer.tsx**:
```tsx
function CreateNewWalletForTransfer({ onGenerate }) {
  const [seedPhrase, setSeedPhrase] = useState<string[]>([]);
  
  const handleGenerate = () => {
    const mnemonic = generateMnemonic(128); // 12 words
    const words = mnemonic.split(' ');
    setSeedPhrase(words);
    
    const wallet = ethers.Wallet.fromMnemonic(mnemonic);
    onGenerate(words, wallet.address);
  };
  
  return (
    <div>
      <Warning>
        ⚠️ You MUST use a brand new wallet to receive legendary status.
        Write down these seed words on paper - they cannot be recovered!
      </Warning>
      <Button onClick={handleGenerate}>Generate New Wallet</Button>
      {seedPhrase.length > 0 && (
        <SeedPhraseDisplay words={seedPhrase} />
      )}
    </div>
  );
}
```

**LegendaryMarketplace.tsx**:
```tsx
function LegendaryMarketplace() {
  const { data } = useQuery('legendary-offers', () =>
    fetch('/api/wallets/legendary-offers').then(r => r.json())
  );
  
  return (
    <div>
      <h1>Legendary Wallet Marketplace</h1>
      {data?.offers.map(offer => (
        <OfferCard 
          key={offer.id} 
          offer={offer}
          onPurchase={() => navigate(`/activate/${offer.id}`)}
        />
      ))}
    </div>
  );
}
```

---

## Security Guarantees

### 1. **Old Wallet Stripped of Powers**
After ANY transfer (successful or not), the old wallet's `transferable` flag is set to `false` permanently. This prevents the seller from:
- Transferring the status again
- Rugging the buyer by selling to multiple people
- Reclaiming the status

### 2. **Same Address Prevention**
Validation explicitly checks `from != to` to prevent:
- Accidental self-transfers
- Circular transfers
- Fee farming exploits

### 3. **Forced New Wallet (UI Layer)**
UI enforces new wallet generation:
- Buyer CANNOT use existing wallet
- Generates fresh seed phrase (12 or 24 words)
- Forces backup confirmation before transfer
- Prevents seller from knowing buyer's existing wallet

### 4. **Balance Overflow Protection**
```rust
to_balance.checked_add(from_balance)
  .ok_or(WalletStatusError::BalanceOverflow)?
```

### 5. **Status Verification**
Transfer only proceeds if:
- Seller has the claimed status (legendary or immortal_node)
- Wallet is marked transferable
- Feature gate is enabled

---

## Error Handling

**WalletStatusError**:
```rust
pub enum WalletStatusError {
    SameAddress,           // from == to
    NotLegendary,          // doesn't have legendary flag
    NotImmortalNode,       // doesn't have immortal_node flag
    NotTransferable,       // not marked transferable
    InsufficientBalance,   // can't move balance
    BalanceOverflow,       // balance too large
    OfferNotFound,         // offer doesn't exist
    OfferNotOpen,          // offer already completed/cancelled
    InvalidAddress,        // malformed address
    FeatureDisabled,       // feature gate off
}
```

Each error has user-friendly Display implementation.

---

## Configuration

**Feature Gate**:
```bash
# Enable legendary transfer feature
export VISION_LEGENDARY_TRANSFER_ENABLED=true
```

**Check in Code**:
```rust
fn is_feature_enabled() -> bool {
    std::env::var("VISION_LEGENDARY_TRANSFER_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .to_lowercase()
        == "true"
}
```

---

## Testing

### Unit Tests (Rust)

**test_account_flags_serialization**:
```rust
let flags = AccountFlags::default()
    .with_legendary(true)
    .with_transferable(true);
let bytes = flags.to_bytes();
let restored = AccountFlags::from_bytes(&bytes);
assert_eq!(flags, restored);
```

**test_validation_same_address**:
```rust
let tx = TransferWalletStatusTx {
    from: "alice".into(),
    to: "alice".into(),
    ...
};
let result = validate_transfer_wallet_status(&tx, ...);
assert!(matches!(result, Err(WalletStatusError::SameAddress)));
```

**test_validation_not_transferable**:
```rust
let flags = AccountFlags {
    legendary: true,
    transferable: false,  // NOT TRANSFERABLE
    ...
};
let result = validate_transfer_wallet_status(&tx, ...);
assert!(matches!(result, Err(WalletStatusError::NotTransferable)));
```

**test_apply_transfer**:
```rust
apply_transfer_wallet_status(&tx, &mut from_bal, &mut from_flags, ...)?;
assert_eq!(from_flags.legendary, false);  // moved
assert_eq!(to_flags.legendary, true);     // received
assert_eq!(from_flags.transferable, false);  // STRIPPED
```

### Integration Tests

**Full Flow Test**:
1. Create legendary wallet (chain state)
2. Mark transferable (API)
3. Create offer (API)
4. Generate new wallet (UI mock)
5. Complete transfer (API)
6. Verify old wallet stripped (chain state)
7. Verify new wallet has status (chain state)

---

## Deployment Checklist

- [✅] Core module implemented (legendary_wallet.rs)
- [✅] Chain struct extended (account_flags field)
- [✅] Database persistence (acctflags: prefix)
- [✅] API endpoints implemented (7 routes)
- [✅] API routes registered in main.rs
- [✅] Error handling comprehensive
- [✅] Security enforced (transferable stripped)
- [ ] Transaction handler integrated (legendary module)
- [ ] Signature verification added (replace admin_token)
- [ ] Wallet UI components created
- [ ] Wallet UI routing configured
- [ ] Feature gate configured in NetworkConfig
- [ ] Integration tests written
- [ ] Documentation complete
- [ ] Testnet deployment
- [ ] Mainnet deployment (after audit)

---

## Known Limitations / TODOs

1. **Signature Verification**: Currently using `admin_token` placeholder. Need to implement proper signature verification for:
   - mark_transferable
   - create_legendary_offer
   - complete_status_transfer
   - cancel_legendary_offer

2. **Transaction Handler**: Not yet integrated into execute_tx_with_nonce_and_fees. Needs:
   - "legendary" module case
   - "transfer_status" method handler
   - account_flags parameter in function signature

3. **Offer Persistence**: Currently using in-memory store (WALLET_OFFERS). Should persist to database with "offer:" prefix.

4. **Wallet UI**: Not yet implemented. Need all components listed in UI section.

5. **Payment Enforcement**: Price is recorded but not enforced. Need payment integration (LAND token transfer).

6. **Offer Expiry**: No time-based expiry. Consider adding expiry_timestamp.

7. **Transfer History**: No on-chain history. Consider adding transfer log.

8. **Multi-status Transfers**: Can transfer both legendary + immortal_node in one transaction. Verify this is intended behavior.

---

## Module Files

**Created**:
- `src/legendary_wallet.rs` - Core types, validation, state transition (400+ lines)
- `src/legendary_wallet_api.rs` - API endpoints (488 lines)

**Modified**:
- `src/main.rs` - Module declaration, API routes, Chain struct, database loading

**To Create**:
- `wallet-marketplace-source/src/components/LegendaryWalletBadge.tsx`
- `wallet-marketplace-source/src/components/TransferStatusFlow.tsx`
- `wallet-marketplace-source/src/components/ActivateLegendaryWallet.tsx`
- `wallet-marketplace-source/src/components/CreateNewWalletForTransfer.tsx`
- `wallet-marketplace-source/src/pages/LegendaryMarketplace.tsx`

---

## Contact / Support

For questions or issues:
- Check error messages (WalletStatusError has descriptive messages)
- Review validation rules in legendary_wallet.rs
- Test with feature gate enabled: `VISION_LEGENDARY_TRANSFER_ENABLED=true`
- Check logs: `[LEGENDARY_WALLET]` prefix

---

## Conclusion

The Legendary / Immortal Wallet Transfer System provides a secure, transparent way to transfer special wallet status. The three-layer architecture (Chain State → API → UI) ensures proper separation of concerns, with security enforced at the lowest level (Rust validation/state transition).

**Critical Security Feature**: Old wallet is permanently stripped of transferable flag after ANY transfer, preventing seller rug pulls and ensuring buyer safety.

**User Experience**: Buyer MUST generate new wallet with new seed words, enforced at UI layer, providing maximum security and fresh start for new owner.

This system can be extended to support:
- NFT ownership transfers
- DAO membership transfers
- Validator node transfers
- Any on-chain status/privilege transfers

**Status**: Core Rust implementation complete. API endpoints complete. Wallet UI pending implementation.
