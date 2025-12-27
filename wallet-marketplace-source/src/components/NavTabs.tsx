import { NavLink, useLocation } from 'react-router-dom'
import { useWalletStore } from '../state/wallet'
import { env } from '../utils/env'
import '../styles/nav-tabs.css'

interface NavTab {
  label: string
  path: string
  devOnly?: boolean
}

const mainTabs: NavTab[] = [
  { label: 'Command Center', path: '/command-center' },
  { label: 'Wallet', path: '/wallet' },
  { label: 'Mining', path: '/mining' },
  { label: 'Exchange', path: '/exchange' },
  { label: 'Market', path: '/market' },
  { label: 'Settings', path: '/settings' },
]

const devTabs: NavTab[] = [
  { label: 'Miner', path: '/panel.html', devOnly: true },
  { label: 'Dashboard', path: '/dashboard.html', devOnly: true },
  { label: 'Debug Crypto', path: '/debug/crypto', devOnly: true },
]

export default function NavTabs() {
  const { profile } = useWalletStore()
  const location = useLocation()

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

          // Regular router links
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
