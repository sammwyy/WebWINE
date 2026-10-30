import type { ButtonHTMLAttributes, HTMLAttributes, ReactNode } from 'react'
import { ChevronDownGlyph, SystemIcon } from '../icons'
import { Icon } from './Icon'

/** The blue Vista Explorer toolbar. */
export function CommandBar({ className = '', ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={`ww-commandbar ${className}`} role="toolbar" {...props} />
}

interface CommandProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon?: ReactNode
  /** show the drop-down caret */
  dropdown?: boolean
}

export function Command({ icon, dropdown = false, children, className = '', ...props }: CommandProps) {
  return (
    <button className={`ww-command ${className}`} {...props}>
      {icon && <Icon icon={icon} size={16} />}
      {children}
      {dropdown && <ChevronDownGlyph className="ww-command-caret" width={7} height={7} />}
    </button>
  )
}

export function CommandSpacer() {
  return <div className="ww-commandbar-spacer" />
}

export function HelpCommand(props: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button className="ww-command ww-command-help" title="Help" {...props}>
      <SystemIcon name="help-contents" size={16} />
    </button>
  )
}
