# Vision Governance System - LAND Democracy

## Overview

The Vision Governance System is a comprehensive on-chain democracy where LAND deed holders and founders can propose and vote on changes to the Vision Node protocol.

## Key Features

- **10,000 LAND Proposal Fee**: Non-refundable fee that goes to miner/staker rewards
- **48-Hour Voting Period**: Fixed voting window for all proposals
- **One Wallet, One Vote**: Democratic voting (not stake-weighted)
- **51% Majority**: Simple majority wins (YES >= 51%)
- **Instant Notifications**: "DING DONG BITCH" alerts to all eligible voters
- **Auto-Close**: Proposals automatically close after 48 hours

## Eligibility

### Who Can Propose?
- LAND deed holders
- Founders

### Who Can Vote?
- LAND deed holders
- Founders

## API Documentation

### 1. Submit Proposal

**Endpoint:** `POST /gov/proposal/create`

**Cost:** 10,000 LAND (non-refundable)

**Request Body:**
```json
{
  "title": "Increase base staking reward to 4.5 LAND",
  "proposal_type": "economic",
  "body": "Long description here...",
  "technical_impact": "Node update required, changes staking logic.",
  "proposer_wallet": "0x..."
}
```

**Proposal Types:**
- `protocol` - Protocol changes
- `economic` - Economic/tokenomics changes  
- `feature` - Feature requests
- `community` - Community proposals
- `general` - General proposals

**Response:**
```json
{
  "success": true,
  "id": "uuid...",
  "status": "open",
  "created_at": 1700000000,
  "closes_at": 1700172800,
  "title": "...",
  "proposal_type": "economic",
  "body": "...",
  "technical_impact": "..."
}
```

**Errors:**
- `400` - Missing required fields
- `400` - Only LAND deed holders and founders can create proposals
- `400` - Insufficient LAND (requires 10,000 LAND)

---

### 2. Vote on Proposal

**Endpoint:** `POST /gov/vote`

**Request Body:**
```json
{
  "proposal_id": "uuid...",
  "voter_wallet": "0x...",
  "vote": "yes"
}
```

**Vote Options:** `"yes"` or `"no"`

**Response:**
```json
{
  "success": true,
  "proposal_id": "uuid...",
  "your_vote": "yes",
  "yes_votes": 123,
  "no_votes": 45
}
```

**Errors:**
- `400` - Missing required fields
- `400` - Only LAND deed holders and founders can vote
- `400` - Invalid vote choice (must be 'yes' or 'no')
- `400` - Proposal is not open for voting
- `400` - Voting window has closed
- `400` - Already voted on this proposal

---

### 3. Get Proposal Details

**Endpoint:** `GET /gov/proposal/:id`

**Response:**
```json
{
  "success": true,
  "id": "uuid...",
  "title": "...",
  "proposal_type": "economic",
  "body": "...",
  "technical_impact": "...",
  "proposer_wallet": "0x...",
  "created_at": 1700000000,
  "closes_at": 1700172800,
  "status": "Open",
  "yes_votes": 123,
  "no_votes": 45
}
```

**Status Values:**
- `Open` - Currently accepting votes
- `Approved` - Passed with YES >= 51%
- `Rejected` - Failed with YES < 51%
- `Expired` - Closed with no votes

---

### 4. List Active Proposals

**Endpoint:** `GET /gov/proposals?status=active`

**Response:**
```json
{
  "success": true,
  "proposals": [
    {
      "id": "uuid...",
      "title": "...",
      "status": "Open",
      "created_at": 1700000000,
      "closes_at": 1700172800,
      "yes_votes": 123,
      "no_votes": 45
    }
  ],
  "count": 5
}
```

---

### 5. List Proposal History

**Endpoint:** `GET /gov/proposals?status=history&limit=50&offset=0`

**Query Parameters:**
- `limit` - Number of proposals to return (default: 50)
- `offset` - Pagination offset (default: 0)

**Response:**
```json
{
  "success": true,
  "proposals": [
    {
      "id": "uuid...",
      "title": "...",
      "status": "Approved",
      "created_at": 1699000000,
      "closes_at": 1699172800,
      "yes_votes": 523,
      "no_votes": 214
    }
  ],
  "count": 50
}
```

---

### 6. Tally Proposal

**Endpoint:** `GET /gov/tally/:id`

Returns current vote tally and status.

