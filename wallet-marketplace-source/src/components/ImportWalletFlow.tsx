import { useState } from 'react'
import { useOnboardingStore } from '../state/onboarding'
import { deriveKeys } from '../lib/keystore'
import { useWalletStore } from '../state/wallet'
import { isValidHandle } from '../lib/guards'

interface ImportWalletFlowProps {
  onBack: () => void
  onSuccess: () => void
}

export default function ImportWalletFlow({ onBack, onSuccess }: ImportWalletFlowProps) {
  const { reset: resetOnboarding } = useOnboardingStore()
  const { setProfile } = useWalletStore()
  const [handle, setHandleInput] = useState('')
  const [seedPhrase, setSeedPhrase] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  const handleImport = async () => {
    const cleanHandle = handle.trim().toLowerCase()
    const cleanSeed = seedPhrase.trim().toLowerCase()
    
    // Validate handle (optional field)
    if (cleanHandle && !isValidHandle(cleanHandle)) {
      setError('Handles are 3–24 chars: letters, numbers, underscores, dots, hyphens')
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
      
      // Create profile (use handle if provided, otherwise use address prefix)
      const profile = {
        handle: cleanHandle || keys.address.substring(0, 8),
        address: keys.address,
        createdAt: Date.now()
      }
      
      // Store in wallet state
      setProfile(profile)
      
      // Clear onboarding state
      resetOnboarding()
      
      console.log('Wallet imported successfully')
      
      // Navigate to wallet home (skip Discord linking for now)
      setTimeout(() => {
        onSuccess()
      }, 500)
    } catch (err) {
      console.error('Wallet import error:', err)
      setError(`Failed to import wallet: ${err instanceof Error ? err.message : 'Unknown error'}`)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="wizard-flow">
      <div className="wizard-header">
        <h1 className="wizard-title">
          Import your <span className="wizard-accent">wallet</span>
        </h1>
        <p className="wizard-subtitle">
          Restore your wallet using your seed phrase.
        </p>
      </div>

      <div className="wizard-body">
        <div className="wizard-field">
          <label className="wizard-label">Handle (optional)</label>
          <div className="wizard-handle-input-wrapper">
            <span className="wizard-handle-prefix">@</span>
            <input
              type="text"
              className="wizard-input wizard-handle-input"
              placeholder="your-handle"
              maxLength={24}
              value={handle}
              onChange={(e) => setHandleInput(e.target.value)}
              disabled={loading}
            />
          </div>
        </div>

        <div className="wizard-field">
          <label className="wizard-label">Seed Phrase (12 or 24 words)</label>
          <textarea
            className="wizard-input wizard-textarea"
            rows={4}
            placeholder="word1 word2 word3 ..."
            value={seedPhrase}
            onChange={(e) => setSeedPhrase(e.target.value)}
            disabled={loading}
          />
        </div>

        {error && (
          <div className="wizard-error">
            {error}
          </div>
        )}

        <div className="wizard-actions">
          <button
            onClick={onBack}
            disabled={loading}
            className="btn-secondary"
          >
            Back
          </button>
          
          <button
            className="btn-primary"
            onClick={handleImport}
            disabled={!seedPhrase.trim() || loading}
          >
            {loading ? 'Importing...' : 'Continue'}
          </button>
        </div>
      </div>
    </div>
  )
}
