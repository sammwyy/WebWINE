import { useEffect, useRef, useState } from 'react'
import type { AddressBarProps, NavButtonsProps } from '@/shared/ui'
import { Icon } from '@/shared/ui'
import { ArrowGlyph, ChevronDownGlyph, SystemIcon } from '@/shared/icons'

/* glyph icons */

function UpFolderIcon() {
  return (
    <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true">
      <path d="M2.5 6.5h6l2 2h10v12h-18z" fill="#f6d77c" stroke="#b8902f" />
      <path d="M2.5 10.5h18v10h-18z" fill="#fbe49a" stroke="#b8902f" />
      <path d="M12 9.5 7.5 14h3v4.5h3V14h3z" fill="#3cb43c" stroke="#1d7a1d" strokeLinejoin="round" />
    </svg>
  )
}

function SearchIcon() {
  return (
    <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true">
      <circle cx="10" cy="10" r="6.5" fill="#d9ecff" stroke="#2c5aa0" strokeWidth="2" />
      <path d="M7 8.5a3.5 3.5 0 0 1 3-2.5" fill="none" stroke="#fff" strokeWidth="1.5" strokeLinecap="round" />
      <path d="M14.8 14.8 21 21" stroke="#5a4a2a" strokeWidth="3.2" strokeLinecap="round" />
    </svg>
  )
}

function GoIcon() {
  return (
    <svg viewBox="0 0 18 18" width="18" height="18" aria-hidden="true">
      <rect x="0.5" y="0.5" width="17" height="17" rx="3" fill="#3fae35" stroke="#1d6d17" />
      <rect x="1.5" y="1.5" width="15" height="7" rx="2" fill="#fff" opacity="0.3" />
      <path d="M4 9h7M8 5.5 11.5 9 8 12.5" fill="none" stroke="#fff" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

/* toolbar: Back / Forward / Up */

export function XpNavButtons({ canGoBack, canGoForward, canGoUp, onBack, onForward, onUp }: NavButtonsProps) {
  return (
    <div className="xp-toolbar">
      <button className="xp-tbtn" disabled={!canGoBack} onClick={onBack} title="Back">
        <span className="xp-nav-circle">
          <ArrowGlyph direction="left" />
        </span>
        <span>Back</span>
        <ChevronDownGlyph className="xp-caret" width={6} height={6} />
      </button>
      <button className="xp-tbtn" disabled={!canGoForward} onClick={onForward} title="Forward">
        <span className="xp-nav-circle">
          <ArrowGlyph direction="right" />
        </span>
        <ChevronDownGlyph className="xp-caret" width={6} height={6} />
      </button>
      <button className="xp-tbtn" disabled={!canGoUp} onClick={onUp} title="Up">
        <UpFolderIcon />
      </button>
    </div>
  )
}

/* toolbar: Search / Folders (replaces the search box) */

export function XpSearchButtons() {
  return (
    <div className="xp-toolbar">
      <span className="xp-tsep" />
      <button className="xp-tbtn" title="Search">
        <SearchIcon />
        <span>Search</span>
      </button>
      <button className="xp-tbtn" title="Folders">
        <SystemIcon name="folder" size={24} />
        <span>Folders</span>
      </button>
    </div>
  )
}

/* "Address" row: location combo with drop-down + Go */

export function XpAddressBar({ icon, path, onRefresh }: AddressBarProps) {
  const [open, setOpen] = useState(false)
  const comboRef = useRef<HTMLDivElement>(null)
  const current = path[path.length - 1]

  useEffect(() => {
    if (!open) return
    const onDown = (e: PointerEvent) => {
      if (!comboRef.current?.contains(e.target as Node)) setOpen(false)
    }
    window.addEventListener('pointerdown', onDown)
    return () => window.removeEventListener('pointerdown', onDown)
  }, [open])

  return (
    <div className="xp-addressrow">
      <span className="xp-address-label">Address</span>
      <div className="xp-combo" ref={comboRef}>
        <button className="xp-combo-field" onClick={() => setOpen((o) => !o)}>
          <Icon icon={current?.icon ?? icon} size={16} />
          <span>{current?.label}</span>
        </button>
        <button className="xp-combo-arrow" onClick={() => setOpen((o) => !o)} title="Show locations">
          <ChevronDownGlyph width={7} height={7} />
        </button>
        {open && (
          <div className="xp-combo-list">
            {path.map((crumb, i) => (
              <div
                key={i}
                className={`xp-combo-item ${i === path.length - 1 ? 'current' : ''}`}
                style={{ paddingLeft: 4 + i * 14 }}
                onClick={() => {
                  crumb.onClick?.()
                  setOpen(false)
                }}
              >
                <Icon icon={crumb.icon ?? icon} size={16} />
                <span>{crumb.label}</span>
              </div>
            ))}
          </div>
        )}
      </div>
      <button className="xp-go" onClick={onRefresh} title="Go">
        <GoIcon />
        <span>Go</span>
      </button>
    </div>
  )
}
