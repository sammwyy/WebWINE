import type { HTMLAttributes, ReactNode } from 'react'
import { ChevronUpGlyph } from '../icons'
import { Icon } from './Icon'

interface NavPaneProps extends HTMLAttributes<HTMLDivElement> {
  width?: number
  /** label of the collapsible footer bar ("Folders") */
  footer?: ReactNode
}

/** Left navigation pane ("Favorite Links" / "Folders"). */
export function NavPane({ width = 170, footer, children, className = '', style, ...props }: NavPaneProps) {
  return (
    <nav className={`ww-navpane ${className}`} style={{ width, ...style }} {...props}>
      <div className="ww-navpane-scroll">{children}</div>
      {footer && (
        <div className="ww-navpane-footer">
          <span>{footer}</span>
          <ChevronUpGlyph />
        </div>
      )}
    </nav>
  )
}

export function NavSection({ title, children }: { title?: ReactNode; children?: ReactNode }) {
  return (
    <div className="ww-navsection">
      {title && <div className="ww-navsection-title">{title}</div>}
      {children}
    </div>
  )
}

interface NavLinkProps extends HTMLAttributes<HTMLDivElement> {
  icon?: ReactNode
  selected?: boolean
}

export function NavLink({ icon, selected = false, children, className = '', ...props }: NavLinkProps) {
  return (
    <div className={`ww-navlink ${selected ? 'selected' : ''} ${className}`} {...props}>
      {icon != null && <Icon icon={icon} size={16} />}
      {children}
    </div>
  )
}
