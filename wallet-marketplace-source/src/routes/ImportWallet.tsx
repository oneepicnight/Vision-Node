import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useOnboardingStore } from '../state/onboarding'
import { deriveKeys } from '../lib/keystore'
import { useWalletStore } from '../state/wallet'
import { isValidHandle } from '../lib/guards'

export default function ImportWallet() {
  const navigate = useNavigate()
  const { reset: resetOnboarding } = useOnboardingStore()
  const { setProfile } = useWalletStore()
  const [handle, setHandleInput] = useState('')
  const [seedPhrase, setSeedPhrase] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  const handleImport = async () => {
    const cleanHandle = handle.trim().toLowerCase()
    const cleanSeed = seedPhrase.trim().toLowerCase()
    
    // Validate handle
    if (!cleanHandle || !isValidHandle(cleanHandle)) {
      setError('Handles are 3–24 chars: a–z, 0–9, . _ -')
      return
    }
    
    // Validate seed phrase (should be 12 or 24 words)
    const words = cleanSeed.split(/\s+/).filter(w => w.length > 0)
    if (words.length !== 12 && words.length !== 24) {
      setError('Seed phrase must be 12 or 24 words')
      return
    }
    
    setError('')
    setLoading(true)
    
    try {
      console.log('Importing wallet with seed phrase...')
      
      // Derive keys from seed phrase
      const keys = await deriveKeys(words)
      console.log('Derived address:', keys.address)
      
      // Create profile
      const profile = {
        handle: cleanHandle,
        address: keys.address,
        createdAt: Date.now()
      }
      
      // Store in wallet state
      setProfile(profile)
      
      // Clear onboarding state
      resetOnboarding()
      
      console.log('Wallet imported successfully')
      
      // Navigate to home
      navigate('/home')
    } catch (err) {
      console.error('Wallet import error:', err)
      setError(`Failed to import wallet: ${err instanceof Error ? err.message : 'Unknown error'}`)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="page-container">
      <div className="form-container">
        <div>
          <h2 className="form-title">
            Import <span className="form-accent">Wallet</span>
          </h2>
          <p className="form-subtitle">
            Restore your wallet using your seed phrase.
          </p>
        </div>

        <div>
          <div className="input-group">
            <span className="input-prefix">@</span>
            <input 
              type="text"
              placeholder="your-handle"
              maxLength={24}
              value={handle}
              onChange={(e) => setHandleInput(e.target.value)}
              className="text-input"
              disabled={loading}
            />
          </div>
          
          <div style={{ marginTop: '1rem' }}>
            <label style={{ display: 'block', marginBottom: '0.5rem', color: 'rgba(255, 255, 255, 0.7)', fontSize: '0.875rem' }}>
              Seed Phrase (12 or 24 words)
            </label>
            <textarea 
              placeholder="word1 word2 word3 ..."
              rows={4}
              value={seedPhrase}
              onChange={(e) => setSeedPhrase(e.target.value)}
              className="text-input"
              style={{ resize: 'vertical', fontFamily: 'monospace', fontSize: '0.875rem' }}
              disabled={loading}
            />
          </div>
          
          <div className="error-message">
            {error}
          </div>
        </div>

        <div>
          <button 
            onClick={handleImport}
            disabled={loading}
            className="primary-button"
          >
            {loading ? 'Importing...' : 'Import Wallet'}
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
