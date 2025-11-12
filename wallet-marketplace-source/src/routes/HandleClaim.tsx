import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useOnboardingStore } from '../state/onboarding'
import { generateMnemonic } from '../lib/keystore'
import { isValidHandle } from '../lib/guards'

export default function HandleClaim() {
  const navigate = useNavigate()
  const { setHandle, setMnemonic } = useOnboardingStore()
  const [input, setInput] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  const handleClaim = async () => {
    const handle = input.trim().toLowerCase()
    
    // Validate handle using guards
    if (!handle || !isValidHandle(handle)) {
      setError('Handles are 3–24 chars: a–z, 0–9, . _ -')
      return
    }
    
    setError('')
    setLoading(true)
    
    try {
      console.log('Starting wallet generation...')
      
      // Generate mnemonic and store in onboarding state
      const mnemonic = generateMnemonic()
      console.log('Generated mnemonic:', mnemonic.length, 'words')
      
      setHandle(handle)
      setMnemonic(mnemonic)
      
      console.log('Stored handle and mnemonic, navigating...')
      
      // Navigate to secure key page
      navigate('/secure')
    } catch (err) {
      console.error('Wallet generation error:', err)
      setError(`Failed to generate wallet: ${err instanceof Error ? err.message : 'Unknown error'}`)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="page-container">
      <div className="form-container">
        <div>
          <h2 className="form-title">
            Claim your <span className="form-accent">handle</span>
          </h2>
          <p className="form-subtitle">
            This is the name glowing above your avatar.
          </p>
        </div>

        <div>
          <div className="input-group">
            <span className="input-prefix">@</span>
            <input 
              type="text"
              placeholder="neo-vision"
              maxLength={24}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyPress={(e) => e.key === 'Enter' && !loading && handleClaim()}
              className="text-input"
              disabled={loading}
            />
          </div>
          
          <div className="error-message">
            {error}
          </div>
        </div>

        <div>
          <button 
            onClick={handleClaim}
            disabled={loading}
            className="primary-button"
          >
            {loading ? 'Generating...' : 'Claim & Generate Wallet'}
          </button>
          
          <button 
            onClick={() => navigate('/')}
            disabled={loading}
            className="secondary-button"
          >
            Back
          </button>
        </div>
      </div>
    </div>
  )
}