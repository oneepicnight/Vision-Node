import { useNavigate } from 'react-router-dom'

export default function Splash() {
  const navigate = useNavigate()

  return (
    <div className="splash-container">
      <div className="splash-content">
        {/* Animated background grid */}
        <div className="splash-bg">
          <div className="splash-gradient"></div>
        </div>
        
        <div>
          <h1 className="splash-title">
            Welcome, <span className="splash-accent">Dreamer</span>.
          </h1>
          <p className="splash-subtitle">
            The world is yours to shape.
          </p>
        </div>
        
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', width: '100%', maxWidth: '300px', alignItems: 'center' }}>
          <button 
            onClick={() => navigate('/handle')}
            className="splash-button"
            style={{ width: '100%' }}
          >
            Create New Wallet
          </button>
          
          <button 
            onClick={() => navigate('/import')}
            className="splash-button"
            style={{ width: '100%', background: 'rgba(59, 130, 246, 0.1)', border: '1px solid rgba(59, 130, 246, 0.5)' }}
          >
            Import Existing Wallet
          </button>
        </div>
      </div>
    </div>
  )
}