# Vision Governance - Quick Reference

## 🎯 Key Facts

- **Fee:** 10,000 LAND (non-refundable, goes to stakers)
- **Voting Period:** 48 hours (fixed)
- **Win Condition:** 51% YES votes
- **Vote Weight:** One wallet = one vote
- **Eligibility:** LAND deed holders + founders only

## 🚀 Quick Start

### Submit a Proposal (Backend)
```bash
POST /gov/proposal/create
{
  "title": "Your proposal title",
  "proposal_type": "economic",
  "body": "Long description...",
  "technical_impact": "Optional details",
  "proposer_wallet": "0x..."
}
```

### Vote (Backend)
```bash
POST /gov/vote
{
  "proposal_id": "uuid",
  "voter_wallet": "0x...",
  "vote": "yes"
}
```

### Get Notifications
```bash
GET /wallet/notifications?wallet=0x...
```

## 📋 API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/gov/proposal/create` | POST | Submit proposal (costs 10k LAND) |
| `/gov/proposal/:id` | GET | Get proposal details |
| `/gov/proposals?status=active` | GET | List active proposals |
| `/gov/proposals?status=history` | GET | List closed proposals |
| `/gov/vote` | POST | Cast YES/NO vote |
| `/gov/tally/:id` | GET | Get vote tally |
| `/gov/config` | GET | Get config |
| `/gov/stats` | GET | Get statistics |
| `/wallet/notifications` | GET | Get notifications |
| `/wallet/notifications/:id/read` | POST | Mark as read |

## 🎨 Frontend Checklist

### Governance Page
- [ ] Active proposals list with countdown timers
- [ ] YES/NO voting buttons (only if eligible)
- [ ] Progress bars showing vote distribution
- [ ] Submit proposal form (only for deed holders)
- [ ] 10,000 LAND fee warning
- [ ] Proposal history table
- [ ] Status badges (Open/Approved/Rejected/Expired)

### Notifications
- [ ] Bell icon in header with unread count
- [ ] Dropdown showing notifications
- [ ] "DING DONG BITCH" message styling
- [ ] Click notification → open proposal
- [ ] Mark as read on click

### UX Copy
```
Heading: "Vision Governance"
Subheading: "Where LAND holders decide the future of the chain."

Empty state: "No active proposals right now. Got an idea worth 10,000 LAND?"

Voting hint: "You have one vote per proposal. Choose wisely."

Fee warning: "Submitting a proposal costs 10,000 LAND and is non-refundable."

Notification: "DING DONG BITCH – A new governance proposal is live. Tap to read & vote."
```

## 🔐 Security Checks

### Backend
- ✅ Verify deed ownership before proposal/vote
- ✅ Check LAND balance >= 10,000 before proposal
- ✅ Enforce one vote per wallet
- ✅ Auto-close after 48 hours
- ✅ Route fees to staking pool

### Frontend
- ✅ Hide forms if user not eligible
- ✅ Show eligibility requirements
- ✅ Disable vote buttons after voting
- ✅ Show countdown timer
- ✅ Validate form before submission

## 📊 Status Flow

```
SUBMISSION → NOTIFICATION → VOTING (48h) → AUTO-CLOSE
    ↓            ↓              ↓              ↓
10k LAND    "DING DONG"   YES/NO votes    Result
   fee         broadcast                   calculated
```

## 🎯 Decision Logic

```javascript
if (total_votes === 0) {
  status = "Expired"
} else {
  yes_percent = (yes_votes / total_votes) * 100
  status = yes_percent >= 51 ? "Approved" : "Rejected"
}
```

## 🛠️ Test Commands

```bash
# List active
curl "http://localhost:8080/gov/proposals?status=active"

# Submit (requires deed/founder + 10k LAND)
curl -X POST http://localhost:8080/gov/proposal/create \
  -H "Content-Type: application/json" \
  -d '{"title":"Test","proposal_type":"general","body":"Test","proposer_wallet":"founder_address"}'

# Vote YES
curl -X POST http://localhost:8080/gov/vote \
  -H "Content-Type: application/json" \
  -d '{"proposal_id":"uuid","voter_wallet":"founder_address","vote":"yes"}'

# Get notifications
curl "http://localhost:8080/wallet/notifications?wallet=founder_address"
```

## 🐛 Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| "Only LAND deed holders..." | User doesn't own deed | Direct to marketplace |
| "Insufficient LAND..." | Balance < 10,000 LAND | Show balance, link to get LAND |
| "Already voted..." | Trying to vote twice | Show previous vote |
| "Voting window has closed" | Proposal expired | Show final results |

## 📝 Module Structure

```
src/
  governance_democracy.rs     # Core logic
  main.rs                     # API endpoints
  land_deeds.rs               # Deed ownership checks

docs/
  GOVERNANCE_DEMOCRACY.md     # Full documentation
```

## 🔥 Implementation Status

✅ Backend complete
✅ 10 API endpoints
✅ Notification system
✅ Auto-close logic
✅ Fee routing to stakers
✅ One wallet = one vote
✅ 48-hour voting window
✅ 51% majority rule
⏳ Frontend implementation (your turn!)

---

**Ready to build the UI!** Use the full documentation in `GOVERNANCE_DEMOCRACY.md` for detailed integration guide.
