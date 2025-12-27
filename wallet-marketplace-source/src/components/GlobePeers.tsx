// Globe Peers - 3D Spinning Globe with Red Peer Dots
// Exact implementation from panel.html with wallet's globe size

import { useEffect, useRef } from 'react'
import './GlobePeers.css'

declare global {
  interface Window {
    Globe?: any
    THREE?: any
  }
}

// Helper function to create shooting stars (from panel.html)
function createShootingStars(container: HTMLElement) {
  for (let i = 0; i < 5; i++) {
    const star = document.createElement('div')
    star.className = 'shooting-star'
    star.style.top = `${Math.random() * 30}%`
    star.style.left = `${Math.random() * 100}%`
    star.style.animationDelay = `${Math.random() * 4}s`
    container.appendChild(star)
  }

  // Add Milky Way
  const milkyWay = document.createElement('div')
  milkyWay.className = 'milky-way'
  container.appendChild(milkyWay)
}

export default function GlobePeers() {
  const globeRef = useRef<any>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const animationIdRef = useRef<number | null>(null)

  useEffect(() => {
    // Wait for libraries to load
    const initTimeout = setTimeout(() => {
      try {
        if (typeof window.Globe !== 'undefined' && typeof window.THREE !== 'undefined' && containerRef.current) {
          const globeElement = containerRef.current

          // Create subtle shooting stars
          createShootingStars(globeElement)

          // Initialize globe with exact panel.html configuration
          globeRef.current = window.Globe()
            (globeElement)
            .globeImageUrl('https://unpkg.com/three-globe/example/img/earth-night.jpg')
            .bumpImageUrl('https://unpkg.com/three-globe/example/img/earth-topology.png')
            .backgroundImageUrl('') // No background image - using CSS starfield
            .pointsData([]) // Will be populated by peer data
            .pointColor(() => '#ff0000')
            .pointAltitude(0.01)
            .pointRadius(0.8)
            .pointsMerge(false)
            .atmosphereColor('#ff4444')
            .atmosphereAltitude(0.15)
            .showAtmosphere(true)

          // Position camera - move left and up to center in box
          globeRef.current.pointOfView({ lat: 10, lng: -15, altitude: 2.3 }, 0)

          // Auto-rotate ONLY the globe (not background)
          globeRef.current.controls().autoRotate = true
          globeRef.current.controls().autoRotateSpeed = 0.8
          globeRef.current.controls().enableZoom = true
          globeRef.current.controls().minDistance = 150
          globeRef.current.controls().maxDistance = 500

          // Disable background rotation - globe spins, stars stay fixed
          const scene = globeRef.current.scene()
          if (scene && scene.rotation) {
            scene.rotation.y = 0 // Lock background rotation
          }

          // Pulsing animation for red dots
          let pulseFactor = 0
          const animate = () => {
            try {
              pulseFactor += 0.02
              const pulseScale = 0.8 + Math.sin(pulseFactor) * 0.3

              if (globeRef.current && globeRef.current.pointsData && globeRef.current.pointsData().length > 0) {
                globeRef.current.pointRadius(pulseScale)
              }

              animationIdRef.current = requestAnimationFrame(animate)
            } catch (e) {
              console.error('Globe animation error:', e)
            }
          }
          animate()

          console.log('🌍 3D Globe initialized - peers will light up the world in RED!')
        } else {
          console.warn('Globe or THREE not defined yet')
        }
      } catch (error) {
        console.error('Globe initialization error:', error)
      }
    }, 1000) // Wait 1 second for libraries to fully load

    // Cleanup
    return () => {
      clearTimeout(initTimeout)
      if (animationIdRef.current !== null) {
        cancelAnimationFrame(animationIdRef.current)
      }
    }
  }, [])

  // Update globe with peer data (exact panel.html implementation)
  useEffect(() => {
    // Simulate peer data for demo - in production this would come from props
    const demoInterval = setInterval(() => {
      if (!globeRef.current) return

      // Real city coordinates for realistic globe visualization
      const cityCoords: Record<string, { lat: number; lng: number }> = {
        'New York': { lat: 40.7128, lng: -74.0060 },
        'London': { lat: 51.5074, lng: -0.1278 },
        'Tokyo': { lat: 35.6762, lng: 139.6503 },
        'Sydney': { lat: -33.8688, lng: 151.2093 },
        'Mumbai': { lat: 19.0760, lng: 72.8777 },
        'São Paulo': { lat: -23.5505, lng: -46.6333 },
        'Berlin': { lat: 52.5200, lng: 13.4050 },
        'Singapore': { lat: 1.3521, lng: 103.8198 },
        'Toronto': { lat: 43.6532, lng: -79.3832 },
        'Paris': { lat: 48.8566, lng: 2.3522 },
        'Seoul': { lat: 37.5665, lng: 126.9780 },
        'Dubai': { lat: 25.2048, lng: 55.2708 }
      }

      // Generate demo peers (in production, get from API)
      const demoPeers = [
        'vision-New York-1',
        'vision-London-2',
        'vision-Tokyo-3',
        'vision-Sydney-4',
        'vision-Mumbai-5',
        'vision-São Paulo-6',
        'vision-Berlin-7',
        'vision-Singapore-8'
      ]

      const peerLocations = demoPeers.map((peer, idx) => {
        let lat: number, lng: number

        // Extract city from peer string
        const cityMatch = peer.match(/vision-([^-]+)/)
        
        if (cityMatch) {
          const cityName = cityMatch[1]
          const coords = cityCoords[cityName]
          
          if (coords) {
            lat = coords.lat
            lng = coords.lng
          } else {
            // Hash-based random location if city not found
            const hash = peer.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0)
            lat = ((hash % 180) - 90) + (Math.sin(idx) * 20)
            lng = ((hash * 7) % 360) - 180
          }
        } else {
          // Fallback random positioning
          const hash = peer.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0)
          lat = ((hash % 180) - 90) + (Math.sin(idx) * 20)
          lng = ((hash * 7) % 360) - 180
        }

        return {
          lat: lat,
          lng: lng,
          size: 0.6 + Math.random() * 0.4,
          peer: peer
        }
      })

      // Update globe with red dots
      globeRef.current.pointsData(peerLocations)
        .pointLat('lat')
        .pointLng('lng')
        .pointColor(() => '#ff0000')
        .pointAltitude(0.01)
        .pointRadius((d: any) => d.size)
        .pointsMerge(false)

    }, 5000) // Update every 5 seconds

    return () => clearInterval(demoInterval)
  }, [])

  return (
    <div className="geo-map">
      <div className="globe-title">🌍 Peer Network</div>
      <div id="peer-map" ref={containerRef}></div>
      <div className="globe-stats">8 peers lighting up the world 🔴</div>
    </div>
  )
}
