import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import '../styles/wallet-aaa.css'
import CreateWalletFlow from '../components/CreateWalletFlow'
import ImportWalletFlow from '../components/ImportWalletFlow'

type WizardMode = 'welcome' | 'create' | 'import'

export default function Splash() {
  const navigate = useNavigate()
  const [mode, setMode] = useState<WizardMode>('welcome')

  const handleBackToWelcome = () => {
    setMode('welcome')
  }

  const handleCreateSuccess = () => {
    // After create flow completes, navigate to secure key page, then command center
    navigate('/secure')
  }

  const handleImportSuccess = () => {
    // After import flow completes, navigate to command center
    navigate('/command-center')
  }

  return (
    <div className="vision-landing">
      {/* CENTERED CARD - GLOBE IS CSS PSEUDO-ELEMENT BEHIND IT */}
      <div className={`vision-panel vision-wizard-panel ${mode !== 'welcome' ? 'vision-panel-extended' : ''}`}>
        {mode === 'welcome' && (
          <div className="vision-wizard-content fade-in">
            <h1 className="vision-title-main">Welcome,</h1>
            <h2 className="vision-title-sub">Dreamer.</h2>
            <p className="vision-subline">The Constellation Awaits You.</p>

            <div className="wallet-actions">
              <button 
                onClick={() => setMode('create')}
                className="btn-primary"
              >
                CREATE NEW WALLET
              </button>
              
              <button 
                onClick={() => setMode('import')}
                className="btn-secondary"
              >
                IMPORT EXISTING WALLET
              </button>
            </div>
          </div>
        )}

        {mode === 'create' && (
          <div className="vision-wizard-content fade-in">
            <CreateWalletFlow 
              onBack={handleBackToWelcome}
              onSuccess={handleCreateSuccess}
            />
          </div>
        )}

        {mode === 'import' && (
          <div className="vision-wizard-content fade-in">
            <ImportWalletFlow 
              onBack={handleBackToWelcome}
              onSuccess={handleImportSuccess}
            />
          </div>
        )}
      </div>
    </div>
  )
}