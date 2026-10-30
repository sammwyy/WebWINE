import { useState } from 'react'
import type { CSSProperties, ReactNode } from 'react'
import { ChevronUpGlyph } from '../icons'

export interface ListColumn<T> {
  key: string
  label: ReactNode
  width?: number
  align?: 'left' | 'right'
  /** muted grey text (secondary columns) */
  muted?: boolean
  render: (item: T) => ReactNode
}

export interface ListGroup<T> {
  id: string
  label: ReactNode
  items: T[]
}

interface ListViewProps<T> {
  columns: ListColumn<T>[]
  /** flat items — or use `groups` */
  items?: T[]
  groups?: ListGroup<T>[]
  getKey: (item: T) => string
  view?: 'details' | 'tiles'
  /** content of a tile in `tiles` view */
  renderTile?: (item: T) => ReactNode
  /** column key that shows the sort arrow */
  sortedBy?: string
  selectedKey?: string | null
  onSelect?: (item: T | null) => void
  /** double-click / Enter */
  onActivate?: (item: T) => void
  className?: string
}

/** Vista list view with column headers, collapsible groups and details/tiles views. */
export function ListView<T>({
  columns,
  items,
  groups,
  getKey,
  view = 'details',
  renderTile,
  sortedBy,
  selectedKey,
  onSelect,
  onActivate,
  className = '',
}: ListViewProps<T>) {
  const [internalSel, setInternalSel] = useState<string | null>(null)
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set())
  const selected = selectedKey !== undefined ? selectedKey : internalSel

  const select = (item: T | null) => {
    setInternalSel(item ? getKey(item) : null)
    onSelect?.(item)
  }

  const toggleGroup = (id: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })

  const colStyle = (col: ListColumn<T>): CSSProperties => ({
    width: col.width ?? 120,
    justifyContent: col.align === 'right' ? 'flex-end' : undefined,
    textAlign: col.align,
  })

  const renderItems = (list: T[]) => {
    if (view === 'tiles') {
      return (
        <div className="ww-listview-tiles">
          {list.map((item) => {
            const key = getKey(item)
            return (
              <div
                key={key}
                className={`ww-listview-tile ${key === selected ? 'selected' : ''}`}
                onClick={(e) => {
                  e.stopPropagation()
                  select(item)
                }}
                onDoubleClick={() => onActivate?.(item)}
              >
                {renderTile?.(item) ?? columns[0]?.render(item)}
              </div>
            )
          })}
        </div>
      )
    }
    return list.map((item) => {
      const key = getKey(item)
      return (
        <div
          key={key}
          className={`ww-listview-row ${key === selected ? 'selected' : ''}`}
          onClick={(e) => {
            e.stopPropagation()
            select(item)
          }}
          onDoubleClick={() => onActivate?.(item)}
        >
          {columns.map((col) => (
            <div key={col.key} className={`ww-listview-cell ${col.muted ? 'muted' : ''}`} style={colStyle(col)}>
              {col.render(item)}
            </div>
          ))}
        </div>
      )
    })
  }

  const selectedItem = () =>
    (groups?.flatMap((g) => g.items) ?? items ?? []).find((i) => getKey(i) === selected)

  return (
    <div
      className={`ww-listview ${className}`}
      tabIndex={0}
      onKeyDown={(e) => {
        const item = selectedItem()
        if (e.key === 'Enter' && item) onActivate?.(item)
      }}
    >
      <div className="ww-listview-header">
        {columns.map((col) => (
          <div
            key={col.key}
            className={`ww-listview-col ${col.key === sortedBy ? 'sorted' : ''}`}
            style={colStyle(col)}
          >
            {col.label}
          </div>
        ))}
      </div>
      <div className="ww-listview-body" onClick={() => select(null)}>
        {groups
          ? groups.map((group) => (
              <div key={group.id}>
                <div className="ww-listview-group" onClick={(e) => {
                  e.stopPropagation()
                  toggleGroup(group.id)
                }}>
                  <span>
                    {group.label} ({group.items.length})
                  </span>
                  <ChevronUpGlyph
                    className="toggle"
                    style={{ rotate: collapsed.has(group.id) ? '180deg' : undefined }}
                  />
                </div>
                {!collapsed.has(group.id) && renderItems(group.items)}
              </div>
            ))
          : renderItems(items ?? [])}
      </div>
    </div>
  )
}
