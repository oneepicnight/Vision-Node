// Vision World Globe - Cinematic CSS Implementation
// AAA game-quality idle animation with layered depth

import './VisionGlobe.css'

export default function VisionGlobe() {
  return (
    <div className="vision-globe-container">
      {/* Deep space background with nebula */}
      <div className="vision-space"></div>
      
      {/* Starfield layer */}
      <div className="vision-stars"></div>
      
      {/* Main Earth sphere */}
      <div className="vision-earth">
        {/* Cloud layer with parallax motion */}
        <div className="vision-clouds"></div>
        
        {/* Inner depth vignette */}
        <div className="vision-inner-shadow"></div>
      </div>
      
      {/* Atmosphere rim light */}
      <div className="vision-atmosphere"></div>
      
      {/* Cinematic lens effects */}
      <div className="vision-lens-effects"></div>
    </div>
  )
}

