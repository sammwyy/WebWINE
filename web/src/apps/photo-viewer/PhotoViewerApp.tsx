import { useEffect, useState, useRef } from "react";
import { useWindowStore } from "@/state/windowStore";
import { WindowTitlebar } from "@/modules/windows/WindowTitlebar";
import type {
  RuntimeBridge,
  DirectoryEntry,
} from "@/core/bridge/runtime-bridge";
import { basename } from "@/shared/lib/utils";
import { resolveIcon } from "@/shared/lib/icons/icon-resolver";
import {
  ZoomInRegular,
  ZoomOutRegular,
  MaximizeRegular,
  ArrowRotateCounterclockwiseRegular,
  ArrowRotateClockwiseRegular,
  DeleteRegular,
  ChevronLeftFilled,
  ChevronRightFilled,
  FullScreenMaximizeRegular,
  FullScreenMinimizeRegular,
} from "@fluentui/react-icons";

const IMAGE_EXTENSIONS = [
  ".png",
  ".jpg",
  ".jpeg",
  ".gif",
  ".webp",
  ".bmp",
  ".ico",
  ".svg",
];

function isImage(name: string) {
  const lower = name.toLowerCase();
  return IMAGE_EXTENSIONS.some((ext) => lower.endsWith(ext));
}

function dirname(path: string) {
  const parts = path.split("\\");
  parts.pop();
  return parts.join("\\") || path;
}

export async function openPhotoViewer(path: string, runtime: RuntimeBridge) {
  const name = path ? basename(path) : "Pictures";

  const resolved = await resolveIcon(
    { name, path, kind: "file", size: 0 },
    runtime,
  );

  const icon =
    resolved?.src ||
    `${import.meta.env.BASE_URL}theme/icons/places/pictures.webp`;

  let winId = "";

  winId = useWindowStore.getState().openWindow({
    title: path ? `${name} - WebWINE: Photo Viewer` : "WebWINE: Photo Viewer",
    icon,
    width: 800,
    height: 600,
    hideTitlebar: true,
    content: (
      <PhotoViewerApp
        initialPath={path}
        runtime={runtime}
        winId={() => winId}
      />
    ),
  });
}