**Response:**
```json
{
  "success": true,
  "proposal_id": "uuid...",
  "status": "Approved",
  "yes_votes": 523,
  "no_votes": 214,
  "total_votes": 737
}
```

---

### 7. Get Governance Config

**Endpoint:** `GET /gov/config`

**Response:**
```json
{
  "proposal_fee_land": 10000,
  "voting_period_hours": 48,
  "pass_threshold_percent": 51,
  "vote_type": "one_wallet_one_vote",
  "eligible_voters": "land_deed_holders_and_founders"
}
```

---

### 8. Get Governance Stats

**Endpoint:** `GET /gov/stats`

**Response:**
```json
{
  "proposals": {
    "total": 127,
    "open": 5,
    "approved": 87,
    "rejected": 32,
    "expired": 3
  },
  "votes": {
    "total": 4521
  },
  "notifications": {
    "total": 12340,
    "unread": 234
  },
  "config": {
    "proposal_fee_land": 10000000000,
    "voting_period_hours": 48,
    "pass_threshold_percent": 51,
    "vote_type": "one_wallet_one_vote"
  }
}
```

---

## Notification API

### 9. Get Wallet Notifications

**Endpoint:** `GET /wallet/notifications?wallet=0x...`

**Query Parameters:**
- `wallet` - Wallet address (required)

**Response:**
```json
{
  "success": true,
  "notifications": [
    {
      "id": "uuid...",
      "wallet": "0x...",
      "created_at": 1700000000,
      "message": "DING DONG BITCH – A new governance proposal is live. Tap to read & vote.",
      "kind": "governance_proposal",
      "related_id": "proposal-uuid...",
      "read": false
    }
  ],
  "count": 5,
  "unread_count": 3
}
```

---

### 10. Mark Notification as Read

**Endpoint:** `POST /wallet/notifications/:id/read`

**Response:**
```json
{
  "success": true,
  "message": "Notification marked as read"
}
```

---

## How It Works

### Proposal Lifecycle

```
1. SUBMISSION
   ↓
   - Proposer pays 10,000 LAND fee
   - Fee goes to staking rewards pool
   - Proposal created with 48-hour window
   ↓
2. NOTIFICATION
   ↓
   - All deed holders get notified
   - "DING DONG BITCH" message sent
   ↓
3. VOTING (48 hours)
   ↓
   - Eligible wallets vote YES/NO
   - One wallet = one vote
   - Cannot change vote after casting
   ↓
4. AUTO-CLOSE
   ↓
   - After 48 hours, proposal closes
   - YES >= 51% → Approved
   - YES < 51% → Rejected
   - No votes → Expired
```

### Fee Distribution

```
Proposer pays 10,000 LAND
         ↓
Staking Rewards Pool
         ↓
Distributed to miners/stakers
```

### Voting Rules

- **One wallet, one vote** (not weighted by LAND holdings)
- **51% majority wins** (YES votes / total votes)
- **No quorum requirement** (any participation counts)
- **No vote changes** (final after casting)
- **48-hour fixed window** (no extensions)

---

## Frontend Integration

### Governance Page Components

**Required UI Elements:**
1. **Active Proposals List**
   - Card-based display
   - "View & Vote" buttons
   - Countdown timers
   - YES/NO progress bars

2. **Submit Proposal Form**
   - Only shown to eligible users
   - Fields: title, type, body, technical impact
   - 10,000 LAND fee warning

3. **History View**
   - Table/list of closed proposals
   - Status badges (Approved/Rejected/Expired)
   - Final vote counts

4. **Notifications Bell**
   - Unread count badge
   - Dropdown with notification list
   - "DING DONG BITCH" message style

### Example React Component

