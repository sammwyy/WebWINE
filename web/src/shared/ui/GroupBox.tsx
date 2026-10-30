import type { HTMLAttributes, ReactNode } from 'react'

interface GroupBoxProps extends Omit<HTMLAttributes<HTMLFieldSetElement>, 'title'> {
  title: ReactNode
}

export function GroupBox({ title, children, className = '', ...props }: GroupBoxProps) {
  return (
    <fieldset className={`ww-groupbox ${className}`} {...props}>
      <legend>{title}</legend>
      {children}
    </fieldset>
  )
}
