import { useNavigate } from 'react-router-dom'
import { useWalletStore } from '../state/wallet'
import { useNodeStatus } from '../hooks/useNodeStatus'
import { useMiningStatus } from '../hooks/useMiningStatus'
import { useGuardianStatus } from '../hooks/useGuardianStatus'
import '../styles/mini-command-center.css'

export default function MiniCommandCenter() {
  const navigate = useNavigate()
  const { balances } = useWalletStore()
  const nodeStatus = useNodeStatus()
  const miningStatus = useMiningStatus()
  const guardianStatus = useGuardianStatus()

  const handleClick = () => {
    navigate('/command-center')
  }

  return (
    <div 
      className="mini-command-center" 
      onClick={handleClick}
      title="Click to open full Command Center"
    >
      {/* Node Status */}
      <div className="mini-cc-card">
        <div className="mini-cc-icon">
          <div className={`mini-cc-status-dot ${nodeStatus.online ? 'online' : 'offline'}`}></div>
        </div>
        <div className="mini-cc-content">
          <div className="mini-cc-label">Node</div>
          <div className="mini-cc-value">
            {nodeStatus.online ? 'Online' : 'Offline'}
            <span className="mini-cc-subvalue"> • {(nodeStatus.height || 0).toLocaleString()}</span>
          </div>
        </div>
      </div>

      {/* Mining Status */}
      <div className="mini-cc-card">
        <div className="mini-cc-icon">
          <div className={`mini-cc-status-dot ${miningStatus.active ? 'mining' : 'idle'}`}>
            {miningStatus.active && <div className="mini-cc-pulse"></div>}
          </div>
        </div>
        <div className="mini-cc-content">
          <div className="mini-cc-label">Mining</div>
          <div className="mini-cc-value">
            {miningStatus.mode.toUpperCase()}
            {miningStatus.active && (
              <span className="mini-cc-subvalue"> • {(miningStatus.hashrate / 1000000).toFixed(1)} MH/s</span>
            )}
          </div>
        </div>
      </div>

      {/* Wallet Balances */}
      <div className="mini-cc-card">
        <div className="mini-cc-icon">💎</div>
        <div className="mini-cc-content">
          <div className="mini-cc-label">Wallet</div>
          <div className="mini-cc-value">
            {(balances?.LAND || 0).toFixed(2)} LAND
            <span className="mini-cc-subvalue"> • {(balances?.CASH || 0).toFixed(0)} CASH</span>
          </div>
        </div>
      </div>

      {/* Guardian/Beacon Status */}
      <div className="mini-cc-card">
        <div className="mini-cc-icon">
          <div className={`mini-cc-status-dot ${guardianStatus.guardianOnline ? 'online' : 'offline'}`}></div>
        </div>
        <div className="mini-cc-content">
          <div className="mini-cc-label">Guardian</div>
          <div className="mini-cc-value">
            {guardianStatus.beaconConnected ? 'Connected' : 'Not Connected'}
          </div>
        </div>
      </div>

      {/* Hover Hint */}
      <div className="mini-cc-hint">
        Click for full Command Center
      </div>
    </div>
  )
}
