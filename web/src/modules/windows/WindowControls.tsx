import { useCurrentTheme } from '@/shared/lib/theme-registry'
import type { CaptionGlyphKind } from '@/shared/lib/theme-registry'

interface WindowControlsProps {
  maximized?: boolean
  minimizable?: boolean
  maximizable?: boolean
  onMinimize?: () => void
  onToggleMaximize?: () => void
  onClose?: () => void
}

/** Neutral thin-line glyphs; themes can replace them via `components.CaptionGlyph`. */
export function DefaultCaptionGlyph({ kind }: { kind: CaptionGlyphKind }) {
  const common = { width: 10, height: 10, viewBox: '0 0 10 10', fill: 'none', stroke: 'currentColor', strokeWidth: 1 }
  switch (kind) {
    case 'minimize':
      return <svg {...common}><path d="M0 5.5h10" /></svg>
    case 'maximize':
      return <svg {...common}><rect x="0.5" y="0.5" width="9" height="9" /></svg>
    case 'restore':
      return (
        <svg {...common}>
          <rect x="0.5" y="2.5" width="7" height="7" />
          <path d="M2.5 2.5v-2h7v7h-2" />
        </svg>
      )
    case 'close':
      return <svg {...common}><path d="M0.5 0.5l9 9M9.5 0.5l-9 9" /></svg>
  }
}

/** Caption buttons (minimize / maximize-restore / close). */
export function WindowControls({
  maximized = false,
  minimizable = true,
  maximizable = true,
  onMinimize,
  onToggleMaximize,
  onClose,
}: WindowControlsProps) {
  const Glyph = useCurrentTheme()?.components?.CaptionGlyph ?? DefaultCaptionGlyph

  return (
    <div
      className="ww-caption-buttons"
      onPointerDown={(e) => e.stopPropagation()}
      onDoubleClick={(e) => e.stopPropagation()}
    >
      {minimizable && (
        <button className="ww-caption-btn min" title="Minimize" onClick={onMinimize}>
          <Glyph kind="minimize" />
        </button>
      )}
      {maximizable && (
        <button
          className="ww-caption-btn max"
          title={maximized ? 'Restore Down' : 'Maximize'}
          onClick={onToggleMaximize}
        >
          <Glyph kind={maximized ? 'restore' : 'maximize'} />
        </button>
      )}
      <button className="ww-caption-btn close" title="Close" onClick={onClose}>
        <Glyph kind="close" />
      </button>
    </div>
  )
}
