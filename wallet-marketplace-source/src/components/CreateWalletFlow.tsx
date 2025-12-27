import { useState, useEffect } from 'react'
import { useOnboardingStore } from '../state/onboarding'
import { generateMnemonic } from '../lib/keystore'
import { isValidHandle } from '../lib/guards'

interface CreateWalletFlowProps {
  onBack: () => void
  onSuccess: () => void
}

export default function CreateWalletFlow({ onBack, onSuccess }: CreateWalletFlowProps) {
  const { setHandle, setMnemonic } = useOnboardingStore()
  const [input, setInput] = useState('')
  const [error, setError] = useState('')
  const [validationError, setValidationError] = useState('')
  const [loading, setLoading] = useState(false)
  const [success, setSuccess] = useState(false)
  const [isValidating, setIsValidating] = useState(false)

  // Real-time validation
  useEffect(() => {
    if (!input) {
      setValidationError('')
      return
    }

    const handle = input.trim().toLowerCase()
    
    // Check length
    if (handle.length < 3) {
      setValidationError('Handle must be at least 3 characters')
      return
    }
    
    if (handle.length > 24) {
      setValidationError('Handle must be 24 characters or less')
      return
    }

    // Check allowed characters
    if (!isValidHandle(handle)) {
      setValidationError('Only letters, numbers, underscores, dots, and hyphens')
      return
    }

    setValidationError('')
  }, [input])

  // Check handle availability (simulated - can be replaced with real API call)
  const checkAvailability = async (handleToCheck: string): Promise<boolean> => {
    // Simulate network delay
    await new Promise(resolve => setTimeout(resolve, 300))
    
    // TODO: Replace with actual API call to check handle availability
    console.log('Checking availability for:', handleToCheck)
    return true
  }

  const handleClaim = async () => {
    const handle = input.trim().toLowerCase()
    
    // Final validation
    if (!handle || !isValidHandle(handle)) {
      setError('Please enter a valid handle')
      return
    }

    if (validationError) {
      setError(validationError)
      return
    }
    
    setError('')
    setLoading(true)
    setIsValidating(true)
    
    try {
      // Check if handle is available
      const isAvailable = await checkAvailability(handle)
      
      if (!isAvailable) {
        setError('That handle is already taken')
        setLoading(false)
        setIsValidating(false)
        return
      }

      console.log('Handle available, generating wallet...')
      
      // Generate mnemonic and store in onboarding state
      const mnemonic = generateMnemonic()
      console.log('Generated mnemonic:', mnemonic.length, 'words')
      
      setHandle(handle)
      setMnemonic(mnemonic)
      
      // Show success state
      setSuccess(true)
      
      // Navigate after brief success display
      setTimeout(() => {
        onSuccess()
      }, 800)
    } catch (err) {
      console.error('Wallet generation error:', err)
      setError(`Failed to generate wallet: ${err instanceof Error ? err.message : 'Unknown error'}`)
      setLoading(false)
      setIsValidating(false)
    }
  }

  const isInputValid = input.trim().length >= 3 && !validationError

  if (success) {
    return (
      <div className="wizard-success">
        <div className="wizard-success-icon">✓</div>
        <h1 className="wizard-title">Handle claimed!</h1>
        <p className="wizard-subtitle">Generating your wallet...</p>
      </div>
    )
  }

  return (
    <div className="wizard-flow">
      <div className="wizard-header">
        <h1 className="wizard-title">
          Claim your <span className="wizard-accent">handle</span>
        </h1>
        <p className="wizard-subtitle">
          This is the name glowing above your avatar.
        </p>
      </div>

      <div className="wizard-body">
        <div className="wizard-field">
          <label className="wizard-label">Your Handle</label>
          <div className="wizard-handle-input-wrapper">
            <span className="wizard-handle-prefix">@</span>
            <input 
              type="text"
              placeholder="neo-vision"
              maxLength={24}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyPress={(e) => e.key === 'Enter' && !loading && isInputValid && handleClaim()}
              className="wizard-input wizard-handle-input"
              disabled={loading}
              autoFocus
            />
          </div>
          
          <p className="wizard-helper">
            Handles are 3–24 characters. Letters, numbers, underscores, dots, and hyphens.
          </p>
          
          {validationError && (
            <div className="wizard-validation-error">
              {validationError}
            </div>
          )}
          
          {error && !validationError && (
            <div className="wizard-error">
              {error}
            </div>
          )}
        </div>

        <div className="wizard-actions">
          <button 
            onClick={onBack}
            disabled={loading}
            className="btn-secondary"
          >
            Back
          </button>
          
          <button 
            onClick={handleClaim}
            disabled={loading || !isInputValid}
            className="btn-primary"
          >
            {loading ? (
              <>
                {isValidating ? 'Checking...' : 'Generating...'}
              </>
            ) : (
              'Continue'
            )}
          </button>
        </div>
      </div>
    </div>
  )
}
