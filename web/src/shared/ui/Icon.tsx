import type { ReactNode } from 'react'
import { SystemIcon } from '../icons/SystemIcon'
import { IconSizeContext } from '../icons/icon-size'
import { iconUrlFallback } from '../lib/personalization'

interface IconProps {
  /**
   * a <SystemIcon>, an <svg>/<img> element, any node, or a string:
   * image URL (contains `/` or is a data: URI) or a glyph/emoji
   */
  icon?: ReactNode
  size?: number
  className?: string
}

const isImageUrl = (s: string) => s.includes('/') || s.startsWith('data:') || s.startsWith('blob:')

/** Square icon box: scales SVG/IMG children to `size` and tells <SystemIcon> which bitmap to use. */
export function Icon({ icon, size = 16, className = '' }: IconProps) {
  let content = icon ?? <SystemIcon name="preferences-system-windows" />
  if (typeof icon === 'string') {
    content = isImageUrl(icon) ? (
      <img
        src={icon}
        alt=""
        draggable={false}
        style={{ objectFit: 'contain' }}
        onError={(e) => {
          // icon missing from the active icon pack: use the default pack's one
          const fallback = iconUrlFallback(e.currentTarget.src)
          if (fallback) e.currentTarget.src = fallback
        }}
      />
    ) : (
      <span style={{ fontSize: size * 0.85 }}>{icon}</span>
    )
  }
  return (
    <span className={`ww-icon ${className}`} style={{ width: size, height: size }}>
      <IconSizeContext.Provider value={size}>{content}</IconSizeContext.Provider>
    </span>
  )
}
