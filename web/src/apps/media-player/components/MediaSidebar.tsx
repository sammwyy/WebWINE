import {
  PlayRegular,
  VideoRegular,
  MusicNote1Regular,
} from "@fluentui/react-icons";
import type { DirectoryEntry } from "@/core/bridge/runtime-bridge";
import { isVideoFile } from "../lib/utils";

interface MediaSidebarProps {
  sidebarOpen: boolean;
  initialPath: string;
  playlist: DirectoryEntry[];
  currentIndex: number;
  loading: boolean;
  onSelect: (index: number) => void;
}

export function MediaSidebar({
  sidebarOpen,
  initialPath,
  playlist,
  currentIndex,
  loading,
  onSelect,
}: MediaSidebarProps) {
  return (
    <div
      className={`flex-none flex flex-col bg-[#1a1a1a] border-r border-[#333333] transition-all duration-300 ease-in-out ${
        sidebarOpen ? "w-64" : "w-0 opacity-0 overflow-hidden"
      }`}
    >
      <div className="h-12 px-4 flex items-center border-b border-[#333333] font-semibold text-[13px] whitespace-nowrap">
        {initialPath ? "Folder Playlist" : "Media Library"}
      </div>
      <div className="flex-1 overflow-y-auto py-2 px-1 custom-scrollbar">
        {playlist.length === 0 && !loading && (
          <div className="text-center text-[#888] text-[12px] mt-4">
            No media found.
          </div>
        )}
        {playlist.map((item, idx) => {
          const active = idx === currentIndex;
          return (
            <div
              key={item.path}
              onClick={() => onSelect(idx)}
              className={`flex items-center gap-3 px-3 py-2 mx-1 rounded cursor-pointer transition-colors text-[13px] ${
                active
                  ? "bg-[#333333] text-white"
                  : "text-[#cccccc] hover:bg-[#2b2b2b]"
              }`}
            >
              <div className="flex-none flex items-center justify-center w-4 text-[#888888]">
                {active ? (
                  <PlayRegular fontSize={14} className="text-white" />
                ) : isVideoFile(item.name) ? (
                  <VideoRegular fontSize={14} />
                ) : (
                  <MusicNote1Regular fontSize={14} />
                )}
              </div>
              <div className="flex-1 truncate" title={item.name}>
                {item.name}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
