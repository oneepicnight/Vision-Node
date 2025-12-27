import { useState } from 'react'

interface FarmHandModalProps {
  isOpen: boolean
  onClose: () => void
  onCreated: () => void
}

export default function FarmHandModal({ isOpen, onClose, onCreated }: FarmHandModalProps) {
  const [rigName, setRigName] = useState('')
  const [walletAddress, setWalletAddress] = useState('')
  const [connectionMode, setConnectionMode] = useState<'local' | 'public'>('local')
  const [defaultThreads, setDefaultThreads] = useState<number>(4)
  const [isCreating, setIsCreating] = useState(false)
  const [error, setError] = useState<string | null>(null)

  if (!isOpen) return null

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    
    if (!rigName.trim()) {
      setError('Rig name is required')
      return
    }

    setIsCreating(true)

    try {
      const response = await fetch('/admin/farm/farmhand/create', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          rig_name: rigName.trim(),
          wallet_address: walletAddress.trim() || undefined,
          connection_mode: connectionMode,
          default_threads: defaultThreads > 0 ? defaultThreads : undefined,
        }),
      })

      if (!response.ok) {
        const errorData = await response.json().catch(() => null)
        throw new Error(errorData?.error || `Server error: ${response.status}`)
      }

      // Download the ZIP file
      const blob = await response.blob()
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `farmhand_${rigName.replace(/\s+/g, '_')}.zip`
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)

      // Reset form and close
      setRigName('')
      setWalletAddress('')
      setConnectionMode('local')
      setDefaultThreads(4)
      onCreated()
      onClose()
    } catch (err) {
      console.error('Failed to create FarmHand bundle:', err)
      setError(err instanceof Error ? err.message : 'Failed to create bundle')
    } finally {
      setIsCreating(false)
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Create FarmHand Rig</h2>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="modal-body">
            {error && (
              <div className="alert alert-error">
                {error}
              </div>
            )}

            {/* Rig Name */}
            <div className="form-group">
              <label htmlFor="rig-name">
                Rig Name <span className="required">*</span>
              </label>
              <input
                id="rig-name"
                type="text"
                value={rigName}
                onChange={(e) => setRigName(e.target.value)}
                placeholder="e.g., BasementRig1, UtahWarehouse01"
                maxLength={64}
                required
                disabled={isCreating}
              />
              <small className="form-hint">A friendly name for this mining rig</small>
            </div>

            {/* Connection Mode */}
            <div className="form-group">
              <label>Connection Type <span className="required">*</span></label>
              <div className="radio-group">
                <label className="radio-option">
                  <input
                    type="radio"
                    name="connection-mode"
                    value="local"
                    checked={connectionMode === 'local'}
                    onChange={() => setConnectionMode('local')}
                    disabled={isCreating}
                  />
                  <div className="radio-label">
                    <strong>⚡ Local Node (LAN)</strong>
                    <small>Rig will connect to your local network node</small>
                    <small className="text-muted">Best for basement/garage rigs on same network</small>
                  </div>
                </label>

                <label className="radio-option">
                  <input
                    type="radio"
                    name="connection-mode"
                    value="public"
                    checked={connectionMode === 'public'}
                    onChange={() => setConnectionMode('public')}
                    disabled={isCreating}
                  />
                  <div className="radio-label">
                    <strong>🌐 Public Endpoint (Offsite)</strong>
                    <small>Rig will connect via internet to public farm controller</small>
                    <small className="text-muted">Best for remote warehouses/datacenters</small>
                  </div>
                </label>
              </div>
            </div>

            {/* Wallet Address (Optional) */}
            <div className="form-group">
              <label htmlFor="wallet-address">
                Mining Wallet Address <span className="optional">(optional)</span>
              </label>
              <input
                id="wallet-address"
                type="text"
                value={walletAddress}
                onChange={(e) => setWalletAddress(e.target.value)}
                placeholder="0x..."
                disabled={isCreating}
              />
              <small className="form-hint">
                Leave empty to use rig's default wallet
              </small>
            </div>

            {/* Default Threads (Optional) */}
            <div className="form-group">
              <label htmlFor="default-threads">
                CPU Threads <span className="optional">(optional)</span>
              </label>
              <input
                id="default-threads"
                type="number"
                value={defaultThreads}
                onChange={(e) => setDefaultThreads(parseInt(e.target.value) || 0)}
                min="0"
                max="128"
                disabled={isCreating}
              />
              <small className="form-hint">
                Number of CPU threads to use. Leave at 0 for auto-detect.
              </small>
            </div>

            {/* Info Box */}
            <div className="info-box">
              <strong>📦 What you'll get:</strong>
              <ul>
                <li>ZIP bundle with configuration file</li>
                <li>Windows batch script (START-FARMHAND.bat)</li>
                <li>Linux shell script (start-farmhand.sh)</li>
                <li>Setup instructions (README.txt)</li>
              </ul>
              <p className="text-small">
                ⚠️ You'll need to copy the FarmHand binary (farmhand.exe or farmhand) 
                into the extracted folder before running.
              </p>
            </div>
          </div>

          <div className="modal-footer">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={onClose}
              disabled={isCreating}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={isCreating || !rigName.trim()}
            >
              {isCreating ? 'Creating...' : '📥 Download Bundle'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
