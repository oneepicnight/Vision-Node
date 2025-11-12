import { useEffect } from 'react'
import { HashRouter as Router, Routes, Route, Link } from 'react-router-dom'
import { useWalletStore } from './state/wallet'
import { pingStatus } from './lib/api'
import { requireWallet } from './lib/guards'
import { loadConfig } from './lib/config'
import Splash from './routes/Splash'
import HandleClaim from './routes/HandleClaim'
import ImportWallet from './routes/ImportWallet'
import SecureKey from './routes/SecureKey'
import Home from './routes/Home'
import Settings from './routes/Settings'
import DebugCrypto from './routes/DebugCrypto'
import { Market, Orders } from './modules/market'
import ExchangePage from './modules/exchange/Exchange'
import { VaultCard } from './components/VaultCard'
import { env } from './utils/env'

// Protected components
const ProtectedHome = requireWallet(Home)
const ProtectedSettings = requireWallet(Settings)

// Status bar component
function StatusBar() {
  const { profile, node } = useWalletStore()
  
  const statusColor = {
    'up': 'up',
    'degraded': 'degraded', 
    'down': 'down'
  }[node.status]

  return (
    <div className="status-bar">
      <div className="status-left">
        <div className={`status-dot ${statusColor}`} title={`Node ${node.status}`}></div>
        <span className="status-handle">
          {profile ? `@${profile.handle}` : '@handle'}
        </span>
      </div>
      <div className="status-right">
        {profile && (
          <a 
            href="/settings"
            className="settings-link"
            title="Settings"
          >
            ⚙️
          </a>
        )}
        {!profile && (
          <a 
            href="/#/import"
            className="enter-vision-btn"
            style={{ textDecoration: 'none', display: 'inline-block' }}
          >
            Import Wallet
          </a>
        )}
      </div>
    </div>
  )
}

function App() {
  const { setNode } = useWalletStore()

  // Load config on startup
  useEffect(() => {
    loadConfig().catch(err => console.warn('Config load failed:', err));
  }, []);

  // Poll node status every 5 seconds
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const result = await pingStatus()
        const now = Date.now()
        
        if (result.up && result.info && typeof result.info === 'object') {
          // Node is responding with valid data
          setNode({ status: 'up', lastSeen: now })
        } else if (result.up) {
          // Node responding but missing fields
          setNode({ status: 'degraded', lastSeen: now })
        } else {
          // Network error
          setNode({ status: 'down' })
        }
      } catch (error) {
        setNode({ status: 'down' })
      }
    }

    // Check immediately
    checkStatus()
    
    // Then poll every 5 seconds
    const interval = setInterval(checkStatus, 5000)
    
    return () => clearInterval(interval)
  }, [setNode])

  // Render a small banner when node is down — show DEV bypass hint when enabled
  const { node } = useWalletStore()

  return (
    <Router basename={"/"}>
      <StatusBar />
      {/* Node offline banner */}
      {node.status === 'down' && (
        <div className="node-banner" style={{ background: '#fef3c7', color: '#92400e', padding: '8px 12px', textAlign: 'center' }}>
          Node offline ({env.NODE_URL}). {env.WALLET_DEV_BYPASS ? 'Running in DEV fallback — balances may be stale.' : 'Running in offline mode — balances may be stale.'}
        </div>
      )}
      <nav className="top-nav">
        <Link to="/home">Home</Link>
        <Link to="/market">Market</Link>
        <Link to="/exchange">Exchange</Link>
        <Link to="/settings">Settings</Link>
  {env.FEATURE_DEV_PANEL && <a href="/miner">Miner</a>}
  {env.FEATURE_DEV_PANEL && <a href="/orders">Orders</a>}
  {env.FEATURE_DEV_PANEL && <Link to="/debug/crypto">Debug Crypto</Link>}
      </nav>
      <div className="main-content">
        <div className="p-4">
          <VaultCard />
        </div>
        <Routes>
          <Route path="/" element={<Splash />} />
          <Route path="/market" element={<Market />} />
          <Route path="/exchange" element={<ExchangePage />} />
          <Route path="/handle" element={<HandleClaim />} />
          <Route path="/import" element={<ImportWallet />} />
          <Route path="/secure" element={<SecureKey />} />
          {env.FEATURE_DEV_PANEL && <Route path="/debug/crypto" element={<DebugCrypto />} />}
          <Route path="/home" element={<ProtectedHome />} />
          {env.FEATURE_DEV_PANEL && <Route path="/orders" element={<Orders />} />}
          <Route path="/settings" element={<ProtectedSettings />} />
        </Routes>
      </div>
    </Router>
  )
}

export default App