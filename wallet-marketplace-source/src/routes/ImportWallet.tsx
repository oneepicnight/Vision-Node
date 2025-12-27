import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useOnboardingStore } from '../state/onboarding'
import { deriveKeys } from '../lib/keystore'
import { useWalletStore } from '../state/wallet'
import { isValidHandle } from '../lib/guards'
import '../styles/wallet-aaa.css'
import LinkDiscordStep from '../pages/LinkDiscordStep'

export default function ImportWallet() {
  const navigate = useNavigate()
  const { reset: resetOnboarding } = useOnboardingStore()
  const { setProfile } = useWalletStore()
  const [handle, setHandleInput] = useState('')
  const [seedPhrase, setSeedPhrase] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)
  const [showDiscordStep, setShowDiscordStep] = useState(false)
  const [importedAddress, setImportedAddress] = useState('')

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
      
      // Store imported address for Discord linking
      setImportedAddress(keys.address)
      
      // Clear onboarding state
      resetOnboarding()
      
      console.log('Wallet imported successfully')
      
      // Show Discord linking step
      setShowDiscordStep(true)
    } catch (err) {
      console.error('Wallet import error:', err)
      setError(`Failed to import wallet: ${err instanceof Error ? err.message : 'Unknown error'}`)
    } finally {
      setLoading(false)
    }
  }

  const onBack = () => {
    navigate('/')
  }

  // Show Discord linking step after successful import
  if (showDiscordStep) {
    return (
      <LinkDiscordStep
        walletAddress={importedAddress}
        onSkip={() => {
          console.log('Skipped Discord linking')
          navigate('/wallet')
        }}
        onLinked={() => {
          console.log('Discord linked successfully')
          navigate('/wallet')
        }}
      />
    )
  }

  return (
    <div className="page-container">
      <div className="form-container">
        <div>
          <h2 className="form-title">
            Import your <span className="form-accent">wallet</span>
          </h2>
          <p className="form-subtitle">
            Restore your wallet using your seed phrase.
          </p>
        </div>

        <div className="form-field">
          <label className="field-label">Handle (optional)</label>
          <div className="handle-input-wrapper">
            <span className="handle-prefix">@</span>
            <input
              type="text"
              className="text-input handle-input"
              placeholder="your-handle"
              maxLength={24}
              value={handle}
              onChange={(e) => setHandleInput(e.target.value)}
              disabled={loading}
            />
          </div>
        </div>

        <div className="form-field">
          <label className="field-label">Seed Phrase (12 or 24 words)</label>
          <textarea
            className="text-input textarea-input"
            rows={4}
            placeholder="word1 word2 word3 ..."
            value={seedPhrase}
            onChange={(e) => setSeedPhrase(e.target.value)}
            disabled={loading}
          />
        </div>

        {error && (
          <div className="error-message">
            {error}
          </div>
        )}

        <div>
          <button
            className="primary-button"
            onClick={handleImport}
            disabled={!seedPhrase.trim() || loading}
          >
            {loading ? 'Importing...' : 'Import Wallet'}
          </button>
          
          <button 
            className="secondary-button" 
            onClick={onBack}
            disabled={loading}
          >
            Back
          </button>
        </div>
      </div>
    </div>
  )
}
