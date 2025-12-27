import { useEffect } from 'react'

/**
 * Redirect to the full mining panel HTML page
 */
export default function MinerRedirect() {
  useEffect(() => {
    window.location.href = `${window.location.origin}/panel.html`
  }, [])

  return (
    <div style={{ 
      display: 'flex', 
      justifyContent: 'center', 
      alignItems: 'center', 
      height: '100vh',
      fontFamily: 'Inter, sans-serif',
      color: '#fff'
    }}>
      <p>Redirecting to mining panel...</p>
    </div>
  )
}
