import { createElement } from 'react'
import { ArrowGlyph, ChevronDownGlyph } from '../icons'
import { useThemed } from './themed'

export interface NavButtonsProps {
  canGoBack?: boolean
  canGoForward?: boolean
  /** parent location exists (used by themes that show an "Up" button, e.g. XP) */
  canGoUp?: boolean
  onBack?: () => void
  onForward?: () => void
  onUp?: () => void
}

/** Round glossy back/forward buttons (meant to sit on a GlassPanel). */
function DefaultNavButtons({ canGoBack = false, canGoForward = false, onBack, onForward }: NavButtonsProps) {
  return (
    <div className="ww-navbuttons">
      <button className="ww-navbtn" title="Back" disabled={!canGoBack} onClick={onBack}>
        <ArrowGlyph direction="left" />
      </button>
      <button className="ww-navbtn" title="Forward" disabled={!canGoForward} onClick={onForward}>
        <ArrowGlyph direction="right" />
      </button>
      <span className="ww-navbtn-menu">
        <ChevronDownGlyph width={7} height={7} />
      </span>
    </div>
  )
}

/** Themeable: themes can replace it via `components.ui.NavButtons`. */
export function NavButtons(props: NavButtonsProps) {
  return createElement(useThemed('NavButtons', DefaultNavButtons), props)
}
