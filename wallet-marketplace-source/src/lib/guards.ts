import * as React from 'react'
import { useNavigate } from 'react-router-dom'
import { useWalletStore } from '../state/wallet'

// Validation functions
export function isValidHandle(handle: string): boolean {
  return /^[a-z0-9._-]{3,24}$/.test(handle)
}

export function isValidAddress(address: string): boolean {
  return /^0x[0-9a-fA-F]{40,}$/.test(address)
}

export function isPositiveAmount(amount: string | number): boolean {
  const num = typeof amount === 'string' ? parseFloat(amount) : amount
  return !isNaN(num) && num > 0
}

// Route guard hook
export function useRequireWallet() {
  const { profile } = useWalletStore()
  return profile
}

// Higher-order component for route protection
export function requireWallet<P extends object>(
  Component: React.ComponentType<P>
): React.ComponentType<P> {
  return function ProtectedComponent(props: P) {
    const profile = useRequireWallet()
    const navigate = useNavigate()
    
    React.useEffect(() => {
      if (!profile) {
        console.log('No profile found, redirecting to splash...')
        navigate('/')
      }
    }, [profile, navigate])
    
    if (!profile) {
      return null
    }
    
    return React.createElement(Component, props)
  }
}