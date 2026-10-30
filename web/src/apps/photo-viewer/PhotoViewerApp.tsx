import { useEffect, useState, useRef } from "react";
import { useWindowStore } from "@/state/windowStore";
import { Command, CommandBar, CommandSpacer } from "@/shared/ui";
import { launchGuestPath } from "@/shared/lib/guest-launch";
import { showMessageBox } from "../message-box/MessageBoxApp";
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
  PreviousRegular,
  NextRegular,
  PlayRegular,
  PauseRegular,
  FullScreenMaximizeRegular,
  FullScreenMinimizeRegular,
} from "@fluentui/react-icons";
import { iconUrl } from "@/shared/lib/personalization";

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
    iconUrl("places/pictures");

  let winId = "";

  winId = useWindowStore.getState().openWindow({
    title: path ? `${name} - WebWINE: Photo Viewer` : "WebWINE: Photo Viewer",
    icon,
    width: 800,
    height: 600,
    minWidth: 420,
    minHeight: 300,
    surface: "none",
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
  const [slideShow, setSlideShow] = useState(false);
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
      // only the focused viewer reacts to the arrow keys
      if (!containerRef.current?.closest(".ww-window.active")) return;
      if (e.key === "ArrowLeft") prev();
      else if (e.key === "ArrowRight") next();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [images]);

  useEffect(() => {
    if (!slideShow || images.length < 2) return;
    const id = window.setInterval(() => setCurrentIndex((i) => (i + 1) % images.length), 3000);
    return () => window.clearInterval(id);
  }, [slideShow, images.length]);

  const toggleSlideShow = () => setSlideShow((v) => !v);

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
    const answer = await showMessageBox({
      kind: "message_box",
      title: "Delete File",
      text: `Are you sure you want to permanently delete "${currentImage.name}"?`,
      style: 0x04 | 0x30, // MB_YESNO | MB_ICONEXCLAMATION
    });
    if (answer !== "yes") return;
    try {
      await runtime.deleteNode(currentImage.path);
      setImages((imgs) => imgs.filter((img) => img.path !== currentImage.path));
      if (images.length <= 1) {
        setCurrentIndex(0); // will be empty now
      } else if (currentIndex >= images.length - 1) {
        setCurrentIndex(images.length - 2);
      }
    } catch (err) {
      setError(`Failed to delete file: ${err}`);
    }
  };

  const resetView = () => {
    setZoom(1);
    setRotation(0);
    setPan({ x: 0, y: 0 });
  };

  return (
    <div ref={containerRef} className="flex flex-col w-full h-full select-none overflow-hidden">
      <CommandBar>
        <Command
          icon={iconUrl("apps/explorer")}
          onClick={() => void launchGuestPath(currentImage ? dirname(currentImage.path) : "C:\\Users\\guest\\Pictures", runtime)}
        >
          Open folder
        </Command>
        <Command icon={<DeleteRegular />} disabled={!currentImage} onClick={() => void deleteCurrentImage()}>
          Delete
        </Command>
        <CommandSpacer />
        <Command
          icon={isFullscreen ? <FullScreenMinimizeRegular /> : <FullScreenMaximizeRegular />}
          onClick={toggleFullscreen}
        >
          {isFullscreen ? "Exit full screen" : "Full screen"}
        </Command>
      </CommandBar>

      <div
        className="pv-stage flex-1 relative overflow-hidden"
        onWheel={handleWheel}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onDoubleClick={resetView}
        style={{ cursor: zoom > 1 ? "grab" : "default" }}
      >
        {loading && !currentUrl && <div className="absolute inset-0 grid place-items-center ww-muted">Loading…</div>}
        {error && <div className="absolute inset-0 grid place-items-center p-4 text-center text-[var(--ww-danger)]">{error}</div>}
        {!loading && !error && images.length === 0 && (
          <div className="absolute inset-0 grid place-items-center ww-muted">There are no pictures in this folder.</div>
        )}
        {currentUrl && (
          <div className="absolute inset-0 flex items-center justify-center p-3">
            <img
              src={currentUrl}
              alt={currentImage?.name}
              draggable={false}
              className="pv-image"
              style={{
                transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom}) rotate(${rotation}deg)`,
                transition: isDragging.current ? "none" : "transform 0.1s ease-out",
              }}
            />
          </div>
        )}
      </div>

      <div className="ww-mediabar justify-center">
        <span className="ww-mediabar-info">{images.length > 0 ? `${currentIndex + 1} of ${images.length}` : ""}</span>
        <button type="button" className="ww-media-btn" title="Zoom in" onClick={() => setZoom((z) => Math.min(z + 0.25, 5))}>
          <ZoomInRegular />
        </button>
        <button type="button" className="ww-media-btn" title="Zoom out" onClick={() => setZoom((z) => Math.max(z - 0.25, 0.1))}>
          <ZoomOutRegular />
        </button>
        <button type="button" className="ww-media-btn" title="Actual size / fit" onClick={resetView}>
          <MaximizeRegular />
        </button>
        <span className="ww-mediabar-sep" />
        <button type="button" className="ww-media-btn" title="Previous (Left arrow)" disabled={images.length < 2} onClick={prev}>
          <PreviousRegular />
        </button>
        <button type="button" className="ww-media-btn main" title="Play slide show" disabled={images.length < 2} onClick={toggleSlideShow}>
          {slideShow ? <PauseRegular /> : <PlayRegular />}
        </button>
        <button type="button" className="ww-media-btn" title="Next (Right arrow)" disabled={images.length < 2} onClick={next}>
          <NextRegular />
        </button>
        <span className="ww-mediabar-sep" />
        <button type="button" className="ww-media-btn" title="Rotate counterclockwise" onClick={() => setRotation((r) => r - 90)}>
          <ArrowRotateCounterclockwiseRegular />
        </button>
        <button type="button" className="ww-media-btn" title="Rotate clockwise" onClick={() => setRotation((r) => r + 90)}>
          <ArrowRotateClockwiseRegular />
        </button>
        <button type="button" className="ww-media-btn" title="Delete" disabled={!currentImage} onClick={() => void deleteCurrentImage()}>
          <DeleteRegular />
        </button>
        <span className="ww-mediabar-info text-right">{Math.round(zoom * 100)}%</span>
      </div>
    </div>
  );
}