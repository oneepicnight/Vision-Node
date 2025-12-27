# Multi-Currency UI - Quick Reference

## Components

### 1. MultiCurrencyBalances
**Location:** `wallet-marketplace-source/src/components/MultiCurrencyBalances.tsx`  
**Purpose:** Display LAND/BTC/BCH/DOGE balances with available/locked breakdown  
**Refresh:** Every 5 seconds  
**API:** `GET /api/wallet/balances?user_id={userId}`

### 2. DepositAddresses
**Location:** `wallet-marketplace-source/src/components/DepositAddresses.tsx`  
**Purpose:** Show deposit addresses with QR codes for BTC/BCH/DOGE  
**Features:** QR generation, copy address, download QR, network info  
**API:** `GET /api/wallet/deposit/{currency}?user_id={userId}`

### 3. Currency Pair Selector
**Location:** `wallet-marketplace-source/src/modules/exchange/Exchange.tsx`  
**Purpose:** Switch between BTC/BCH/DOGE/CASH trading pairs  
**Pairs:** BTC/LAND, BCH/LAND, DOGE/LAND, CASH/LAND

### 4. VaultStatusDashboard
**Location:** `wallet-marketplace-source/src/components/VaultStatusDashboard.tsx`  
**Purpose:** Admin dashboard showing vault balances and 50/30/20 split  
**Refresh:** Every 10 seconds  
**API:** `GET /api/vault/status`

## Routes

### Admin Vault
**Path:** `/admin/vault`  
**File:** `wallet-marketplace-source/src/routes/AdminVault.tsx`  
**Access:** Protected (requires wallet authentication)

## API Functions (src/lib/api.ts)

```typescript
// Get multi-currency balances
getWalletBalances(userId: string): Promise<MultiCurrencyBalance>

// Get deposit address with QR
getDepositAddress(userId: string, currency: string): Promise<DepositAddress>

// Get vault status (admin)
getVaultStatus(): Promise<VaultStatus>
```

## State (src/state/wallet.ts)

```typescript
// New state properties
multiCurrencyBalances: MultiCurrencyBalances
setMultiCurrencyBalances(balances: MultiCurrencyBalances)
```

## Integration

### Home Page
Added to `routes/Home.tsx`:
1. `<MultiCurrencyBalances />` - Shows all currency balances
2. `<DepositAddresses />` - Deposit interface with QR codes

### Exchange Page
Added to `modules/exchange/Exchange.tsx`:
1. Currency pair selector buttons at top
2. Automatically refreshes order book on pair change

### App Navigation
Added to `App.tsx`:
1. "Vault Admin" link in top nav
2. `/admin/vault` route

## Color Scheme

- **LAND:** Purple
- **BTC:** Orange  
- **BCH:** Green
- **DOGE:** Yellow
- **CASH:** Blue

## Testing Commands

```bash
# Development
cd wallet-marketplace-source
npm run dev

# Production build
npm run build

# Run tests
npm run test
```

## Mock Mode

Set `MOCK_CHAIN=true` in environment to test without backend:
- Demo balances returned
- Sample deposit addresses shown
- Mock vault data displayed

## Backend Endpoints Required

All already implemented in `src/main.rs`:
- `GET /api/wallet/balances`
- `GET /api/wallet/deposit/:currency`
- `GET /api/vault/status`

## Files Changed

### Created (4):
1. `components/MultiCurrencyBalances.tsx`
2. `components/DepositAddresses.tsx`
3. `components/VaultStatusDashboard.tsx`
4. `routes/AdminVault.tsx`

### Modified (5):
1. `lib/api.ts`
2. `state/wallet.ts`
3. `routes/Home.tsx`
4. `modules/exchange/Exchange.tsx`
5. `App.tsx`

## Build Success

```
✓ TypeScript compilation passed
✓ Vite build completed
✓ Output: dist/assets/index-*.js (908 KB → 300 KB gzipped)
```

## Next Steps

1. Start the backend node: `.\START-VISION-NODE.bat`
2. Start the wallet UI: `cd wallet-marketplace-source && npm run dev`
3. Navigate to `http://localhost:5173`
4. View multi-currency balances on Home page
5. Use deposit addresses to receive crypto
6. Switch trading pairs in Exchange
7. View vault status at `/admin/vault`