```typescript
interface Proposal {
  id: string;
  title: string;
  proposal_type: string;
  body: string;
  technical_impact?: string;
  proposer_wallet: string;
  created_at: number;
  closes_at: number;
  status: 'Open' | 'Approved' | 'Rejected' | 'Expired';
  yes_votes: number;
  no_votes: number;
}

function GovernancePage() {
  const [proposals, setProposals] = useState<Proposal[]>([]);
  const [userWallet, setUserWallet] = useState<string>('');
  const [isEligible, setIsEligible] = useState(false);

  // Fetch active proposals
  useEffect(() => {
    fetch('/gov/proposals?status=active')
      .then(res => res.json())
      .then(data => setProposals(data.proposals));
  }, []);

  // Check eligibility (has deed or is founder)
  useEffect(() => {
    // Check if user owns a deed via your wallet API
    checkDeedOwnership(userWallet).then(setIsEligible);
  }, [userWallet]);

  const submitProposal = async (formData) => {
    const response = await fetch('/gov/proposal/create', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        ...formData,
        proposer_wallet: userWallet
      })
    });
    
    if (response.ok) {
      toast.success('Proposal submitted! Voting live for 48 hours.');
      // Refresh proposals list
    } else {
      const error = await response.json();
      toast.error(error.error);
    }
  };

  const vote = async (proposalId: string, voteChoice: 'yes' | 'no') => {
    const response = await fetch('/gov/vote', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        proposal_id: proposalId,
        voter_wallet: userWallet,
        vote: voteChoice
      })
    });

    if (response.ok) {
      toast.success(`Voted ${voteChoice.toUpperCase()}!`);
      // Update proposal in state
    }
  };

  return (
    <div>
      <h1>Vision Governance</h1>
      <p>Where LAND holders decide the future of the chain.</p>
      
      {/* Active Proposals */}
      <section>
        <h2>Active Proposals</h2>
        {proposals.map(proposal => (
          <ProposalCard 
            key={proposal.id}
            proposal={proposal}
            onVote={(choice) => vote(proposal.id, choice)}
            canVote={isEligible}
          />
        ))}
      </section>

      {/* Submit Form (only if eligible) */}
      {isEligible && (
        <section>
          <h2>Submit Proposal</h2>
          <ProposalForm onSubmit={submitProposal} />
          <p className="warning">
            Submitting costs 10,000 LAND and is non-refundable.
          </p>
        </section>
      )}
    </div>
  );
}
```

---

## Security & Best Practices

### Backend Security
- ✅ Eligibility checks on every vote/proposal
- ✅ LAND balance verification before proposal submission
- ✅ One vote per wallet enforced
- ✅ Automatic expiration handling
- ✅ Fee routing to staking pool

### Frontend Best Practices
- Check eligibility before showing forms
- Display countdown timers for voting windows
- Show real-time vote counts
- Prevent double voting (disable buttons after voting)
- Clear error messages for insufficient LAND
- Link to LAND marketplace if user needs deeds

### Rate Limiting Recommendations
- Limit proposal creation to 1 per wallet per hour
- Cache notification checks (5-second TTL)
- Debounce vote button clicks

---

## Testing

### Test Scenarios

1. **Submit Proposal**
   ```bash
   curl -X POST http://localhost:8080/gov/proposal/create \
     -H "Content-Type: application/json" \
     -d '{
       "title": "Test Proposal",
       "proposal_type": "general",
       "body": "Test body",
       "proposer_wallet": "founder_address"
     }'
   ```

2. **Vote YES**
   ```bash
   curl -X POST http://localhost:8080/gov/vote \
     -H "Content-Type: application/json" \
     -d '{
       "proposal_id": "uuid...",
       "voter_wallet": "founder_address",
       "vote": "yes"
     }'
   ```

3. **Get Notifications**
   ```bash
   curl "http://localhost:8080/wallet/notifications?wallet=founder_address"
   ```

---

## Troubleshooting

### Common Issues

**"Only LAND deed holders and founders can create proposals"**
- User doesn't own a LAND deed
- Solution: Direct to LAND marketplace

**"Insufficient LAND to submit proposal"**
- User has < 10,000 LAND
- Solution: Show current balance, link to acquire LAND

**"Already voted on this proposal"**
- User trying to vote twice
- Solution: Show their previous vote, explain no changes allowed

**"Voting window has closed"**
- Proposal expired
- Solution: Show proposal history, final results

---

## Future Enhancements

Potential improvements for v2:

1. **Delegation**: Allow deed holders to delegate votes
2. **Quadratic Voting**: Weight votes by sqrt(deeds owned)
3. **Proposal Categories**: Different rules for different types
4. **Execution Logic**: Auto-execute approved proposals
5. **Discussion Forum**: Threaded comments per proposal
6. **Proposal Amendments**: Allow proposers to update before voting
7. **Multi-Sig Proposals**: Require multiple proposers for critical changes

---

## Support

For questions or issues:
- GitHub Issues: https://github.com/vision-node/issues
- Discord: #governance channel
- Email: governance@vision-node.io

---

**Last Updated:** November 2025  
**System Version:** 0.7.9+  
**Democracy Level:** "DING DONG BITCH"
