import { useState, useEffect } from 'react'
import './MiningControls.css'

type MiningMode = 'Solo' | 'HostPool' | 'JoinPool' | 'Farm'
type MiningProfile = 'laptop' | 'balanced' | 'beast'

interface MiningControlsProps {
  onStatusChange?: (active: boolean) => void
}

export default function MiningControls({ onStatusChange }: MiningControlsProps) {
  const [_miningActive, setMiningActive] = useState(false)
  const [fansActive, setFansActive] = useState(false)
  const [miningMode, setMiningMode] = useState<MiningMode>('Solo')
  const [miningProfile, setMiningProfile] = useState<MiningProfile>('balanced')
  const [miningThreads, setMiningThreads] = useState('')
  const [simdBatchSize, setSimdBatchSize] = useState('4')
  const [currentThreads, setCurrentThreads] = useState('--')
  const [maxThreads] = useState(navigator.hardwareConcurrency?.toString() || '--')
  const [currentHashrate, setCurrentHashrate] = useState('0 H/s')
  const [miningStatus, setMiningStatus] = useState('--')
  const [miningBlockedReason, setMiningBlockedReason] = useState<string | null>(null)
  
  // Pool settings
  const [poolName, setPoolName] = useState('')
  const [poolPort, setPoolPort] = useState('7072')
  const [poolFee, setPoolFee] = useState('1.5')
  const [poolActive, setPoolActive] = useState(false)
  const [poolUrl, setPoolUrl] = useState('')
  
  // Join pool settings
  const [workerName, setWorkerName] = useState('')
  const [joinPoolUrl, setJoinPoolUrl] = useState('')

  const modeDescriptions: Record<MiningMode, string> = {
    Solo: 'Solo: Mine independently and receive full block rewards',
    HostPool: 'Host Pool: Create a mining pool and share rewards with workers',
    JoinPool: 'Join Pool: Connect to an existing pool and share rewards',
    Farm: 'Farm: Manage multiple mining rigs from one dashboard'
  }

  useEffect(() => {
    // Fetch initial mining status
    fetchMiningStatus()
    fetchConstellationStatus()
    const interval = setInterval(() => {
      fetchMiningStatus()
      fetchConstellationStatus()
    }, 5000)
    return () => clearInterval(interval)
  }, [])

  const fetchMiningStatus = async () => {
    try {
      const response = await fetch('http://127.0.0.1:7070/api/miner/status')
      if (response.ok) {
        const data = await response.json()
        const isEnabled = data.enabled || false
        setMiningActive(isEnabled)
        setFansActive(isEnabled) // Sync fansActive with backend status
        setCurrentThreads(data.threads?.toString() || '--')
        setCurrentHashrate(data.average_hashrate ? `${(data.average_hashrate / 1000000).toFixed(2)} MH/s` : '0 H/s')
        setMiningStatus(isEnabled ? 'Active' : 'Idle')
        onStatusChange?.(isEnabled)
      }
    } catch (error) {
      console.debug('Failed to fetch mining status:', error)
    }
  }

  const fetchConstellationStatus = async () => {
    try {
      const response = await fetch('http://127.0.0.1:7070/api/constellation/status')
      if (response.ok) {
        const data = await response.json()
        setMiningBlockedReason(data.mining_blocked_reason || null)
      }
    } catch (error) {
      console.debug('Failed to fetch constellation status:', error)
    }
  }

  const handleMakeFansGoBrr = async () => {
    if (fansActive) return
    
    try {
      setFansActive(true)
      const threads = miningThreads ? parseInt(miningThreads) : (navigator.hardwareConcurrency || 4)
      const response = await fetch('http://127.0.0.1:7070/api/miner/start', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          threads: threads
        })
      })
      
      if (!response.ok) throw new Error('Failed to start mining')
      
      await fetchMiningStatus()
    } catch (error) {
      console.error('Failed to start mining:', error)
      setFansActive(false)
    }
  }

  const handleStopFans = async () => {
    try {
      const response = await fetch('http://127.0.0.1:7070/api/miner/stop', {
        method: 'POST'
      })
      
      if (!response.ok) throw new Error('Failed to stop mining')
      
      setFansActive(false)
      await fetchMiningStatus()
    } catch (error) {
      console.error('Failed to stop mining:', error)
    }
  }

  const handleApplySettings = async () => {
    try {
      const response = await fetch('http://127.0.0.1:7070/api/miner/update', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          threads: miningThreads ? parseInt(miningThreads) : undefined,
          simd_batch_size: parseInt(simdBatchSize),
          profile: miningProfile
        })
      })
      
      if (!response.ok) throw new Error('Failed to update mining settings')
      
      await fetchMiningStatus()
    } catch (error) {
      console.error('Failed to update mining settings:', error)
    }
  }



  const handleStartPool = async () => {
    try {
      const response = await fetch('http://127.0.0.1:7070/api/pool/start', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: poolName,
          port: parseInt(poolPort),
          fee: parseFloat(poolFee)
        })
      })
      
      if (response.ok) {
        const data = await response.json()
        setPoolActive(true)
        setPoolUrl(data.pool_url || `http://localhost:${poolPort}`)
      }
    } catch (error) {
      console.error('Failed to start pool:', error)
    }
  }

  const handleStopPool = async () => {
    try {
      await fetch('http://127.0.0.1:7070/api/pool/stop', {
        method: 'POST'
      })
      setPoolActive(false)
    } catch (error) {
      console.error('Failed to stop pool:', error)
    }
  }

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
  }

  return (
    <div className="mining-controls-container">
      {/* Mining Status Banner */}
      <div className="mining-banner">
        <div className="mining-banner-header">
          <span className="mining-icon">⛏️</span>
          <div>
            <div className="mining-banner-title">Mining Control</div>
            <div className="mining-banner-subtitle">Mining runs when the node is synced and healthy</div>
          </div>
        </div>
        
        <div className="mining-info-box">
          <div className="mining-info-title">💡 How it works:</div>
          <ul className="mining-info-list">
            <li><strong>Traditional Proof-of-Work mining</strong></li>
            <li>Every block requires computational work</li>
            <li>Mining rewards distributed to block miners</li>
            <li>Link wallet + sync node + mine blocks = earn!</li>
          </ul>
        </div>

        {/* Fans Go Brr Section */}
        <div className="fans-section">
          <div className="fans-header">
            <span style={{ fontSize: '1.2rem' }}>🔥</span>
            <strong>Want to hear your fans?</strong>
          </div>
          <p className="fans-description">
            Start active mining to dedicate CPU power to finding new blocks.
            Higher hashrate increases your chances of mining blocks and earning rewards.
          </p>
          
          {/* Show blocking reason if present */}
          {miningBlockedReason && !fansActive && (
            <div style={{
              padding: '0.75rem',
              marginBottom: '1rem',
              background: 'rgba(251, 191, 36, 0.1)',
              border: '1px solid rgba(251, 191, 36, 0.3)',
              borderRadius: '8px',
              color: '#fbbf24',
              fontSize: '0.9rem',
              fontWeight: '500'
            }}>
              {miningBlockedReason}
            </div>
          )}
          
          {!fansActive ? (
            <button 
              className="btn-fans-brr"
              onClick={handleMakeFansGoBrr}
              disabled={!!miningBlockedReason}
              style={{
                opacity: miningBlockedReason ? 0.5 : 1,
                cursor: miningBlockedReason ? 'not-allowed' : 'pointer'
              }}
              title={miningBlockedReason || 'Start mining'}
            >
              <span>💨</span>
              <span>{miningBlockedReason ? 'Mining Blocked' : 'Make Fans Go BRRRR!'}</span>
            </button>
          ) : (
            <button 
              className="btn-stop-fans"
              onClick={handleStopFans}
            >
              <span>🛑</span>
              <span>Stop Mining (fans will quiet down)</span>
            </button>
          )}
          
          <div className="fan-status">
            Status: <strong>{fansActive ? 'Mining Active 🔥' : 'Quiet (no mining)'}</strong>
          </div>
        </div>
      </div>

      {/* Mining Mode Selector */}
      <div className="mining-section">
        <label className="section-label">Mining Mode:</label>
        <div className="mode-buttons">
          {(['Solo', 'HostPool', 'JoinPool', 'Farm'] as MiningMode[]).map(mode => {
            const isComingSoon = mode !== 'Solo'
            return (
              <button
                key={mode}
                className={`mode-btn ${miningMode === mode ? 'active' : ''} ${isComingSoon ? 'coming-soon' : ''}`}
                onClick={() => !isComingSoon && setMiningMode(mode)}
                disabled={isComingSoon}
                title={isComingSoon ? 'Coming Soon' : ''}
              >
                {mode === 'Solo' && '⛏️'} {mode === 'HostPool' && '🏊'} 
                {mode === 'JoinPool' && '🤝'} {mode === 'Farm' && '🚜'} 
                {' '}{mode.replace(/([A-Z])/g, ' $1').trim()}
              </button>
            )
          })}
        </div>
        <div className="mode-description">
          {modeDescriptions[miningMode]}
        </div>
      </div>

      {/* Performance Tuning */}
      <div className="performance-section">
        <h3 className="section-title">🎯 Performance Tuning</h3>
        
        <div className="form-group">
          <label>Mining Profile:</label>
          <select 
            value={miningProfile}
            onChange={(e) => setMiningProfile(e.target.value as MiningProfile)}
            className="mining-select"
          >
            <option value="laptop">💻 Laptop (low impact)</option>
            <option value="balanced">⚖️ Balanced</option>
            <option value="beast">🔥 Beast Mode (all cores)</option>
          </select>
          <small>Choose how aggressive mining should be</small>
        </div>

        <div className="form-group">
          <label>Mining Threads (override):</label>
          <input
            type="number"
            value={miningThreads}
            onChange={(e) => setMiningThreads(e.target.value)}
            placeholder="0 = auto (based on profile)"
            className="mining-input"
            min="0"
          />
          <small>Leave 0 or empty to auto-detect from CPU and profile</small>
        </div>

        <div className="form-group">
          <label>SIMD Batch Size:</label>
          <input
            type="number"
            value={simdBatchSize}
            onChange={(e) => setSimdBatchSize(e.target.value)}
            className="mining-input"
            min="1"
            max="1024"
          />
          <small>How many nonces to process per inner loop (1-1024)</small>
        </div>

        {/* Apply Settings Button */}
        {fansActive && (
          <button 
            className="btn-apply-settings"
            onClick={handleApplySettings}
            style={{ 
              width: '100%', 
              marginTop: '1rem',
              padding: '0.75rem',
              background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
              color: 'white',
              border: 'none',
              borderRadius: '8px',
              cursor: 'pointer',
              fontWeight: '600',
              fontSize: '0.95rem',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '0.5rem'
            }}
          >
            <span>✓</span>
            Apply Settings to Active Miner
          </button>
        )}

        <div className="mining-stats">
          <div>
            Mining Threads: <strong>{currentThreads}</strong> / {maxThreads}
          </div>
          <div>
            Status: <strong>{miningStatus}</strong>
          </div>
        </div>

        <div className="mining-hashrate">
          Current Hashrate: <strong>{currentHashrate}</strong>
        </div>
      </div>

      {/* Pool Configuration (for HostPool mode) */}
      {miningMode === 'HostPool' && (
        <div className="pool-section">
          <h3 className="section-title">🏊 Host Pool Configuration</h3>
          
          <div className="fee-notice">
            <strong>ℹ️ Fee Structure:</strong>
            <div style={{ marginTop: '0.25rem' }}>
              • Foundation: 1% (supports development)<br />
              • Pool Fee: {poolFee}% (configurable)
            </div>
          </div>

          <div className="form-group">
            <label>Pool Name:</label>
            <input
              type="text"
              value={poolName}
              onChange={(e) => setPoolName(e.target.value)}
              placeholder="My Awesome Pool"
              className="mining-input"
              maxLength={50}
            />
          </div>

          <div className="form-group">
            <label>Pool Port:</label>
            <div className="port-buttons">
              <button
                className={`port-btn ${poolPort === '7072' ? 'active' : ''}`}
                onClick={() => setPoolPort('7072')}
              >
                7072
              </button>
              <button
                className={`port-btn ${poolPort === '8082' ? 'active' : ''}`}
                onClick={() => setPoolPort('8082')}
              >
                8082
              </button>
            </div>
          </div>

          <div className="form-group">
            <label>Pool Fee Percentage:</label>
            <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
              <input
                type="number"
                value={poolFee}
                onChange={(e) => setPoolFee(e.target.value)}
                className="mining-input"
                min="0"
                max="10"
                step="0.1"
                style={{ flex: 1 }}
              />
              <span>%</span>
            </div>
          </div>

          <div style={{ display: 'flex', gap: '1rem' }}>
            {!poolActive ? (
              <button className="btn-primary" onClick={handleStartPool} style={{ flex: 1 }}>
                <span>▶️</span> Start Pool
              </button>
            ) : (
              <button className="btn-secondary" onClick={handleStopPool} style={{ flex: 1 }}>
                <span>⏹️</span> Stop Pool
              </button>
            )}
          </div>

          <div className="pool-status">
            Pool Status: <strong>{poolActive ? 'Running' : 'Stopped'}</strong>
          </div>

          {poolActive && poolUrl && (
            <div className="pool-url-display">
              <div className="pool-url-header">
                🌐 POOL ADDRESS
              </div>
              <div className="pool-url-subtitle">
                Share this with miners to join your pool
              </div>
              <div style={{ display: 'flex', gap: '0.75rem', marginTop: '0.75rem' }}>
                <input
                  type="text"
                  value={poolUrl}
                  readOnly
                  className="pool-url-input"
                />
                <button
                  className="btn-copy"
                  onClick={() => copyToClipboard(poolUrl)}
                >
                  📋 Copy
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Join Pool Configuration */}
      {miningMode === 'JoinPool' && (
        <div className="pool-section">
          <h3 className="section-title">🤝 Join Pool Configuration</h3>

          <div className="form-group">
            <label>Worker Name:</label>
            <input
              type="text"
              value={workerName}
              onChange={(e) => setWorkerName(e.target.value)}
              placeholder="My Mining Rig"
              className="mining-input"
              maxLength={32}
            />
          </div>

          <div className="form-group">
            <label>Pool URL:</label>
            <input
              type="text"
              value={joinPoolUrl}
              onChange={(e) => setJoinPoolUrl(e.target.value)}
              placeholder="http://pool-host:7072"
              className="mining-input"
            />
            <small>Paste the pool URL provided by the pool operator</small>
          </div>
        </div>
      )}
    </div>
  )
}