export function PhotoViewerApp({
  initialPath,
  runtime,
  winId,
}: {
  initialPath: string;
  runtime: RuntimeBridge;
  winId?: () => string;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [images, setImages] = useState<DirectoryEntry[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [currentUrl, setCurrentUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [zoom, setZoom] = useState(1);
  const [rotation, setRotation] = useState(0);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isFullscreen, setIsFullscreen] = useState(false);
  const isDragging = useRef(false);
  const lastMousePos = useRef({ x: 0, y: 0 });

  useEffect(() => {
    let alive = true;

    async function init() {
      try {
        setLoading(true);
        let dirPath = "C:\\Users\\guest\\Pictures";
        let targetFile = "";

        if (initialPath) {
          const isFile = basename(initialPath).includes(".");
          if (isFile) {
            dirPath = dirname(initialPath);
            targetFile = basename(initialPath).toLowerCase();
          } else {
            dirPath = initialPath;
          }
        }

        const entries = await runtime.listDir(dirPath);
        if (!alive) return;

        let imgEntries = entries.filter(
          (e) => e.kind === "file" && isImage(e.name),
        );
        imgEntries.sort((a, b) => a.name.localeCompare(b.name));

        if (targetFile) {
          const singleFileEntry = imgEntries.find(
            (e) => e.name.toLowerCase() === targetFile,
          );
          if (singleFileEntry) {
            imgEntries = [singleFileEntry];
          } else {
            // fallback if it somehow wasn't in listDir but we know the path
            imgEntries = [
              { name: targetFile, path: initialPath, kind: "file", size: 0 },
            ];
          }
        }

        setImages(imgEntries);
        setCurrentIndex(0);
      } catch (err) {
        if (alive) setError(String(err));
      } finally {
        if (alive) setLoading(false);
      }
    }

    init();

    return () => {
      alive = false;
    };
  }, [initialPath, runtime]);

  const currentImage = images[currentIndex];

  useEffect(() => {
    if (!currentImage) {
      setCurrentUrl(null);
      return;
    }

    let alive = true;
    setLoading(true);
    setError(null);
    setZoom(1);
    setRotation(0);
    setPan({ x: 0, y: 0 });

    if (winId) {
      const id = winId();
      if (id) {
        useWindowStore
          .getState()
          .setTitle(id, `${currentImage.name} - WebWINE: Photo Viewer`);
      }
    }

    runtime
      .readFile(currentImage.path)
      .then((bytes) => {
        if (!alive) return;
        const blob = new Blob([bytes]);
        const url = URL.createObjectURL(blob);
        setCurrentUrl(url);
        setLoading(false);
      })
      .catch((err) => {
        if (!alive) return;
        setError(String(err));
        setLoading(false);
      });

    return () => {
      alive = false;
      setCurrentUrl((url) => {
        if (url) URL.revokeObjectURL(url);
        return null;
      });
    };
  }, [currentImage, runtime, winId]);

  const prev = () => {
    if (images.length > 0) {
      setCurrentIndex((i) => (i - 1 + images.length) % images.length);
    }
  };

  const next = () => {
    if (images.length > 0) {
      setCurrentIndex((i) => (i + 1) % images.length);
    }
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "ArrowLeft") prev();
      else if (e.key === "ArrowRight") next();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [images]);

  useEffect(() => {
    const onFullscreenChange = () => {
      setIsFullscreen(!!document.fullscreenElement);
    };
    document.addEventListener("fullscreenchange", onFullscreenChange);
    return () =>
      document.removeEventListener("fullscreenchange", onFullscreenChange);
  }, []);

  const toggleFullscreen = () => {
    if (!document.fullscreenElement) {
      containerRef.current?.requestFullscreen().catch(() => {});
    } else {
      document.exitFullscreen().catch(() => {});
    }
  };

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const zoomFactor = 0.1;
    if (e.deltaY < 0) setZoom((z) => Math.min(z + zoomFactor, 5));
    else setZoom((z) => Math.max(z - zoomFactor, 0.1));
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    isDragging.current = true;
    lastMousePos.current = { x: e.clientX, y: e.clientY };
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging.current) return;
    const dx = e.clientX - lastMousePos.current.x;
    const dy = e.clientY - lastMousePos.current.y;
    setPan((p) => ({ x: p.x + dx, y: p.y + dy }));
    lastMousePos.current = { x: e.clientX, y: e.clientY };
  };

  const handleMouseUp = () => {
    isDragging.current = false;
  };

  const deleteCurrentImage = async () => {
    if (!currentImage) return;
    if (!confirm(`Are you sure you want to delete ${currentImage.name}?`))
      return;
    try {
      await runtime.deleteNode(currentImage.path);
      setImages((imgs) => imgs.filter((img) => img.path !== currentImage.path));
      if (images.length <= 1) {
        setCurrentIndex(0); // will be empty now
      } else if (currentIndex >= images.length - 1) {
        setCurrentIndex(images.length - 2);
      }
    } catch (err) {
      alert(`Failed to delete file: ${err}`);
    }
  };

  return (
    <div
      ref={containerRef}
      className="flex flex-col w-full h-full bg-[#111111] select-none font-[var(--system-font)] text-[#f2f2f2] overflow-hidden group"
    >
      {/* Top Title/Toolbar Area - Custom WindowTitlebar */}
      <WindowTitlebar
        windowId={winId?.() || ""}
        className="!bg-[#111111] !border-[#222222] !border-b !h-12 !px-2 z-10"
      >
        <div className="flex items-center gap-1 window-controls">
          <button
            onClick={() => setRotation((r) => r - 90)}
            className="w-9 h-9 flex items-center justify-center rounded hover:bg-[#2b2b2b] text-[#f2f2f2] transition-colors"
            title="Rotate Left"
          >
            <ArrowRotateCounterclockwiseRegular fontSize={18} />
          </button>
          <button
            onClick={() => setRotation((r) => r + 90)}
            className="w-9 h-9 flex items-center justify-center rounded hover:bg-[#2b2b2b] text-[#f2f2f2] transition-colors"
            title="Rotate Right"
          >
            <ArrowRotateClockwiseRegular fontSize={18} />
          </button>
          <div className="w-px h-5 bg-[#333333] mx-1" />
          <button
            onClick={() => setZoom((z) => Math.min(z + 0.25, 5))}
            className="w-9 h-9 flex items-center justify-center rounded hover:bg-[#2b2b2b] text-[#f2f2f2] transition-colors"
            title="Zoom In"
          >
            <ZoomInRegular fontSize={18} />
          </button>
          <button
            onClick={() => setZoom((z) => Math.max(z - 0.25, 0.1))}
            className="w-9 h-9 flex items-center justify-center rounded hover:bg-[#2b2b2b] text-[#f2f2f2] transition-colors"
            title="Zoom Out"
          >
            <ZoomOutRegular fontSize={18} />
          </button>
          <button
            onClick={() => {
              setZoom(1);
              setRotation(0);
              setPan({ x: 0, y: 0 });
            }}
            className="w-9 h-9 flex items-center justify-center rounded hover:bg-[#2b2b2b] text-[#f2f2f2] transition-colors"
            title="Fit to window"
          >
            <MaximizeRegular fontSize={18} />
          </button>
          <div className="w-px h-5 bg-[#333333] mx-1" />
          <button
            onClick={toggleFullscreen}
            className="w-9 h-9 flex items-center justify-center rounded hover:bg-[#2b2b2b] text-[#f2f2f2] transition-colors"
            title="Fullscreen"
          >
            {isFullscreen ? (
              <FullScreenMinimizeRegular fontSize={18} />
            ) : (
              <FullScreenMaximizeRegular fontSize={18} />
            )}
          </button>
          <button
            onClick={deleteCurrentImage}
            className="w-9 h-9 flex items-center justify-center rounded hover:bg-[#ff4444] text-[#f2f2f2] hover:text-white transition-colors"
            title="Delete"
          >
            <DeleteRegular fontSize={18} />
          </button>
        </div>

        <div className="absolute left-1/2 -translate-x-1/2 flex items-center pointer-events-none">
          <span className="truncate max-w-[300px] text-[13px] font-semibold text-[#f2f2f2]">
            {currentImage?.name || "Photos"}
          </span>
        </div>
      </WindowTitlebar>

      {/* Main Image Area */}
      <div
        className="flex-1 relative overflow-hidden bg-[#1a1a1a]"
        onWheel={handleWheel}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        style={{ cursor: isDragging.current ? "grabbing" : "grab" }}
      >
        {loading && !currentUrl && (
          <div className="absolute inset-0 flex items-center justify-center text-[#888888]">
            Loading...
          </div>
        )}
        {error && (
          <div className="absolute inset-0 flex items-center justify-center text-[#ff6b6b] text-center p-4">
            {error}
          </div>
        )}
        {!loading && !error && images.length === 0 && (
          <div className="absolute inset-0 flex items-center justify-center text-[#888888]">
            We didn't find any pictures.
          </div>
        )}
        {currentUrl && (
          <div className="absolute inset-0 flex items-center justify-center">
            <img
              src={currentUrl}
              alt={currentImage?.name}
              draggable={false}
              style={{
                transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom}) rotate(${rotation}deg)`,
                transition: isDragging.current
                  ? "none"
                  : "transform 0.1s ease-out",
                maxHeight: "100%",
                maxWidth: "100%",
                objectFit: "contain",
              }}
            />
          </div>
        )}

        {/* Floating Navigation Controls */}
        {images.length > 1 && (
          <>
            <button
              onClick={prev}
              className="absolute left-4 top-1/2 -translate-y-1/2 w-12 h-12 flex items-center justify-center rounded-full bg-black/50 hover:bg-black/80 text-white opacity-0 group-hover:opacity-100 transition-opacity z-10"
              title="Previous"
            >
              <ChevronLeftFilled fontSize={24} />
            </button>
            <button
              onClick={next}
              className="absolute right-4 top-1/2 -translate-y-1/2 w-12 h-12 flex items-center justify-center rounded-full bg-black/50 hover:bg-black/80 text-white opacity-0 group-hover:opacity-100 transition-opacity z-10"
              title="Next"
            >
              <ChevronRightFilled fontSize={24} />
            </button>
          </>
        )}
      </div>

      {/* Bottom Filmstrip (optional placeholder for Win10 look) */}
      {images.length > 1 && (
        <div className="h-16 flex-none flex items-center justify-center bg-[#111111] px-4 gap-2 text-[12px] text-[#888888]">
          {currentIndex + 1} of {images.length}
        </div>
      )}
    </div>
  );
}
