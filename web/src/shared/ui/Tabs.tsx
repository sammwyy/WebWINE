import { useState } from 'react'
import type { ReactNode } from 'react'

export interface TabItem {
  id: string
  label: ReactNode
  content: ReactNode
}

interface TabsProps {
  tabs: TabItem[]
  /** controlled selected tab id */
  value?: string
  defaultValue?: string
  onChange?: (id: string) => void
  className?: string
}

export function Tabs({ tabs, value, defaultValue, onChange, className = '' }: TabsProps) {
  const [internal, setInternal] = useState(defaultValue ?? tabs[0]?.id)
  const selected = value ?? internal
  const current = tabs.find((t) => t.id === selected) ?? tabs[0]

  const select = (id: string) => {
    setInternal(id)
    onChange?.(id)
  }

  return (
    <div className={`ww-tabs ${className}`}>
      <div className="ww-tablist" role="tablist">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={tab.id === current?.id}
            className={`ww-tab ${tab.id === current?.id ? 'selected' : ''}`}
            onClick={() => select(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div className="ww-tabpanel" role="tabpanel">
        {current?.content}
      </div>
    </div>
  )
}
