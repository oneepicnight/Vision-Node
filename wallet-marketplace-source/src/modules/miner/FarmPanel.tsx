import { useState, useEffect } from 'react'
import FarmHandModal from './FarmHandModal'
import MiningEndpointsModal from './MiningEndpointsModal'

interface FarmRig {
  rig_id: string
  name: string
  os: string
  cpu_threads: number
  status: string
  hashrate: number
  last_heartbeat: number
  profile?: string | null
  endpoint_mode?: string | null
}

interface MiningInfo {
  solo_enabled: boolean
  pool_enabled: boolean
  farm_enabled: boolean
  public_pool_url: string | null
  local_node_url: string | null
  public_farm_base_url: string | null
  local_farm_ws_url: string | null
  public_farm_ws_url: string | null
}

export default function FarmPanel() {
  const [rigs, setRigs] = useState<FarmRig[]>([])
  const [miningInfo, setMiningInfo] = useState<MiningInfo | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [showFarmHandModal, setShowFarmHandModal] = useState(false)
  const [showEndpointsModal, setShowEndpointsModal] = useState(false)

  useEffect(() => {
    loadRigs()
    loadMiningInfo()
    
    // Poll every 5 seconds
    const interval = setInterval(() => {
      loadRigs()
    }, 5000)
    
    return () => clearInterval(interval)
  }, [])

  const loadRigs = async () => {
    try {
      const response = await fetch('/admin/farm/rigs')
      if (!response.ok) throw new Error('Failed to load rigs')
      
      const data = await response.json()
      setRigs(Array.isArray(data) ? data : [])
      setError(null)
    } catch (err) {
      console.error('Failed to load farm rigs:', err)
      setError('Failed to load farm rigs')
    } finally {
      setIsLoading(false)
    }
  }

  const loadMiningInfo = async () => {
    try {
      const response = await fetch('/mining/info')
      if (!response.ok) throw new Error('Failed to load mining info')
      
      const data = await response.json()
      setMiningInfo(data)
    } catch (err) {
      console.error('Failed to load mining info:', err)
    }
  }

  const handleStartRig = async (rigId: string) => {
    try {
      const response = await fetch(`/admin/farm/rigs/${rigId}/start`, {
        method: 'POST',
      })
      if (!response.ok) throw new Error('Failed to start rig')
      await loadRigs()
    } catch (err) {
      console.error('Failed to start rig:', err)
      alert('Failed to start rig')
    }
  }

  const handleStopRig = async (rigId: string) => {
    try {
      const response = await fetch(`/admin/farm/rigs/${rigId}/stop`, {
        method: 'POST',
      })
      if (!response.ok) throw new Error('Failed to stop rig')
      await loadRigs()
    } catch (err) {
      console.error('Failed to stop rig:', err)
      alert('Failed to stop rig')
    }
  }

  const handleRemoveRig = async (rigId: string) => {
    if (!confirm('Are you sure you want to remove this rig?')) return
    
    try {
      const response = await fetch(`http://127.0.0.1:7070/admin/farm/rigs/${rigId}`, {
        method: 'DELETE',
      })
      if (!response.ok) throw new Error('Failed to remove rig')
      await loadRigs()
    } catch (err) {
      console.error('Failed to remove rig:', err)
      alert('Failed to remove rig')
    }
  }

  const getStatusBadgeClass = (status: string) => {
    switch (status.toLowerCase()) {
      case 'online': return 'badge-success'
      case 'mining': return 'badge-mining'
      case 'pending': return 'badge-warning'
      case 'offline': return 'badge-error'
      case 'error': return 'badge-error'
      default: return 'badge-default'
    }
  }

  const getEndpointBadge = (mode: string | null | undefined) => {
    if (!mode) return null
    return mode === 'local' ? (
      <span className="badge badge-local" title="LAN Connection">⚡ Local</span>
    ) : (
      <span className="badge badge-public" title="Internet Connection">🌐 Public</span>
    )
  }

  const formatHashrate = (hashrate: number) => {
    if (hashrate === 0) return 'Idle'
    if (hashrate >= 1000000) return `${(hashrate / 1000000).toFixed(2)} MH/s`
    if (hashrate >= 1000) return `${(hashrate / 1000).toFixed(2)} KH/s`
    return `${hashrate.toFixed(0)} H/s`
  }

  const formatLastSeen = (timestamp: number) => {
    const now = Date.now() / 1000
    const diff = now - timestamp
    
    if (diff < 60) return 'Just now'
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
    return `${Math.floor(diff / 86400)}d ago`
  }

  const totalHashrate = rigs.reduce((sum, rig) => sum + (rig.status === 'mining' ? rig.hashrate : 0), 0)
  const activeRigs = rigs.filter(r => r.status === 'online' || r.status === 'mining').length
  const miningRigs = rigs.filter(r => r.status === 'mining').length

  return (
    <div className="farm-panel">
      <div className="farm-header">
        <div className="farm-title-section">
          <h2 className="farm-title">🚜 Farm Management</h2>
          <p className="farm-subtitle">Control and monitor your distributed mining rigs</p>
        </div>
        
        <div className="farm-actions">
          <button 
            className="btn btn-secondary"
            onClick={() => setShowEndpointsModal(true)}
          >
            ⚙️ Configure Endpoints
          </button>
          <button 
            className="btn btn-primary"
            onClick={() => setShowFarmHandModal(true)}
          >
            ➕ Add FarmHand Rig
          </button>
        </div>
      </div>

      {/* Stats Overview */}
      <div className="farm-stats-grid">
        <div className="stat-card">
          <div className="stat-icon">📊</div>
          <div className="stat-content">
            <div className="stat-value">{rigs.length}</div>
            <div className="stat-label">Total Rigs</div>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon">🟢</div>
          <div className="stat-content">
            <div className="stat-value">{activeRigs}</div>
            <div className="stat-label">Online</div>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon">⛏️</div>
          <div className="stat-content">
            <div className="stat-value">{miningRigs}</div>
            <div className="stat-label">Mining</div>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon">⚡</div>
          <div className="stat-content">
            <div className="stat-value">{formatHashrate(totalHashrate)}</div>
            <div className="stat-label">Total Hashrate</div>
          </div>
        </div>
      </div>

      {/* Endpoint Configuration Status */}
      {miningInfo && (
        <div className="farm-endpoints-status">
          <h3>Configured Endpoints</h3>
          <div className="endpoints-grid">
            <div className="endpoint-item">
              <span className="endpoint-label">⚡ Local Node:</span>
              <span className="endpoint-value">
                {miningInfo.local_node_url || <span className="text-muted">Not configured</span>}
              </span>
            </div>
            <div className="endpoint-item">
              <span className="endpoint-label">🌐 Public Farm:</span>
              <span className="endpoint-value">
                {miningInfo.public_farm_base_url || <span className="text-muted">Not configured</span>}
              </span>
            </div>
          </div>
          {miningInfo.local_farm_ws_url && (
            <div className="endpoint-ws-preview">
              <small>Local WebSocket: <code>{miningInfo.local_farm_ws_url}</code></small>
            </div>
          )}
          {miningInfo.public_farm_ws_url && (
            <div className="endpoint-ws-preview">
              <small>Public WebSocket: <code>{miningInfo.public_farm_ws_url}</code></small>
            </div>
          )}
        </div>
      )}

      {/* Rigs Table */}
      <div className="farm-rigs-section">
        <h3>Farm Rigs</h3>
        
        {isLoading ? (
          <div className="loading-state">
            <div className="spinner"></div>
            <p>Loading rigs...</p>
          </div>
        ) : error ? (
          <div className="error-state">
            <p>❌ {error}</p>
            <button className="btn btn-secondary" onClick={loadRigs}>Retry</button>
          </div>
        ) : rigs.length === 0 ? (
          <div className="empty-state">
            <div className="empty-icon">🚜</div>
            <h3>No Farm Rigs Yet</h3>
            <p>Create your first FarmHand rig to get started</p>
            <button 
              className="btn btn-primary"
              onClick={() => setShowFarmHandModal(true)}
            >
              ➕ Add FarmHand Rig
            </button>
          </div>
        ) : (
          <div className="rigs-table-container">
            <table className="rigs-table">
              <thead>
                <tr>
                  <th>Rig Name</th>
                  <th>Status</th>
                  <th>Connection</th>
                  <th>OS</th>
                  <th>Threads</th>
                  <th>Hashrate</th>
                  <th>Last Seen</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {rigs.map((rig) => (
                  <tr key={rig.rig_id}>
                    <td>
                      <div className="rig-name">
                        <strong>{rig.name}</strong>
                        <small className="rig-id">{rig.rig_id}</small>
                      </div>
                    </td>
                    <td>
                      <span className={`badge ${getStatusBadgeClass(rig.status)}`}>
                        {rig.status.toUpperCase()}
                      </span>
                    </td>
                    <td>
                      {getEndpointBadge(rig.endpoint_mode)}
                    </td>
                    <td>{rig.os}</td>
                    <td>{rig.cpu_threads}</td>
                    <td>{formatHashrate(rig.hashrate)}</td>
                    <td>{formatLastSeen(rig.last_heartbeat)}</td>
                    <td>
                      <div className="rig-actions">
                        {rig.status === 'pending' && (
                          <span className="text-muted" title="Waiting for rig to connect">⏳</span>
                        )}
                        {(rig.status === 'online' || rig.status === 'idle') && (
                          <button
                            className="btn-icon btn-success"
                            onClick={() => handleStartRig(rig.rig_id)}
                            title="Start Mining"
                          >
                            ▶️
                          </button>
                        )}
                        {rig.status === 'mining' && (
                          <button
                            className="btn-icon btn-warning"
                            onClick={() => handleStopRig(rig.rig_id)}
                            title="Stop Mining"
                          >
                            ⏸️
                          </button>
                        )}
                        <button
                          className="btn-icon btn-danger"
                          onClick={() => handleRemoveRig(rig.rig_id)}
                          title="Remove Rig"
                        >
                          🗑️
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Modals */}
      <FarmHandModal
        isOpen={showFarmHandModal}
        onClose={() => setShowFarmHandModal(false)}
        onCreated={() => {
          loadRigs()
          loadMiningInfo()
        }}
      />

      <MiningEndpointsModal
        isOpen={showEndpointsModal}
        onClose={() => setShowEndpointsModal(false)}
        onSaved={() => loadMiningInfo()}
      />
    </div>
  )
}
