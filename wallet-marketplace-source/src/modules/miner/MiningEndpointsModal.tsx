import { useState, useEffect } from 'react'

interface MiningEndpointsModalProps {
  isOpen: boolean
  onClose: () => void
  onSaved: () => void
}

interface MiningEndpoints {
  public_pool_url: string | null
  local_node_url: string | null
  public_farm_base_url: string | null
}

export default function MiningEndpointsModal({ isOpen, onClose, onSaved }: MiningEndpointsModalProps) {
  const [endpoints, setEndpoints] = useState<MiningEndpoints>({
    public_pool_url: '',
    local_node_url: '',
    public_farm_base_url: '',
  })
  const [isSaving, setIsSaving] = useState(false)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState(false)

  useEffect(() => {
    if (isOpen) {
      loadEndpoints()
    }
  }, [isOpen])

  const loadEndpoints = async () => {
    setIsLoading(true)
    setError(null)
    try {
      const response = await fetch('/admin/mining/endpoints')
      if (!response.ok) throw new Error('Failed to load endpoints')
      
      const data = await response.json()
      setEndpoints({
        public_pool_url: data.public_pool_url || '',
        local_node_url: data.local_node_url || '',
        public_farm_base_url: data.public_farm_base_url || '',
      })
    } catch (err) {
      console.error('Failed to load endpoints:', err)
      setError('Failed to load current configuration')
    } finally {
      setIsLoading(false)
    }
  }

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setSuccess(false)
    setIsSaving(true)

    try {
      // Send non-empty values to backend
      const payload: any = {}
      if (endpoints.public_pool_url?.trim()) {
        payload.public_pool_url = endpoints.public_pool_url.trim()
      }
      if (endpoints.local_node_url?.trim()) {
        payload.local_node_url = endpoints.local_node_url.trim()
      }
      if (endpoints.public_farm_base_url?.trim()) {
        payload.public_farm_base_url = endpoints.public_farm_base_url.trim()
      }

      const response = await fetch('/admin/mining/endpoints', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(payload),
      })

      if (!response.ok) {
        const errorData = await response.json().catch(() => null)
        throw new Error(errorData?.message || `Server error: ${response.status}`)
      }

      setSuccess(true)
      setTimeout(() => {
        onSaved()
        onClose()
      }, 1500)
    } catch (err) {
      console.error('Failed to save endpoints:', err)
      setError(err instanceof Error ? err.message : 'Failed to save configuration')
    } finally {
      setIsSaving(false)
    }
  }

  if (!isOpen) return null

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content modal-wide" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Configure Mining Endpoints</h2>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>

        <form onSubmit={handleSave}>
          <div className="modal-body">
            {error && (
              <div className="alert alert-error">
                {error}
              </div>
            )}

            {success && (
              <div className="alert alert-success">
                ✅ Configuration saved successfully!
              </div>
            )}

            {isLoading ? (
              <div className="loading-spinner">Loading configuration...</div>
            ) : (
              <>
                {/* Public Pool URL */}
                <div className="form-group">
                  <label htmlFor="public-pool-url">
                    🌐 Public Pool URL
                  </label>
                  <input
                    id="public-pool-url"
                    type="text"
                    value={endpoints.public_pool_url || ''}
                    onChange={(e) => setEndpoints({ ...endpoints, public_pool_url: e.target.value })}
                    placeholder="stratum+tcp://pool.visionworld.tech:4242"
                    disabled={isSaving}
                  />
                  <small className="form-hint">
                    External mining pool address for pool miners to connect to
                  </small>
                </div>

                {/* Local Node URL */}
                <div className="form-group">
                  <label htmlFor="local-node-url">
                    ⚡ Local Node URL (for LAN Farm Rigs)
                  </label>
                  <input
                    id="local-node-url"
                    type="text"
                    value={endpoints.local_node_url || ''}
                    onChange={(e) => setEndpoints({ ...endpoints, local_node_url: e.target.value })}
                    placeholder="http://192.168.1.10:7070"
                    disabled={isSaving}
                  />
                  <small className="form-hint">
                    Your local network IP address. FarmHand rigs on LAN will connect here.
                    <br />
                    💡 Find your IP: <code>ipconfig</code> (Windows) or <code>ip addr</code> (Linux)
                  </small>
                  {endpoints.local_node_url && (
                    <div className="endpoint-preview">
                      <strong>WebSocket URL:</strong> {endpoints.local_node_url.replace(/^http/, 'ws')}/farm/ws
                    </div>
                  )}
                </div>

                {/* Public Farm Base URL */}
                <div className="form-group">
                  <label htmlFor="public-farm-url">
                    🌍 Public Farm Base URL (for Offsite Rigs)
                  </label>
                  <input
                    id="public-farm-url"
                    type="text"
                    value={endpoints.public_farm_base_url || ''}
                    onChange={(e) => setEndpoints({ ...endpoints, public_farm_base_url: e.target.value })}
                    placeholder="https://farm.visionworld.tech"
                    disabled={isSaving}
                  />
                  <small className="form-hint">
                    Public domain or IP for remote FarmHand rigs to connect via internet.
                    <br />
                    ⚠️ Requires port forwarding or reverse proxy setup.
                  </small>
                  {endpoints.public_farm_base_url && (
                    <div className="endpoint-preview">
                      <strong>WebSocket URL:</strong> {endpoints.public_farm_base_url.replace(/^http/, 'ws').replace(/^https/, 'wss')}/farm/ws
                    </div>
                  )}
                </div>

                {/* Info Box */}
                <div className="info-box">
                  <strong>ℹ️ How it works:</strong>
                  <ul>
                    <li><strong>Local Node URL:</strong> Used for FarmHand rigs on your LAN (basement, garage, etc.)</li>
                    <li><strong>Public Farm URL:</strong> Used for FarmHand rigs at remote locations (warehouses, datacenters)</li>
                    <li><strong>Public Pool URL:</strong> Used for external miners connecting to your pool</li>
                  </ul>
                  <p className="text-small">
                    You can configure both local and public URLs to support mixed deployments.
                  </p>
                </div>
              </>
            )}
          </div>

          <div className="modal-footer">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={onClose}
              disabled={isSaving}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={isSaving || isLoading}
            >
              {isSaving ? 'Saving...' : '💾 Save Configuration'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
