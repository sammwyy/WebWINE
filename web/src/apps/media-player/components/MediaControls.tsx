import {
  PlayFilled,
  PauseFilled,
  StopFilled,
  PreviousFilled,
  NextFilled,
  ArrowShuffleFilled,
  ArrowRepeatAllFilled,
  Speaker2Filled,
  SpeakerMuteFilled,
  FullScreenMaximizeRegular,
  GridRegular,
  MoviesAndTvRegular,
} from "@fluentui/react-icons";
import { formatTime } from "../lib/utils";

interface MediaControlsProps {
  title: string;
  subtitle?: string;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  isMuted: boolean;
  shuffle: boolean;
  repeat: boolean;
  hasMedia: boolean;
  canPlay: boolean;
  nowPlaying: boolean;
  /** show the Library / Now Playing switch in the bar (WMP 12 Now Playing has it top-right instead) */
  showViewToggle: boolean;
  isFullscreen: boolean;
  onPlayToggle: () => void;
  onStop: () => void;
  onPrev: () => void;
  onNext: () => void;
  onShuffle: () => void;
  onRepeat: () => void;
  onMuteToggle: () => void;
  onVolumeChange: (vol: number) => void;
  onSeek: (time: number) => void;
  onToggleView: () => void;
  onFullscreenToggle: () => void;
}

/** The glossy black Windows Media Player 12 transport bar. */
export function MediaControls(p: MediaControlsProps) {
  const pct = p.duration > 0 ? (p.currentTime / p.duration) * 100 : 0;
  const vol = p.isMuted ? 0 : p.volume;

  return (
    <div className="wmp-bar">
      <input
        className="wmp-seek"
        type="range"
        min={0}
        max={p.duration || 0}
        step={0.1}
        value={p.currentTime}
        disabled={!p.hasMedia}
        style={{ "--pct": `${pct}%` } as React.CSSProperties}
        onChange={(e) => p.onSeek(Number(e.target.value))}
      />

      <div className="wmp-bar-row">
        <div className="wmp-bar-info">
          {p.hasMedia ? (
            <>
              <span className="truncate">{p.title}</span>
              <span className="wmp-bar-time">
                {formatTime(p.currentTime)}
                {p.subtitle ? ` · ${p.subtitle}` : ""}
              </span>
            </>
          ) : null}
        </div>

        <div className="wmp-transport">
          <button type="button" className={`wmp-btn ${p.shuffle ? "on" : ""}`} title="Turn shuffle on" onClick={p.onShuffle}>
            <ArrowShuffleFilled />
          </button>
          <button type="button" className={`wmp-btn ${p.repeat ? "on" : ""}`} title="Turn repeat on" onClick={p.onRepeat}>
            <ArrowRepeatAllFilled />
          </button>
          <span className="wmp-divider" />
          <button type="button" className="wmp-btn" title="Stop" disabled={!p.hasMedia} onClick={p.onStop}>
            <StopFilled />
          </button>
          <button type="button" className="wmp-btn" title="Previous" disabled={!p.hasMedia} onClick={p.onPrev}>
            <PreviousFilled />
          </button>
          <button
            type="button"
            className="wmp-play"
            title={p.isPlaying ? "Pause" : "Play"}
            disabled={!p.canPlay}
            onClick={p.onPlayToggle}
          >
            {p.isPlaying ? <PauseFilled /> : <PlayFilled />}
          </button>
          <button type="button" className="wmp-btn" title="Next" disabled={!p.hasMedia} onClick={p.onNext}>
            <NextFilled />
          </button>
          <span className="wmp-divider" />
          <button type="button" className="wmp-btn" title={p.isMuted ? "Sound" : "Mute"} onClick={p.onMuteToggle}>
            {vol === 0 ? <SpeakerMuteFilled /> : <Speaker2Filled />}
          </button>
          <input
            className="wmp-volume"
            type="range"
            min={0}
            max={1}
            step={0.05}
            value={vol}
            style={{ "--pct": `${vol * 100}%` } as React.CSSProperties}
            onChange={(e) => p.onVolumeChange(Number(e.target.value))}
          />
        </div>

        <div className="wmp-bar-right">
          {p.nowPlaying && (
            <button type="button" className="wmp-btn" title="View full screen" disabled={!p.hasMedia} onClick={p.onFullscreenToggle}>
              <FullScreenMaximizeRegular />
            </button>
          )}
          {p.showViewToggle && (
            <button
              type="button"
              className="wmp-btn"
              title={p.nowPlaying ? "Switch to Library" : "Switch to Now Playing"}
              onClick={p.onToggleView}
            >
              {p.nowPlaying ? <GridRegular /> : <MoviesAndTvRegular />}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
