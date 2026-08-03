import {
  PlayRegular,
  PauseRegular,
  Speaker2Regular,
  SpeakerMuteRegular,
  FullScreenMaximizeRegular,
  FullScreenMinimizeRegular,
} from "@fluentui/react-icons";
import { formatTime } from "../lib/utils";

interface MediaControlsProps {
  controlsVisible: boolean;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  isMuted: boolean;
  isFullscreen: boolean;
  onPlayToggle: () => void;
  onMuteToggle: () => void;
  onFullscreenToggle: () => void;
  onSeek: (time: number) => void;
  onVolumeChange: (vol: number) => void;
}

export function MediaControls({
  controlsVisible,
  isPlaying,
  currentTime,
  duration,
  volume,
  isMuted,
  isFullscreen,
  onPlayToggle,
  onMuteToggle,
  onFullscreenToggle,
  onSeek,
  onVolumeChange,
}: MediaControlsProps) {
  return (
    <div
      className={`absolute bottom-0 left-0 right-0 p-4 pt-12 bg-gradient-to-t from-[rgba(0,0,0,0.8)] to-transparent transition-opacity duration-300 flex flex-col gap-2 ${
        controlsVisible || !isPlaying ? "opacity-100" : "opacity-0 pointer-events-none"
      }`}
    >
      {/* Progress Bar */}
      <div className="flex items-center gap-3">
        <span className="text-[11px] font-medium min-w-[35px] text-right">
          {formatTime(currentTime)}
        </span>
        <input
          type="range"
          min="0"
          max={duration || 0}
          value={currentTime}
          onChange={(e) => onSeek(Number(e.target.value))}
          className="flex-1 h-1.5 bg-[#444] rounded-full appearance-none cursor-pointer accent-[#60cdff] hover:accent-[#28c3ff]"
        />
        <span className="text-[11px] font-medium min-w-[35px]">
          {formatTime(duration)}
        </span>
      </div>

      {/* Bottom Controls */}
      <div className="flex items-center gap-2 mt-1">
        <button
          onClick={onPlayToggle}
          className="w-10 h-10 flex items-center justify-center rounded-full hover:bg-[rgba(255,255,255,0.1)] transition-colors"
        >
          {isPlaying ? (
            <PauseRegular fontSize={24} />
          ) : (
            <PlayRegular fontSize={24} />
          )}
        </button>

        <div className="flex items-center gap-2 group/volume relative">
          <button
            onClick={onMuteToggle}
            className="w-10 h-10 flex items-center justify-center rounded-full hover:bg-[rgba(255,255,255,0.1)] transition-colors"
          >
            {isMuted || volume === 0 ? (
              <SpeakerMuteRegular fontSize={20} />
            ) : (
              <Speaker2Regular fontSize={20} />
            )}
          </button>
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={isMuted ? 0 : volume}
            onChange={(e) => onVolumeChange(Number(e.target.value))}
            className="w-0 opacity-0 group-hover/volume:w-20 group-hover/volume:opacity-100 transition-all duration-300 h-1.5 bg-[#444] rounded-full appearance-none cursor-pointer accent-[#60cdff]"
          />
        </div>

        <div className="flex-1"></div>

        <button
          onClick={onFullscreenToggle}
          className="w-10 h-10 flex items-center justify-center rounded-full hover:bg-[rgba(255,255,255,0.1)] transition-colors"
        >
          {isFullscreen ? (
            <FullScreenMinimizeRegular fontSize={20} />
          ) : (
            <FullScreenMaximizeRegular fontSize={20} />
          )}
        </button>
      </div>
    </div>
  );
}
