import type { PointerEvent as ReactPointerEvent } from 'react'

export type ResizeDirection = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw'

const DIRECTIONS: ResizeDirection[] = ['n', 's', 'e', 'w', 'ne', 'nw', 'se', 'sw']

interface ResizeHandlesProps {
  onResizeStart: (direction: ResizeDirection, e: ReactPointerEvent) => void
}

export function ResizeHandles({ onResizeStart }: ResizeHandlesProps) {
  return (
    <>
      {DIRECTIONS.map((dir) => (
        <div
          key={dir}
          className={`ww-resize ${dir}`}
          onPointerDown={(e) => onResizeStart(dir, e)}
        />
      ))}
    </>
  )
}
