import * as React from 'react'
import * as ReactDOM from 'react-dom/client'

function SimpleApp() {
  return (
    <div style={{ 
      backgroundColor: '#ff0000', 
      color: 'white', 
      padding: '20px',
      fontSize: '24px'
    }}>
      <h1>SIMPLE TEST - Can you see this?</h1>
      <p>If you see this red box, React is working!</p>
    </div>
  )
}

const root = document.getElementById('root')
if (root) {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <SimpleApp />
    </React.StrictMode>
  )
} else {
  console.error('Root element not found!')
}