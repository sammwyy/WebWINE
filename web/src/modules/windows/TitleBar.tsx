import type { HTMLAttributes, ReactNode } from 'react'
import { Icon } from '@/shared/ui/Icon'

interface TitleBarProps extends Omit<HTMLAttributes<HTMLDivElement>, 'title'> {
  icon?: ReactNode
  title: ReactNode
  /** caption buttons, rendered at the right edge */
  children?: ReactNode
}

export function TitleBar({ icon, title, children, className = '', ...props }: TitleBarProps) {
  return (
    <div className={`ww-window-caption ${className}`} {...props}>
      <Icon icon={icon} size={16} />
      <div className="ww-window-title">
        <span>{title}</span>
      </div>
      {children}
    </div>
  )
}
