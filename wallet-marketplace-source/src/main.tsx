import './polyfills'
import * as React from 'react'
import * as ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'
import './styles/wallet.css'
import './styles/wallet-vision.css'
import Toaster from './components/Toaster'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
  <App />
  <Toaster />
  </React.StrictMode>,
)