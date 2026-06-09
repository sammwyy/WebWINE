import { MusicNote1Regular } from "@fluentui/react-icons";

interface AudioOverlayProps {
  metadata: {
    title?: string;
    artist?: string;
    album?: string;
    pictureUrl?: string;
  } | null;
  fallbackName: string;
}

export function AudioOverlay({ metadata, fallbackName }: AudioOverlayProps) {
  return (
    <div
      className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none"
      style={{
        background: metadata?.pictureUrl
          ? `linear-gradient(rgba(0,0,0,0.8), rgba(0,0,0,0.95)), url(${metadata.pictureUrl}) center/cover no-repeat`
          : "none",
      }}
    >
      {metadata?.pictureUrl ? (
        <img
          src={metadata.pictureUrl}
          alt="Cover"
          className="w-56 h-56 object-cover rounded-xl shadow-2xl mb-6 ring-1 ring-white/10"
        />
      ) : (
        <div className="w-56 h-56 bg-[#222] rounded-xl shadow-2xl mb-6 flex items-center justify-center ring-1 ring-white/5">
          <MusicNote1Regular fontSize={64} className="text-[#555]" />
        </div>
      )}
      <div className="text-center px-8 max-w-2xl">
        <h2 className="text-2xl font-bold text-white mb-1 truncate">
          {metadata?.title || fallbackName}
        </h2>
        <p className="text-lg text-[#aaa] truncate">
          {metadata?.artist || "Unknown Artist"}
        </p>
        {metadata?.album && (
          <p className="text-sm text-[#777] mt-1 truncate">
            {metadata.album}
          </p>
        )}
      </div>
    </div>
  );
}
