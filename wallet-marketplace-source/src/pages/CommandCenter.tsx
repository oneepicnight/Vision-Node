import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useWalletStore } from '../state/wallet'
import { useNodeStatus } from '../hooks/useNodeStatus'
import { useMiningStatus } from '../hooks/useMiningStatus'
import { useGuardianStatus } from '../hooks/useGuardianStatus'
import { useConstellationStatus, computeP2PHealth } from '../hooks/useConstellationStatus'
import RoutingIntelligenceDashboard from '../components/RoutingIntelligenceDashboard'
import { VaultStatusDashboard } from '../components/VaultStatusDashboard'
import '../styles/command-center.css'
import '../styles/routing-intelligence.css'

interface ApprovalStatus {
  approved: boolean
  wallet_address: string | null
  node_id: string
}

// Helper function to get mood emoji
function getMoodEmoji(moodType: string): string {
  const moodEmojis: Record<string, string> = {
    calm: '🌊',
    warning: '⚠️',
    storm: '⛈️',
    celebration: '🎉',
    guardian: '🛡️',
    wounded: '💔',
    rage: '🔥'
  }
  return moodEmojis[moodType.toLowerCase()] || '🌌'
}

export default function CommandCenter() {
  const navigate = useNavigate()
  const { profile, balances } = useWalletStore()
  
  // Use shared hooks
  const nodeStatus = useNodeStatus()
  const miningStatus = useMiningStatus()
  const guardianStatus = useGuardianStatus()
  const constellation = useConstellationStatus()
  
  // Compute P2P IPv4 health
  const p2pHealth = computeP2PHealth(constellation)
  
  const [mood, setMood] = useState<{
    mood: string
    score: number
    reason: string
    details?: any
  } | null>(null)
  
  const [events, setEvents] = useState<Array<{id: string, time: string, type: string, message: string}>>([])
  
  // Node approval state
  const [approvalStatus, setApprovalStatus] = useState<ApprovalStatus | null>(null)
  const [approving, setApproving] = useState(false)
  
  // Hashrate history
  const [hashrateHistory, setHashrateHistory] = useState<number[]>([])
  
  // Mining stats
  const [miningStats, setMiningStats] = useState<any>(null)

  // Fetch mood data
  useEffect(() => {
    const fetchMood = async () => {
      try {
        const moodResponse = await fetch('http://127.0.0.1:7070/api/mood')
        const moodData = await moodResponse.json()
        
        // Store mood data for display
        if (moodData.mood) {
          setMood({
            mood: moodData.mood,
            score: moodData.score,
            reason: moodData.reason,
            details: moodData.details
          })
          
          const moodEvent = `Mood: ${moodData.mood} - ${moodData.reason}`
          setEvents(prev => [{
            id: `mood-${Date.now()}`,
            time: new Date().toLocaleTimeString(),
            type: 'mood',
            message: moodEvent
          }, ...prev].slice(0, 20))
        }
      } catch (err) {
        console.debug('Mood fetch failed:', err)
      }
    }
    
    fetchMood()
    const interval = setInterval(fetchMood, 10000)
    return () => clearInterval(interval)
  }, [])
  
  // Fetch approval status
  useEffect(() => {
    const fetchApproval = async () => {
      try {
        const response = await fetch('http://127.0.0.1:7070/api/node/approval/status')
        if (response.ok) {
          const data = await response.json()
          setApprovalStatus(data)
        }
      } catch (err) {
        console.debug('Approval status fetch failed:', err)
      }
    }
    
    fetchApproval()
    const interval = setInterval(fetchApproval, 30000)
    return () => clearInterval(interval)
  }, [])
  
  // Fetch recent blocks
  useEffect(() => {
    const fetchBlocks = async () => {
      try {
        const response = await fetch('http://127.0.0.1:7070/api/miner/stats')
        if (response.ok) {
          const data = await response.json()
          setMiningStats(data)
          // Add to hashrate history
          if (data.average_hashrate) {
            setHashrateHistory(prev => [...prev.slice(-19), data.average_hashrate])
          }
        }
      } catch (err) {
        console.debug('Mining stats fetch failed:', err)
      }
    }
    
    fetchBlocks()
    const interval = setInterval(fetchBlocks, 5000)
    return () => clearInterval(interval)
  }, [])
  
  // Mock event stream (replace with real event source)
  useEffect(() => {
    const addEvent = (type: string, message: string) => {
      const event = {
        id: Date.now().toString(),
        time: new Date().toLocaleTimeString(),
        type,
        message
      }
      setEvents(prev => [event, ...prev].slice(0, 20)) // Keep last 20 events
    }
    
    // Example events
    if (nodeStatus.online) {
      addEvent('node', 'Node connected to network')
    }
    
    if (miningStatus.active) {
      addEvent('mining', `Mining started in ${miningStatus.mode} mode`)
    }
  }, [nodeStatus.online, miningStatus.active, miningStatus.mode])
  
  const handleStartMining = async (mode: 'solo' | 'pool') => {
    try {
      await fetch('http://127.0.0.1:7070/api/miner/start', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ pool_mining: mode === 'pool' })
      })
    } catch (err) {
      console.error('Failed to start mining:', err)
    }
  }
  
  const handleStopMining = async () => {
    try {
      await fetch('http://127.0.0.1:7070/api/miner/stop', {
        method: 'POST'
      })
    } catch (err) {
      console.error('Failed to stop mining:', err)
    }
  }
  
  const handleApproveNode = async () => {
    if (approving) return
    setApproving(true)
    
    try {
      // Step 1: Get wallet address from status
      const statusResponse = await fetch('http://127.0.0.1:7070/api/website/status')
      if (!statusResponse.ok) {
        alert('Failed to fetch node status')
        return
      }
      
      const statusText = await statusResponse.text()
      if (!statusText) {
        alert('Empty response from status endpoint')
        return
      }
      
      const status = JSON.parse(statusText)
      const walletAddress = status.wallet_address
      
      if (!walletAddress) {
        alert('No wallet found. Please create a wallet first.')
        return
      }
      
      // Step 2: Get challenge
      const challengeResponse = await fetch('http://127.0.0.1:7070/api/node/approval/challenge', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ wallet_address: walletAddress })
      })
      
      if (!challengeResponse.ok) {
        const errorText = await challengeResponse.text()
        alert(`Failed to get challenge: ${errorText}`)
        return
      }
      
      const challengeData = await challengeResponse.json()
      
      // Step 3: Sign the challenge
      const signResponse = await fetch('http://127.0.0.1:7070/api/wallet/sign_message', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ 
          wallet_address: walletAddress,
          message: challengeData.message 
        })
      })
      
      if (!signResponse.ok) {
        const errorText = await signResponse.text()
        alert(`Failed to sign message: ${errorText}`)
        return
      }
      
      const signData = await signResponse.json()
      
      // Step 4: Submit approval
      const approvalResponse = await fetch('http://127.0.0.1:7070/api/node/approval/submit', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          wallet_address: walletAddress,
          ts_unix: challengeData.ts_unix,
          nonce_hex: challengeData.nonce_hex,
          signature_b64: signData.signature_b64
        })
      })
      
      const approvalText = await approvalResponse.text()
      let approvalResult
      try {
        approvalResult = JSON.parse(approvalText)
      } catch (e) {
        approvalResult = { ok: false, error: approvalText }
      }
      
      if (approvalResult.ok) {
        alert('✅ Node approved successfully! Mining is now enabled.')
        // Refresh approval status
        const response = await fetch('http://127.0.0.1:7070/api/node/approval/status')
        if (response.ok) {
          const data = await response.json()
          setApprovalStatus(data)
        }
      } else {
        alert(`❌ Approval failed: ${approvalResult.error || 'Unknown error'}`)
      }
    } catch (error: any) {
      alert(`❌ Approval error: ${error.message}`)
    } finally {
      setApproving(false)
    }
  }

  return (
    <div className="command-center">
      {/* Header */}
      <div className="cc-header">
        <h1 className="cc-title">COMMAND CENTER</h1>
        <p className="cc-subtitle">Real-time overview of your node, mining, wallet, and Guardian status.</p>
        
        {/* Mood Ring */}
        {mood && (
          <div className="cc-mood-ring">
            <div className={`cc-mood-indicator cc-mood-${mood.mood}`}>
              <span className="cc-mood-emoji">{getMoodEmoji(mood.mood)}</span>
              <div className="cc-mood-info">
                <span className="cc-mood-name">{mood.mood.toUpperCase()}</span>
                <span className="cc-mood-score">Health: {(mood.score * 100).toFixed(1)}%</span>
              </div>
            </div>
            <p className="cc-mood-reason">{mood.reason}</p>
          </div>
        )}
      </div>

      {/* Top Row - Status Cards */}
      <div className="cc-status-row">
        {/* Node Status */}
        <div className="cc-status-card">
          <div className="cc-card-header">
            <span className="cc-card-title">NODE STATUS</span>
            <div className={`cc-status-indicator ${nodeStatus.online ? 'online' : 'offline'}`}>
              {nodeStatus.online ? 'Online' : 'Offline'}
            </div>
          </div>
          <div className="cc-card-body">
            <div className="cc-stat">
              <span className="cc-stat-label">Network</span>
              <span className="cc-stat-value">{nodeStatus.network}</span>
            </div>
            <div className="cc-stat">
              <span className="cc-stat-label">Height</span>
              <span className="cc-stat-value">{nodeStatus.height.toLocaleString()}</span>
            </div>
          </div>
        </div>

        {/* P2P Connection Health (P2P Robustness #7) */}
        <div className="cc-status-card" style={{
          background: nodeStatus.p2pHealth === 'isolated' ? 'rgba(220, 38, 38, 0.1)' :
                      nodeStatus.p2pHealth === 'weak' ? 'rgba(245, 158, 11, 0.1)' :
                      nodeStatus.p2pHealth === 'immortal' ? 'rgba(59, 130, 246, 0.1)' :
                      'rgba(34, 197, 94, 0.1)',
          borderColor: nodeStatus.p2pHealth === 'isolated' ? 'rgba(220, 38, 38, 0.5)' :
                       nodeStatus.p2pHealth === 'weak' ? 'rgba(245, 158, 11, 0.5)' :
                       nodeStatus.p2pHealth === 'immortal' ? 'rgba(59, 130, 246, 0.5)' :
                       'rgba(34, 197, 94, 0.5)'
        }}>
          <div className="cc-card-header">
            <span className="cc-card-title">P2P HEALTH</span>
            <div className={`cc-status-indicator ${nodeStatus.p2pHealth === 'isolated' ? 'offline' : 'mining'}`}>
              {nodeStatus.p2pHealth === 'isolated' ? '❌ ISOLATED' :
               nodeStatus.p2pHealth === 'weak' ? '⚠️ WEAK' :
               nodeStatus.p2pHealth === 'ok' ? '🟢 OK' :
               nodeStatus.p2pHealth === 'stable' ? '🟢 STABLE' :
               '🔵 IMMORTAL'}
            </div>
          </div>
          <div className="cc-card-body">
            <div className="cc-stat">
              <span className="cc-stat-label">Status</span>
              <span className="cc-stat-value" style={{
                color: nodeStatus.p2pHealth === 'isolated' ? '#dc2626' :
                       nodeStatus.p2pHealth === 'weak' ? '#f59e0b' :
                       nodeStatus.p2pHealth === 'immortal' ? '#3b82f6' :
                       'var(--accent-green)'
              }}>
                {nodeStatus.p2pHealth === 'isolated' ? 'No peers connected' :
                 nodeStatus.p2pHealth === 'weak' ? '1 peer - vulnerable' :
                 nodeStatus.p2pHealth === 'ok' ? 'Stable mining connection' :
                 nodeStatus.p2pHealth === 'stable' ? 'Excellent network health' :
                 'Maximum resilience 🎉'}
              </span>
            </div>
            <div className="cc-stat">
              <span className="cc-stat-label">Peers</span>
              <span className="cc-stat-value">{nodeStatus.peerCount}</span>
            </div>
            {nodeStatus.p2pHealth === 'isolated' && (
              <div style={{
                marginTop: '0.5rem',
                padding: '0.5rem',
                background: 'rgba(220, 38, 38, 0.1)',
                borderRadius: '4px',
                fontSize: '0.85rem',
                color: '#dc2626'
              }}>
                ⚠️ Mining blocked - waiting for peers
              </div>
            )}
            {nodeStatus.p2pHealth === 'weak' && (
              <div style={{
                marginTop: '0.5rem',
                padding: '0.5rem',
                background: 'rgba(245, 158, 11, 0.1)',
                borderRadius: '4px',
                fontSize: '0.85rem',
                color: '#f59e0b'
              }}>
                ⚠️ Single peer connection - vulnerable to partition
              </div>
            )}
            {nodeStatus.p2pHealth === 'immortal' && (
              <div style={{
                marginTop: '0.5rem',
                padding: '0.5rem',
                background: 'rgba(59, 130, 246, 0.1)',
                borderRadius: '4px',
                fontSize: '0.85rem',
                color: '#3b82f6'
              }}>
                🎉 32+ peers - your node is virtually unstoppable!
              </div>
            )}
          </div>
        </div>

        {/* Mining Status */}
        <div className="cc-status-card">
          <div className="cc-card-header">
            <span className="cc-card-title">MINING STATUS</span>
            <div className={`cc-status-indicator ${miningStatus.active ? 'mining' : 'idle'}`}>
              {miningStatus.active ? 'Active' : 'Idle'}
            </div>
          </div>
          <div className="cc-card-body">
            <div className="cc-stat">
              <span className="cc-stat-label">Mode</span>
              <span className="cc-stat-value">{miningStatus.mode.toUpperCase()}</span>
            </div>
            <div className="cc-stat">
              <span className="cc-stat-label">Hashrate</span>
              <span className="cc-stat-value">
                {miningStatus.active ? `${(miningStatus.hashrate / 1000000).toFixed(2)} MH/s` : 'Idle'}
              </span>
            </div>
          </div>
        </div>

        {/* Wallet Overview */}
        <div className="cc-status-card">
          <div className="cc-card-header">
            <span className="cc-card-title">WALLET OVERVIEW</span>
          </div>
          <div className="cc-card-body">
            <div className="cc-stat">
              <span className="cc-stat-label">LAND</span>
              <span className="cc-stat-value cc-balance">{balances.LAND.toFixed(4)}</span>
            </div>
            <div className="cc-stat">
              <span className="cc-stat-label">CASH</span>
              <span className="cc-stat-value cc-balance">{balances.CASH.toFixed(2)}</span>
            </div>
            <button 
              className="cc-link-button"
              onClick={() => navigate('/wallet')}
            >
              Open Full Wallet →
            </button>
          </div>
        </div>

        {/* Guardian / Beacon */}
        <div className="cc-status-card">
          <div className="cc-card-header">
            <span className="cc-card-title">GUARDIAN / BEACON</span>
          </div>
          <div className="cc-card-body">
            <div className="cc-stat">
              <span className="cc-stat-label">Beacon</span>
              <span className={`cc-stat-value ${guardianStatus.beaconConnected ? 'cc-connected' : 'cc-disconnected'}`}>
                {guardianStatus.beaconConnected ? 'Connected' : 'Not Connected'}
              </span>
            </div>
            <div className="cc-stat">
              <span className="cc-stat-label">Guardian</span>
              <span className={`cc-stat-value ${guardianStatus.guardianOnline ? 'cc-connected' : 'cc-disconnected'}`}>
                {guardianStatus.guardianOnline ? 'Online' : 'Offline'}
              </span>
            </div>
          </div>
        </div>

        {/* P2P IPv4 Health */}
        <div 
          className="cc-status-card"
          title={constellation ? `Peers: ${constellation.connected_peers} | Known: ${constellation.total_known_peers} | Avg latency: ${constellation.avg_peer_latency_ms ?? 'n/a'} ms` : 'Loading...'}
        >
          <div className="cc-card-header">
            <span className="cc-card-title">P2P IPv4 LINK</span>
            <div className="cc-p2p-indicator">
              {p2pHealth === 'stable' && (
                <span className="cc-p2p-dot cc-p2p-stable" />
              )}
              {p2pHealth === 'weak' && (
                <span className="cc-p2p-dot cc-p2p-weak" />
              )}
              {p2pHealth === 'broken' && (
                <span className="cc-p2p-dot cc-p2p-broken" />
              )}
            </div>
          </div>
          <div className="cc-card-body">
            <div className="cc-stat">
              <span className="cc-stat-label">Status</span>
              <span className={`cc-stat-value cc-p2p-status-${p2pHealth}`}>
                {p2pHealth === 'stable' && 'Stable'}
                {p2pHealth === 'weak' && 'Weak'}
                {p2pHealth === 'broken' && 'Broken'}
              </span>
            </div>
            <p className="cc-p2p-description">
              {p2pHealth === 'stable' && 'IPv4 peers locked in. Constellation is humming.'}
              {p2pHealth === 'weak' && 'Some peers reachable, but link is fragile. Keep an eye on it.'}
              {p2pHealth === 'broken' && 'No IPv4 peers + guardian unreachable. Node is mining in isolation.'}
            </p>
          </div>
        </div>
      </div>

      {/* Routing Intelligence Dashboard */}
      <RoutingIntelligenceDashboard />

      {/* Vault Status */}
      <div className="cc-panel" style={{ marginTop: '1.5rem' }}>
        <div className="cc-panel-header">
          <h3 className="cc-panel-title">💰 VAULT STATUS</h3>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <a 
              href="http://127.0.0.1:7070/dashboard.html"
              target="_blank"
              rel="noopener noreferrer"
              className="cc-panel-link"
              style={{ textDecoration: 'none' }}
            >
              Health Dashboard →
            </a>
          </div>
        </div>
        <div className="cc-panel-body">
          <VaultStatusDashboard />
        </div>
      </div>

      {/* Middle Row - Panels */}
      <div className="cc-panels-row">
        {/* Left: Quick Miner Panel */}
        <div className="cc-panel">
          <div className="cc-panel-header">
            <h3 className="cc-panel-title">QUICK MINER CONTROLS</h3>
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              {/* Anchor Mode Toggle */}
              {constellation && (
                <button
                  onClick={() => {
                    const newAnchorState = !constellation.is_anchor;
                    alert(
                      newAnchorState
                        ? 'To enable ANCHOR mode:\n\n1. Stop Vision Node\n2. Set environment variable: P2P_IS_ANCHOR=true\n3. Forward port 7070 on your router\n4. Restart Vision Node\n\nSee ANCHOR_LEAF_GUIDE.md for details.'
                        : 'To disable ANCHOR mode:\n\n1. Stop Vision Node\n2. Remove P2P_IS_ANCHOR environment variable\n3. Restart Vision Node'
                    );
                  }}
                  className="cc-btn"
                  style={{
                    padding: '6px 12px',
                    fontSize: '0.85rem',
                    backgroundColor: constellation.is_anchor ? '#10b981' : '#6b7280',
                    border: 'none',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px'
                  }}
                  title={constellation.is_anchor ? `Anchor Mode: ${constellation.mode}` : 'Enable Anchor Mode'}
                >
                  {constellation.is_anchor ? (
                    <>
                      <span>⚓</span>
                      <span>{constellation.mode}</span>
                      {constellation.public_reachable && <span style={{ fontSize: '0.7rem' }}>✓</span>}
                    </>
                  ) : (
                    <>
                      <span>🍃</span>
                      <span>Leaf</span>
                    </>
                  )}
                </button>
              )}
              <a 
                href="http://127.0.0.1:7070/panel.html"
                target="_blank"
                rel="noopener noreferrer"
                className="cc-panel-link"
                style={{ textDecoration: 'none' }}
              >
                Full Panel →
              </a>
              <a 
                href="http://127.0.0.1:7070/dashboard.html"
                target="_blank"
                rel="noopener noreferrer"
                className="cc-panel-link"
                style={{ textDecoration: 'none' }}
              >
                Health Dashboard →
              </a>
            </div>
          </div>
          <div className="cc-panel-body">
            {!miningStatus.active ? (
              <div className="cc-miner-controls">
                <p className="cc-miner-hint">Start mining to earn LAND rewards</p>
                <div className="cc-button-group">
                  <button 
                    className="cc-btn cc-btn-primary"
                    onClick={() => handleStartMining('solo')}
                  >
                    Start Solo Mining
                  </button>
                  <button 
                    className="cc-btn cc-btn-secondary"
                    onClick={() => handleStartMining('pool')}
                  >
                    Start Pool Mining
                  </button>
                </div>
              </div>
            ) : (
              <div className="cc-miner-controls">
                <div className="cc-mining-active">
                  <div className="cc-mining-pulse"></div>
                  <p className="cc-mining-status">Mining in progress...</p>
                  <p className="cc-mining-mode">{miningStatus.mode.toUpperCase()} MODE</p>
                  <p className="cc-mining-hashrate">
                    {(miningStatus.hashrate / 1000000).toFixed(2)} MH/s
                  </p>
                </div>
                <button 
                  className="cc-btn cc-btn-danger"
                  onClick={handleStopMining}
                >
                  Stop Mining
                </button>
              </div>
            )}
          </div>
        </div>

        {/* Right: Wallet & Activity */}
        <div className="cc-panel">
          <div className="cc-panel-header">
            <h3 className="cc-panel-title">WALLET & ACTIVITY</h3>
          </div>
          <div className="cc-panel-body">
            <div className="cc-wallet-summary">
              <div className="cc-wallet-address">
                <span className="cc-label">Address</span>
                <span className="cc-address">
                  {profile?.address ? `${profile.address.substring(0, 8)}...${profile.address.substring(profile.address.length - 6)}` : 'Not connected'}
                </span>
              </div>
              <div className="cc-wallet-balances">
                <div className="cc-balance-item">
                  <span className="cc-balance-amount">{balances.LAND.toFixed(4)}</span>
                  <span className="cc-balance-token">LAND</span>
                </div>
                <div className="cc-balance-item">
                  <span className="cc-balance-amount">{balances.CASH.toFixed(2)}</span>
                  <span className="cc-balance-token">CASH</span>
                </div>
              </div>
            </div>
            
            <div className="cc-recent-activity">
              <h4 className="cc-activity-title">Recent Activity</h4>
              <div className="cc-activity-list">
                {events.slice(0, 5).map(event => (
                  <div key={event.id} className="cc-activity-item">
                    <span className="cc-activity-time">{event.time}</span>
                    <span className="cc-activity-message">{event.message}</span>
                  </div>
                ))}
                {events.length === 0 && (
                  <p className="cc-activity-empty">No recent activity</p>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Node Approval Section */}
      <div className="cc-panel" style={{ marginTop: '1.5rem' }}>
        <div className="cc-panel-header">
          <h3 className="cc-panel-title">🔐 NODE APPROVAL</h3>
        </div>
        <div className="cc-panel-body">
          <p style={{ color: 'var(--text-secondary)', marginBottom: '1rem' }}>
            Approve this node to enable mining with your wallet
          </p>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ color: 'var(--text-secondary)' }}>Approval Status:</span>
              <span style={{ 
                color: approvalStatus?.approved ? 'var(--accent-green)' : '#fbbf24', 
                fontWeight: 600 
              }}>
                {approvalStatus?.approved ? '✅ Approved' : '⚠️ Not Approved'}
              </span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ color: 'var(--text-secondary)' }}>Wallet:</span>
              <span style={{ 
                color: 'var(--accent-green)', 
                fontFamily: "'Courier New', monospace",
                fontSize: '0.9rem'
              }}>
                {approvalStatus?.wallet_address || 'Not configured'}
              </span>
            </div>
            {!approvalStatus?.approved && constellation?.mining_blocked_reason && (
              <div style={{ 
                color: '#fbbf24', 
                fontSize: '0.9rem',
                padding: '0.75rem',
                background: 'rgba(245, 158, 11, 0.1)',
                borderRadius: '0.5rem',
                border: '1px solid rgba(245, 158, 11, 0.3)'
              }}>
                ⚠️ {constellation.mining_blocked_reason}
              </div>
            )}
            <button 
              className="cc-btn cc-btn-primary"
              onClick={handleApproveNode}
              disabled={approving || approvalStatus?.approved}
              style={{ 
                minWidth: '200px',
                opacity: (approving || approvalStatus?.approved) ? 0.6 : 1,
                cursor: (approving || approvalStatus?.approved) ? 'not-allowed' : 'pointer'
              }}
            >
              {approving ? '⏳ Approving...' : approvalStatus?.approved ? '✅ Already Approved' : '✅ Approve Node With Wallet'}
            </button>
          </div>
        </div>
      </div>

      {/* Mining Stats */}
      {miningStats && (
        <div className="cc-panel" style={{ marginTop: '1.5rem' }}>
          <div className="cc-panel-header">
            <h3 className="cc-panel-title">📊 MINING STATISTICS</h3>
          </div>
          <div className="cc-panel-body">
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1rem' }}>
              <div className="cc-stat">
                <span className="cc-stat-label">Blocks Found</span>
                <span className="cc-stat-value">{miningStats.blocks_found || 0}</span>
              </div>
              <div className="cc-stat">
                <span className="cc-stat-label">Blocks Accepted</span>
                <span className="cc-stat-value">{miningStats.blocks_accepted || 0}</span>
              </div>
              <div className="cc-stat">
                <span className="cc-stat-label">Blocks Rejected</span>
                <span className="cc-stat-value" style={{ color: miningStats.blocks_rejected > 0 ? '#ef4444' : 'inherit' }}>
                  {miningStats.blocks_rejected || 0}
                </span>
              </div>
              <div className="cc-stat">
                <span className="cc-stat-label">Average Hashrate</span>
                <span className="cc-stat-value">
                  {miningStats.average_hashrate ? `${(miningStats.average_hashrate / 1000000).toFixed(2)} MH/s` : 'N/A'}
                </span>
              </div>
              <div className="cc-stat">
                <span className="cc-stat-label">Total Hashes</span>
                <span className="cc-stat-value">{miningStats.total_hashes?.toLocaleString() || 0}</span>
              </div>
              <div className="cc-stat">
                <span className="cc-stat-label">Uptime</span>
                <span className="cc-stat-value">
                  {miningStats.uptime_seconds ? `${Math.floor(miningStats.uptime_seconds / 60)} min` : 'N/A'}
                </span>
              </div>
            </div>
            
            {/* Hashrate History Chart */}
            {hashrateHistory.length > 0 && (
              <div style={{ marginTop: '1.5rem' }}>
                <h4 style={{ marginBottom: '0.5rem', fontSize: '0.9rem', color: 'var(--text-secondary)' }}>
                  Hashrate History
                </h4>
                <div style={{ 
                  display: 'flex', 
                  alignItems: 'flex-end', 
                  gap: '2px', 
                  height: '100px',
                  background: 'rgba(0,0,0,0.2)',
                  padding: '0.5rem',
                  borderRadius: '0.5rem'
                }}>
                  {hashrateHistory.map((rate, i) => {
                    const maxRate = Math.max(...hashrateHistory)
                    const height = maxRate > 0 ? (rate / maxRate) * 100 : 0
                    return (
                      <div 
                        key={i} 
                        style={{ 
                          flex: 1,
                          height: `${height}%`,
                          background: 'linear-gradient(to top, var(--accent-green), var(--accent-cyan))',
                          borderRadius: '2px',
                          minHeight: '2px'
                        }}
                        title={`${(rate / 1000000).toFixed(2)} MH/s`}
                      />
                    )
                  })}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Recent Blocks */}
      {miningStats && miningStats.blocks_found > 0 && (
        <div className="cc-panel" style={{ marginTop: '1.5rem' }}>
          <div className="cc-panel-header">
            <h3 className="cc-panel-title">🎯 RECENT BLOCKS</h3>
          </div>
          <div className="cc-panel-body">
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
              <div style={{ 
                padding: '1rem', 
                background: 'rgba(34, 197, 94, 0.1)',
                borderRadius: '0.5rem',
                border: '1px solid rgba(34, 197, 94, 0.3)'
              }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.5rem' }}>
                  <span style={{ color: 'var(--accent-green)', fontWeight: 600 }}>
                    ✨ {miningStats.blocks_found} block{miningStats.blocks_found !== 1 ? 's' : ''} found
                  </span>
                  <span style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>
                    {miningStats.blocks_accepted} accepted
                  </span>
                </div>
                <div style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
                  Keep mining to find more blocks and earn LAND rewards!
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Peer Information */}
      <div className="cc-panel" style={{ marginTop: '1.5rem' }}>
        <div className="cc-panel-header">
          <h3 className="cc-panel-title">🌐 NETWORK PEERS</h3>
        </div>
        <div className="cc-panel-body">
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(150px, 1fr))', gap: '1rem' }}>
            <div className="cc-stat">
              <span className="cc-stat-label">Connected</span>
              <span className="cc-stat-value">{constellation?.connected_peers || 0}</span>
            </div>
            <div className="cc-stat">
              <span className="cc-stat-label">Total Known</span>
              <span className="cc-stat-value">{constellation?.total_known_peers || 0}</span>
            </div>
            <div className="cc-stat">
              <span className="cc-stat-label">Avg Latency</span>
              <span className="cc-stat-value">
                {constellation?.avg_peer_latency_ms ? `${constellation.avg_peer_latency_ms}ms` : 'N/A'}
              </span>
            </div>
            <div className="cc-stat">
              <span className="cc-stat-label">P2P Health</span>
              <span className="cc-stat-value" style={{
                color: constellation?.p2p_health === 'isolated' ? '#dc2626' :
                       constellation?.p2p_health === 'weak' ? '#f59e0b' :
                       constellation?.p2p_health === 'immortal' ? '#3b82f6' :
                       'var(--accent-green)'
              }}>
                {constellation?.p2p_health?.toUpperCase() || 'UNKNOWN'}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Bottom Row - Live Events */}
      <div className="cc-events-panel">
        <div className="cc-panel-header">
          <h3 className="cc-panel-title">LIVE EVENT STREAM</h3>
          <button 
            className="cc-clear-button"
            onClick={() => setEvents([])}
          >
            Clear
          </button>
        </div>
        <div className="cc-events-body">
          {events.map(event => (
            <div key={event.id} className={`cc-event-item cc-event-${event.type}`}>
              <span className="cc-event-time">[{event.time}]</span>
              <span className="cc-event-type">{event.type.toUpperCase()}</span>
              <span className="cc-event-message">{event.message}</span>
            </div>
          ))}
          {events.length === 0 && (
            <div className="cc-events-empty">
              <p>Monitoring system events...</p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
