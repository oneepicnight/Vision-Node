import { useState, useEffect } from 'react'
import { useWalletStore } from '../state/wallet'
import './GovernancePanel.css'

interface Proposal {
  proposal_id: string
  title: string
  body: string
  proposal_type: string
  proposer_wallet: string
  status: string
  yes_votes: number
  no_votes: number
  total_votes: number
  created_at: string
  voting_ends_at: string
  technical_impact?: string
}

interface GovernanceStats {
  active_proposals: number
  total_votes: number
  proposals_passed: number
}

export default function GovernancePanel() {
  const { profile, balances } = useWalletStore()
  const [stats, setStats] = useState<GovernanceStats>({ active_proposals: 0, total_votes: 0, proposals_passed: 0 })
  const [activeProposals, setActiveProposals] = useState<Proposal[]>([])
  const [loading, setLoading] = useState(true)
  const [activeTab, setActiveTab] = useState<'active' | 'create'>('active')
  
  // Form state for creating proposals
  const [proposalTitle, setProposalTitle] = useState('')
  const [proposalType, setProposalType] = useState('')
  const [proposalBody, setProposalBody] = useState('')
  const [technicalImpact, setTechnicalImpact] = useState('')
  const [submitting, setSubmitting] = useState(false)

  // Check if user owns a deed (DEED balance > 0) - for now use LAND > 0 as proxy
  const hasDeed = (balances?.LAND || 0) > 0

  useEffect(() => {
    if (hasDeed) {
      fetchGovernanceData()
      const interval = setInterval(fetchGovernanceData, 30000) // Refresh every 30s
      return () => clearInterval(interval)
    }
  }, [hasDeed])

  const fetchGovernanceData = async () => {
    try {
      // Fetch stats
      const statsResponse = await fetch('http://127.0.0.1:7070/gov/stats')
      if (statsResponse.ok) {
        const statsData = await statsResponse.json()
        setStats(statsData)
      }

      // Fetch active proposals
      const proposalsResponse = await fetch('http://127.0.0.1:7070/gov/proposals?status=open')
      if (proposalsResponse.ok) {
        const proposalsData = await proposalsResponse.json()
        setActiveProposals(proposalsData)
      }
    } catch (error) {
      console.debug('Failed to fetch governance data:', error)
    } finally {
      setLoading(false)
    }
  }

  const handleVote = async (proposalId: string, voteYes: boolean) => {
    if (!profile?.address) return

    try {
      const response = await fetch('http://127.0.0.1:7070/gov/vote', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          proposal_id: proposalId,
          wallet: profile.address,
          vote_yes: voteYes
        })
      })

      if (response.ok) {
        alert(`✅ Vote cast successfully!`)
        fetchGovernanceData()
      } else {
        const error = await response.json()
        alert(`❌ Vote failed: ${error.error || 'Unknown error'}`)
      }
    } catch (error) {
      alert(`❌ Vote error: ${error}`)
    }
  }

  const handleCreateProposal = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!profile?.address || !hasDeed) return

    if ((balances?.LAND || 0) < 10000) {
      alert('❌ Insufficient LAND balance. Need 10,000 LAND to create a proposal.')
      return
    }

    setSubmitting(true)
    try {
      const response = await fetch('http://127.0.0.1:7070/gov/proposal', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          title: proposalTitle,
          body: proposalBody,
          proposal_type: proposalType,
          proposer_wallet: profile.address,
          technical_impact: technicalImpact || undefined
        })
      })

      if (response.ok) {
        alert('✅ Proposal created successfully!')
        setProposalTitle('')
        setProposalType('')
        setProposalBody('')
        setTechnicalImpact('')
        setActiveTab('active')
        fetchGovernanceData()
      } else {
        const error = await response.json()
        alert(`❌ Failed to create proposal: ${error.error || 'Unknown error'}`)
      }
    } catch (error) {
      alert(`❌ Error creating proposal: ${error}`)
    } finally {
      setSubmitting(false)
    }
  }

  const getTimeRemaining = (endTime: string) => {
    const now = new Date().getTime()
    const end = new Date(endTime).getTime()
    const diff = end - now

    if (diff <= 0) return 'Voting Ended'

    const days = Math.floor(diff / (1000 * 60 * 60 * 24))
    const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60))
    
    if (days > 0) return `${days}d ${hours}h remaining`
    return `${hours}h remaining`
  }

  if (!hasDeed) {
    return (
      <div className="governance-panel">
        <div className="governance-header">
          <h3 className="governance-title">🏛️ LAND GOVERNANCE</h3>
          <p className="governance-subtitle">Decentralized Democracy - Your Voice Matters</p>
        </div>
        <div className="governance-locked">
          <div className="locked-icon">🔒</div>
          <h4>Deed Holder Access Only</h4>
          <p>Governance participation requires owning a LAND Deed NFT.</p>
          <p>Visit the Exchange to acquire a deed and participate in protocol decisions.</p>
        </div>
      </div>
    )
  }

  return (
    <div className="governance-panel">
      <div className="governance-header">
        <h3 className="governance-title">🏛️ LAND GOVERNANCE</h3>
        <p className="governance-subtitle">Decentralized Democracy - Your Voice Matters</p>
      </div>

      {/* Stats */}
      <div className="governance-stats">
        <div className="gov-stat">
          <div className="gov-stat-label">Active Proposals</div>
          <div className="gov-stat-value">{stats.active_proposals}</div>
        </div>
        <div className="gov-stat">
          <div className="gov-stat-label">Total Votes Cast</div>
          <div className="gov-stat-value">{stats.total_votes}</div>
        </div>
        <div className="gov-stat">
          <div className="gov-stat-label">Your Voting Power</div>
          <div className="gov-stat-value">1</div>
        </div>
        <div className="gov-stat">
          <div className="gov-stat-label">Proposals Passed</div>
          <div className="gov-stat-value">{stats.proposals_passed}</div>
        </div>
      </div>

      {/* Tabs */}
      <div className="governance-tabs">
        <button 
          className={`gov-tab ${activeTab === 'active' ? 'active' : ''}`}
          onClick={() => setActiveTab('active')}
        >
          🗳️ Active Proposals
        </button>
        <button 
          className={`gov-tab ${activeTab === 'create' ? 'active' : ''}`}
          onClick={() => setActiveTab('create')}
        >
          ✍️ Create Proposal
        </button>
      </div>

      {/* Active Proposals Tab */}
      {activeTab === 'active' && (
        <div className="governance-content">
          {loading ? (
            <div className="governance-loading">Loading proposals...</div>
          ) : activeProposals.length === 0 ? (
            <div className="governance-empty">
              <p>No active proposals at the moment.</p>
            </div>
          ) : (
            <div className="proposals-list">
              {activeProposals.map(proposal => {
                const yesPercent = proposal.total_votes > 0 
                  ? Math.round((proposal.yes_votes / proposal.total_votes) * 100) 
                  : 0

                return (
                  <div key={proposal.proposal_id} className="proposal-card">
                    <div className="proposal-card-header">
                      <div>
                        <div className="proposal-card-title">{proposal.title}</div>
                        <div className="proposal-card-meta">
                          Type: {proposal.proposal_type} • By: {proposal.proposer_wallet.substring(0, 10)}...
                        </div>
                      </div>
                      <span className="proposal-status">{proposal.status}</span>
                    </div>
                    
                    <div className="proposal-card-body">
                      <p>{proposal.body.substring(0, 200)}{proposal.body.length > 200 ? '...' : ''}</p>
                    </div>

                    <div className="vote-bar-container">
                      <div className="vote-bar">
                        <div className="vote-bar-fill" style={{ width: `${yesPercent}%` }}></div>
                      </div>
                      <div className="vote-counts">
                        <span>✅ YES: {proposal.yes_votes}</span>
                        <span>❌ NO: {proposal.no_votes}</span>
                      </div>
                    </div>

                    <div className="proposal-card-footer">
                      <span className="time-remaining">
                        ⏰ {getTimeRemaining(proposal.voting_ends_at)}
                      </span>
                      <div className="vote-buttons">
                        <button 
                          className="vote-btn yes"
                          onClick={() => handleVote(proposal.proposal_id, true)}
                        >
                          ✅ Vote YES
                        </button>
                        <button 
                          className="vote-btn no"
                          onClick={() => handleVote(proposal.proposal_id, false)}
                        >
                          ❌ Vote NO
                        </button>
                      </div>
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </div>
      )}

      {/* Create Proposal Tab */}
      {activeTab === 'create' && (
        <div className="governance-content">
          <div className="fee-notice">
            <strong>⚠️ Proposal Fee: 10,000 LAND (Non-Refundable)</strong>
            <p>Only LAND deed holders can submit proposals.</p>
          </div>
          <form onSubmit={handleCreateProposal} className="proposal-form">
            <div className="form-group">
              <label>Proposal Title *</label>
              <input
                type="text"
                value={proposalTitle}
                onChange={(e) => setProposalTitle(e.target.value)}
                placeholder="Brief, descriptive title"
                required
              />
            </div>
            <div className="form-group">
              <label>Proposal Type *</label>
              <select
                value={proposalType}
                onChange={(e) => setProposalType(e.target.value)}
                required
              >
                <option value="">Select proposal type...</option>
                <option value="protocol_upgrade">Protocol Upgrade</option>
                <option value="parameter_change">Parameter Change</option>
                <option value="treasury_allocation">Treasury Allocation</option>
                <option value="community_initiative">Community Initiative</option>
                <option value="emergency_action">Emergency Action</option>
                <option value="other">Other</option>
              </select>
            </div>
            <div className="form-group">
              <label>Proposal Description *</label>
              <textarea
                value={proposalBody}
                onChange={(e) => setProposalBody(e.target.value)}
                placeholder="Detailed description of your proposal"
                required
                rows={6}
              />
            </div>
            <div className="form-group">
              <label>Technical Impact Assessment (Optional)</label>
              <textarea
                value={technicalImpact}
                onChange={(e) => setTechnicalImpact(e.target.value)}
                placeholder="Describe any technical changes, risks, or implementation details"
                rows={4}
              />
            </div>
            <button type="submit" className="submit-btn" disabled={submitting}>
              {submitting ? '⏳ Submitting...' : 'Submit Proposal (Costs 10,000 LAND)'}
            </button>
          </form>
        </div>
      )}
    </div>
  )
}
