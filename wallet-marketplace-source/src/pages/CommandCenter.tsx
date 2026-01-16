import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useWalletStore } from '../state/wallet'
import { useNodeStatus } from '../hooks/useNodeStatus'
import { useMiningStatus } from '../hooks/useMiningStatus'
import { useGuardianStatus } from '../hooks/useGuardianStatus'
import { useConstellationStatus, computeP2PHealth } from '../hooks/useConstellationStatus'
import RoutingIntelligenceDashboard from '../components/RoutingIntelligenceDashboard'
import { VaultStatusDashboard } from '../components/VaultStatusDashboard'
import MiningControls from '../components/MiningControls'
import VisionGlobe from '../components/VisionGlobe'
import GovernancePanel from '../components/GovernancePanel'
import LogsPanel from '../components/LogsPanel'
import '../styles/command-center.css'
import '../styles/routing-intelligence.css'

interface ApprovalStatus {
  approved: boolean
  wallet_address: string | null
  node_id: string
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
  
  const [events, setEvents] = useState<Array<{id: string, time: string, type: string, message: string}>>([])
  
  // Node approval state
  const [approvalStatus, setApprovalStatus] = useState<ApprovalStatus | null>(null)
  const [approving, setApproving] = useState(false)
  const [minerWalletAddress, setMinerWalletAddress] = useState<string>('')
  
  // Hashrate history
  const [hashrateHistory, setHashrateHistory] = useState<number[]>([])
  
  // Mining stats
  const [miningStats, setMiningStats] = useState<any>(null)
  
  // Blockchain stats
  const [blockHeight, setBlockHeight] = useState<number>(0)
  const [mempoolSize, setMempoolSize] = useState<number>(0)
  const [avgBlockTime, setAvgBlockTime] = useState<string>('0ms')
  const [blocksFound, setBlocksFound] = useState<number>(0)

  // Fetch approval status and miner wallet
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
      
      // Also fetch the actual miner wallet address
      try {
        const minerResponse = await fetch('http://127.0.0.1:7070/api/miner/wallet')
        if (minerResponse.ok) {
          const minerData = await minerResponse.json()
          setMinerWalletAddress(minerData.wallet || '')
        }
      } catch (err) {
        console.debug('Miner wallet fetch failed:', err)
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
  
  // Fetch blockchain stats
  useEffect(() => {
    const fetchBlockchainStats = async () => {
      try {
        // Fetch node status for block height and mempool
        const statusResponse = await fetch('http://127.0.0.1:7070/api/status')
        if (statusResponse.ok) {
          const statusData = await statusResponse.json()
          setBlockHeight(statusData.height || 0)
          setMempoolSize(statusData.mempool || 0)
        }
        
        // Fetch mining stats for blocks found and avg time
        const minerResponse = await fetch('http://127.0.0.1:7070/api/miner/stats')
        if (minerResponse.ok) {
          const minerData = await minerResponse.json()
          setBlocksFound(minerData.blocks_found || 0)
          if (minerData.average_time_ms) {
            setAvgBlockTime(`${minerData.average_time_ms}ms`)
          }
        }
      } catch (err) {
        console.debug('Blockchain stats fetch failed:', err)
      }
    }
    
    fetchBlockchainStats()
    const interval = setInterval(fetchBlockchainStats, 5000)
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
  
  const handleApproveNode = async () => {
    if (approving) return
    setApproving(true)
    
    try {
      // Get wallet address from the wallet store
      const walletAddress = profile?.address
      
      if (!walletAddress) {
        alert('No wallet found. Please create a wallet first by going to the Wallet page.')
        return
      }
      
      // Check if already approved - if so, just configure miner wallet
      if (approvalStatus?.approved) {
        try {
          const minerWalletResponse = await fetch('http://127.0.0.1:7070/api/miner/wallet', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ wallet: walletAddress })
          })
          
          if (minerWalletResponse.ok) {
            alert('✅ Mining rewards wallet configured!\n\n💰 All mining rewards will now be sent to your wallet')
            
            // Refresh miner wallet address
            const minerResponse = await fetch('http://127.0.0.1:7070/api/miner/wallet')
            if (minerResponse.ok) {
              const minerData = await minerResponse.json()
              setMinerWalletAddress(minerData.wallet || '')
            }
          } else {
            alert('❌ Failed to configure mining rewards wallet')
          }
        } catch (e) {
          alert('❌ Failed to configure mining rewards wallet')
        }
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
        // Step 5: Configure miner wallet address to receive rewards
        try {
          const minerWalletResponse = await fetch('http://127.0.0.1:7070/api/miner/wallet', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ wallet: walletAddress })
          })
          
          if (minerWalletResponse.ok) {
            alert('✅ Wallet linked successfully!\n\n🔗 Node approved for mining\n💰 Wallet configured to receive LAND rewards')
          } else {
            alert('✅ Node approved, but failed to configure mining rewards wallet.\n\nPlease visit http://127.0.0.1:7070/panel.html to manually link your wallet address.')
          }
        } catch (e) {
          alert('✅ Node approved, but failed to configure mining rewards wallet.\n\nPlease visit http://127.0.0.1:7070/panel.html to manually link your wallet address.')
        }
        
