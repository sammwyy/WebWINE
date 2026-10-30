import { createContext } from 'react'

/** Pixel size of the enclosing <Icon> box, so raster icons can pick the best bitmap. */
export const IconSizeContext = createContext<number | null>(null)
