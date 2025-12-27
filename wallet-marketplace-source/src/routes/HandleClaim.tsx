import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useOnboardingStore } from '../state/onboarding'
import { generateMnemonic } from '../lib/keystore'
import { isValidHandle } from '../lib/guards'
import VisionGlobe from '../components/VisionGlobe'
import '../styles/wallet-aaa.css'

export default function HandleClaim() {
  const navigate = useNavigate()
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
    // const response = await api('/api/handle/check', { method: 'POST', body: JSON.stringify({ handle: handleToCheck }) })
    // return response.available
    
    // For now, always return available (simulate success)
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
        navigate('/secure')
      }, 800)
    } catch (err) {
      console.error('Wallet generation error:', err)
      setError(`Failed to generate wallet: ${err instanceof Error ? err.message : 'Unknown error'}`)
      setLoading(false)
      setIsValidating(false)
    }
  }

  const isInputValid = input.trim().length >= 3 && !validationError

  return (
    <div className="vw-welcome-shell">
      {/* Same globe background as splash screen */}
      <div className="pointer-events-none absolute inset-0 z-0 flex items-center justify-center">
        <VisionGlobe />
      </div>
      
      <div className="vw-glow-layer"></div>

      <div className="vw-handle-hero">
        {success ? (
          <div className="vw-handle-success">
            <div className="vw-success-icon">✓</div>
            <h1 className="vw-handle-title">Handle claimed!</h1>
            <p className="vw-handle-subtitle">Generating your wallet...</p>
          </div>
        ) : (
          <>
            <div className="vw-handle-header">
              <h1 className="vw-handle-title">
                Claim your <span className="vw-handle-accent">handle</span>
              </h1>
              <p className="vw-handle-subtitle">
                This is the name glowing above your avatar.
              </p>
            </div>

            <div className="vw-handle-body">
              <div className="vw-handle-input-section">
                <label className="vw-label">Your Handle</label>
                <div className="vw-handle-input-wrapper">
                  <span className="vw-handle-prefix">@</span>
                  <input 
                    type="text"
                    placeholder="neo-vision"
                    maxLength={24}
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyPress={(e) => e.key === 'Enter' && !loading && isInputValid && handleClaim()}
                    className="vw-handle-input"
                    disabled={loading}
                    autoFocus
                  />
                </div>
                
                <p className="vw-helper">
                  Handles are 3–24 characters. Letters, numbers, underscores, dots, and hyphens.
                </p>
                
                {validationError && (
                  <div className="vw-validation-error">
                    {validationError}
                  </div>
                )}
                
                {error && !validationError && (
                  <div className="vw-error-message">
                    {error}
                  </div>
                )}
              </div>

              <div className="vw-handle-actions">
                <button 
                  onClick={handleClaim}
                  disabled={loading || !isInputValid}
                  className="vw-primary-btn vw-btn-full"
                >
                  {loading ? (
                    <>
                      {isValidating ? 'Checking availability...' : 'Generating wallet...'}
                    </>
                  ) : (
                    'Claim & Generate Wallet'
                  )}
                </button>
                
                <button 
                  onClick={() => navigate('/')}
                  disabled={loading}
                  className="vw-text-link"
                >
                  ← Back
                </button>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  )
}