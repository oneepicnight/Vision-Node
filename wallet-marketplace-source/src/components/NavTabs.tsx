import { NavLink, useLocation } from 'react-router-dom'
import { useWalletStore } from '../state/wallet'
import { useExchangeStatus } from '../hooks/useExchangeStatus'
import { env } from '../utils/env'
import '../styles/nav-tabs.css'

interface NavTab {
  label: string
  path: string
  devOnly?: boolean
  requiresUnlock?: boolean
}

const mainTabs: NavTab[] = [
  { label: 'Command Center', path: '/command-center' },
  { label: 'Wallet', path: '/wallet' },
  { label: 'Exchange', path: '/exchange', requiresUnlock: true },
  { label: 'Market', path: '/market' },
  { label: 'Settings', path: '/settings' },
]

const devTabs: NavTab[] = [
  { label: 'Debug Crypto', path: '/debug/crypto', devOnly: true },
]

export default function NavTabs() {
  const { profile } = useWalletStore()
  const location = useLocation()
  const { status: exchangeStatus } = useExchangeStatus(profile?.address || null)

  // Don't render nav on splash/onboarding routes
  const hideOnRoutes = ['/', '/import', '/handle', '/secure']
  if (hideOnRoutes.includes(location.pathname) || !profile) {
    return null
  }

  const allTabs = env.FEATURE_DEV_PANEL ? [...mainTabs, ...devTabs] : mainTabs

  return (
    <nav className="nav-tabs">
      <div className="nav-tabs-container">
        {allTabs.map((tab) => {
          // Check if tab is locked
          const isLocked = tab.requiresUnlock && !exchangeStatus.enabled
          const tooltipText = isLocked ? exchangeStatus.reason || 'Exchange unlocks after your deposit is confirmed.' : undefined

          // Handle external links (dev panel links)
          if (tab.path.includes('.html')) {
            return (
              <a
                key={tab.path}
                href={tab.path}
                className="nav-tab nav-tab-dev"
                target="_blank"
                rel="noopener noreferrer"
              >
                {tab.label}
              </a>
            )
          }

          // Regular router links with lock support
          if (isLocked) {
            return (
              <div
                key={tab.path}
                className="nav-tab nav-tab-locked"
                title={tooltipText}
                onClick={(e) => {
                  e.preventDefault()
                  e.stopPropagation()
                }}
              >
                {tab.label}
              </div>
            )
          }

          return (
            <NavLink
              key={tab.path}
              to={tab.path}
              className={({ isActive }) =>
                `nav-tab ${isActive ? 'nav-tab-active' : ''} ${tab.devOnly ? 'nav-tab-dev' : ''}`
              }
              aria-current={location.pathname === tab.path ? 'page' : undefined}
            >
              {tab.label}
            </NavLink>
          )
        })}
      </div>
    </nav>
  )
}