        // Refresh approval status and miner wallet
        const response = await fetch('http://127.0.0.1:7070/api/node/approval/status')
        if (response.ok) {
          const data = await response.json()
          setApprovalStatus(data)
        }
        
        // Refresh miner wallet address
        const minerResponse = await fetch('http://127.0.0.1:7070/api/miner/wallet')
        if (minerResponse.ok) {
          const minerData = await minerResponse.json()
          setMinerWalletAddress(minerData.wallet || '')
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
      {/* Animated Earth Background */}
      <div style={{
        position: 'fixed',
        top: '50%',
        left: '50%',
        transform: 'translate(-50%, -50%)',
        width: '1400px',
        height: '1400px',
        opacity: 0.08,
        pointerEvents: 'none',
        zIndex: 0,
        filter: 'blur(2px)'
      }}>
        <VisionGlobe />
      </div>

      {/* Content wrapper with z-index */}
      <div style={{ position: 'relative', zIndex: 1 }}>
        {/* Page Header */}
        <div style={{ 
          marginBottom: '2.5rem',
          padding: '3rem 2rem',
          background: 'linear-gradient(135deg, rgba(17, 24, 39, 0.95) 0%, rgba(30, 27, 75, 0.95) 50%, rgba(17, 24, 39, 0.95) 100%)',
          borderRadius: '1.5rem',
          border: '1px solid rgba(138, 92, 255, 0.2)',
          boxShadow: '0 20px 60px rgba(0, 0, 0, 0.5), 0 0 100px rgba(138, 92, 255, 0.1), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
          textAlign: 'center',
          position: 'relative',
          overflow: 'hidden'
        }}>
          {/* Subtle animated gradient background */}
          <div style={{
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: 'radial-gradient(circle at 50% 50%, rgba(138, 92, 255, 0.15) 0%, transparent 70%)',
            opacity: 0.6,
            zIndex: 0
          }} />
          
          <div style={{ position: 'relative', zIndex: 1 }}>
            <h1 style={{ 
              fontSize: '4rem', 
              fontWeight: 900, 
              background: 'linear-gradient(135deg, #c4b5fd 0%, #a78bfa 25%, #8b5cf6 50%, #7c3aed 75%, #6d28d9 100%)',
              WebkitBackgroundClip: 'text',
              WebkitTextFillColor: 'transparent',
              backgroundClip: 'text',
              marginBottom: '1rem',
              letterSpacing: '-0.03em',
              textTransform: 'uppercase',
              filter: 'drop-shadow(0 0 30px rgba(139, 92, 246, 0.5))'
            }}>
              ⚡ Command Center
            </h1>
            <div style={{
              height: '3px',
              width: '120px',
              background: 'linear-gradient(90deg, transparent 0%, #8b5cf6 50%, transparent 100%)',
              margin: '0 auto 1.5rem',
              borderRadius: '3px',
              boxShadow: '0 0 20px rgba(139, 92, 246, 0.6)'
            }} />
            <p style={{ 
              fontSize: '1.15rem', 
              color: 'rgba(229, 231, 235, 0.9)', 
              lineHeight: '1.8',
              maxWidth: '700px',
              margin: '0 auto',
              fontWeight: 400,
              textShadow: '0 2px 10px rgba(0, 0, 0, 0.3)'
            }}>
              Your central hub for node operations, mining controls, wallet management, and real-time network monitoring. 
              Link your wallet, configure mining, and track your node's performance all in one place.
            </p>
          </div>
        </div>

        {/* Blockchain Stats - Above Node Approval */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', gap: '1rem', marginBottom: '1.5rem' }}>
        <div className="cc-panel" style={{ margin: 0 }}>
          <div className="cc-panel-header" style={{ padding: '0.75rem 1rem' }}>
            <span style={{ fontSize: '1.5rem', marginRight: '0.5rem' }}>📦</span>
            <span style={{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-secondary)' }}>BLOCK HEIGHT</span>
          </div>
          <div className="cc-panel-body" style={{ padding: '1rem', textAlign: 'center' }}>
            <div style={{ fontSize: '2rem', fontWeight: 700, color: 'var(--accent-purple)', marginBottom: '0.25rem' }}>
              {blockHeight.toLocaleString()}
            </div>
            <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>Current chain height</div>
          </div>
        </div>

        <div className="cc-panel" style={{ margin: 0 }}>
          <div className="cc-panel-header" style={{ padding: '0.75rem 1rem' }}>
            <span style={{ fontSize: '1.5rem', marginRight: '0.5rem' }}>⏳</span>
            <span style={{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-secondary)' }}>MEMPOOL SIZE</span>
          </div>
          <div className="cc-panel-body" style={{ padding: '1rem', textAlign: 'center' }}>
            <div style={{ fontSize: '2rem', fontWeight: 700, color: 'var(--accent-cyan)', marginBottom: '0.25rem' }}>
              {mempoolSize}
            </div>
            <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>Pending transactions</div>
          </div>
        </div>

        <div className="cc-panel" style={{ margin: 0 }}>
          <div className="cc-panel-header" style={{ padding: '0.75rem 1rem' }}>
            <span style={{ fontSize: '1.5rem', marginRight: '0.5rem' }}>⚡</span>
            <span style={{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-secondary)' }}>AVG BLOCK TIME</span>
          </div>
          <div className="cc-panel-body" style={{ padding: '1rem', textAlign: 'center' }}>
            <div style={{ fontSize: '2rem', fontWeight: 700, color: 'var(--accent-green)', marginBottom: '0.25rem' }}>
              {avgBlockTime}
            </div>
            <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>Average mining duration</div>
          </div>
        </div>

        <div className="cc-panel" style={{ margin: 0 }}>
          <div className="cc-panel-header" style={{ padding: '0.75rem 1rem' }}>
            <span style={{ fontSize: '1.5rem', marginRight: '0.5rem' }}>🎯</span>
            <span style={{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-secondary)' }}>BLOCKS FOUND</span>
          </div>
          <div className="cc-panel-body" style={{ padding: '1rem', textAlign: 'center' }}>
            <div style={{ fontSize: '2rem', fontWeight: 700, color: 'var(--accent-gold)', marginBottom: '0.25rem' }}>
              {blocksFound}
            </div>
            <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>Total blocks produced</div>
          </div>
        </div>
      </div>

      {/* Node Approval Section - Top */}
      <div className="cc-panel" style={{ marginBottom: '1.5rem' }}>
        <div className="cc-panel-header">
          <h3 className="cc-panel-title">🔐 NODE APPROVAL & WALLET LINK</h3>
        </div>
        <div className="cc-panel-body">
          <p style={{ color: 'var(--text-secondary)', marginBottom: '1rem' }}>
            Link your wallet to this node to enable mining and receive LAND rewards
          </p>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '0.75rem', background: 'rgba(138, 92, 255, 0.1)', borderRadius: '0.5rem', border: '1px solid rgba(162, 145, 255, 0.3)' }}>
              <span style={{ color: 'var(--text-secondary)', fontWeight: 600 }}>Approval Status:</span>
              <span style={{ 
                color: approvalStatus?.approved ? 'var(--accent-green)' : '#fbbf24', 
                fontWeight: 600,
                fontSize: '1rem'
              }}>
                {approvalStatus?.approved ? '✅ Approved & Mining Enabled' : '⚠️ Not Approved - Mining Disabled'}
              </span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '0.75rem', background: 'rgba(12, 8, 33, 0.6)', borderRadius: '0.5rem', border: '1px solid var(--border-color)' }}>
              <span style={{ color: 'var(--text-secondary)', fontWeight: 600 }}>Mining Rewards Wallet:</span>
              <span style={{ 
                color: minerWalletAddress && minerWalletAddress !== 'pow_miner' ? 'var(--accent-green)' : '#fbbf24', 
                fontFamily: "'Courier New', monospace",
                fontSize: '0.85rem',
                fontWeight: 600
              }}>
                {minerWalletAddress && minerWalletAddress !== 'pow_miner' ? 
                  `${minerWalletAddress.slice(0, 8)}...${minerWalletAddress.slice(-6)}` 
                  : 'Not configured'}
              </span>
            </div>
            {approvalStatus?.approved && (!minerWalletAddress || minerWalletAddress === 'pow_miner') && (
              <div style={{ 
                color: '#ef4444', 
                fontSize: '0.9rem',
                padding: '0.75rem',
                background: 'rgba(239, 68, 68, 0.1)',
                borderRadius: '0.5rem',
                border: '1px solid rgba(239, 68, 68, 0.3)'
              }}>
                ⚠️ Node is approved but mining rewards wallet is not configured! Click the button below to configure it now.
              </div>
            )}
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
              disabled={approving || (approvalStatus?.approved && !!minerWalletAddress && minerWalletAddress !== 'pow_miner')}
              style={{ 
                minWidth: '250px',
                opacity: (approving || (approvalStatus?.approved && !!minerWalletAddress && minerWalletAddress !== 'pow_miner')) ? 0.6 : 1,
                cursor: (approving || (approvalStatus?.approved && !!minerWalletAddress && minerWalletAddress !== 'pow_miner')) ? 'not-allowed' : 'pointer',
                fontSize: '1rem',
                padding: '0.875rem 1.5rem'
              }}
            >
              {approving ? '⏳ Approving...' : 
               (approvalStatus?.approved && !!minerWalletAddress && minerWalletAddress !== 'pow_miner') ? '✅ Wallet Already Linked' : 
               approvalStatus?.approved ? '💰 Configure Mining Rewards Wallet' :
               '🔗 Link Wallet & Approve Node'}
            </button>
          </div>
        </div>
      </div>

      {/* Mining Controls */}
      <MiningControls />

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
              <span className="cc-stat-value">{(nodeStatus.height || 0).toLocaleString()}</span>
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
              <span className="cc-stat-value cc-balance">{(balances?.LAND || 0).toFixed(4)}</span>
            </div>
            <div className="cc-stat">
              <span className="cc-stat-label">CASH</span>
              <span className="cc-stat-value cc-balance">{(balances?.CASH || 0).toFixed(2)}</span>
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
        {/* Left: Anchor Mode & Links */}
        <div className="cc-panel">
          <div className="cc-panel-header">
            <h3 className="cc-panel-title">NODE CONFIGURATION</h3>
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
            </div>
          </div>
          <div className="cc-panel-body">
            {/* Mining controls now integrated above - no external links needed */}
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
                  <span className="cc-balance-amount">{(balances?.LAND || 0).toFixed(4)}</span>
                  <span className="cc-balance-token">LAND</span>
                </div>
                <div className="cc-balance-item">
                  <span className="cc-balance-amount">{(balances?.CASH || 0).toFixed(2)}</span>
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

      {/* Live Logs Panel */}
      <div style={{ marginBottom: '1.5rem', height: '600px' }}>
        <LogsPanel />
      </div>

      {/* Governance Panel */}
      <GovernancePanel />
      
      </div> {/* End content wrapper */}
    </div>
  )
}
