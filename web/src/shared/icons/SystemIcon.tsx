import { useContext } from 'react'
import type { ImgHTMLAttributes } from 'react'
import { IconSizeContext } from './icon-size'

/**
 * Icons from the Oxygen icon theme (KDE, LGPL-3) — see public/icons/oxygen/COPYING.
 * Bitmaps live in public/icons/oxygen/<size>/<name>.png and are loaded at runtime.
 */
export type SystemIconName =
  | 'application-x-shellscript'
  | 'applications-system'
  | 'computer'
  | 'document-properties'
  | 'document-share'
  | 'drive-harddisk'
  | 'drive-optical'
  | 'edit-delete'
  | 'folder'
  | 'folder-documents'
  | 'folder-image'
  | 'folder-remote'
  | 'folder-sound'
  | 'help-contents'
  | 'preferences-desktop-display'
  | 'preferences-desktop-theme'
  | 'preferences-desktop-wallpaper'
  | 'preferences-system'
  | 'preferences-system-windows'
  | 'text-x-generic'
  | 'tools-media-optical-burn'
  | 'user-identity'
  | 'user-trash'
  | 'view-list-details'
  | 'view-refresh'

const SIZES = [16, 32, 48, 64]

const pick = (px: number) => SIZES.find((s) => s >= px) ?? SIZES[SIZES.length - 1]
const url = (name: SystemIconName, size: number) => `${import.meta.env.BASE_URL}icons/oxygen/${size}/${name}.png`

interface SystemIconProps extends Omit<ImgHTMLAttributes<HTMLImageElement>, 'src'> {
  name: SystemIconName
  /** defaults to the size of the enclosing <Icon>, or 16 */
  size?: number
}

export function SystemIcon({ name, size, ...props }: SystemIconProps) {
  const ctxSize = useContext(IconSizeContext)
  const px = size ?? ctxSize ?? 16
  return (
    <img
      src={url(name, pick(px))}
      srcSet={`${url(name, pick(px))} 1x, ${url(name, pick(px * 2))} 2x`}
      width={px}
      height={px}
      alt=""
      draggable={false}
      {...props}
    />
  )
}
