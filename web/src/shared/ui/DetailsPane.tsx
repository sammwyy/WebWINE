import type { HTMLAttributes, ReactNode } from 'react'
import { Icon } from './Icon'

interface DetailsPaneProps {
  icon?: ReactNode
  title?: ReactNode
  /** label → value pairs */
  properties?: [ReactNode, ReactNode][]
  children?: ReactNode
}

/** Vista bottom details pane (big icon, title and properties). */
export function DetailsPane({ icon, title, properties = [], children }: DetailsPaneProps) {
  return (
    <div className="ww-detailspane">
      {icon != null && <Icon icon={icon} size={48} />}
      {title && <div className="ww-detailspane-title">{title}</div>}
      {properties.length > 0 && (
        <dl className="ww-detailspane-props">
          {properties.map(([label, value], i) => (
            <div key={i} className="contents">
              <dt>{label}:</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
      )}
      {children}
    </div>
  )
}

/** Bottom button row of a dialog (OK / Cancel / Apply), right-aligned. */
export function DialogFooter({ className = '', ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={`ww-dialog-footer ${className}`} {...props} />
}

export function StatusBar({ className = '', ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={`ww-statusbar ${className}`} {...props} />
}
