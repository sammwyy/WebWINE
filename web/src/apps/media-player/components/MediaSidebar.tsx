import { iconUrl } from "@/shared/lib/personalization";

export type LibraryView = "playlists" | "music" | "videos";

interface MediaLibraryTreeProps {
  view: LibraryView;
  counts: Record<LibraryView, number>;
  onSelect: (view: LibraryView) => void;
}

/** Windows Media Player 12 library navigation tree. */
export function MediaLibraryTree({ view, counts, onSelect }: MediaLibraryTreeProps) {
  const item = (id: LibraryView, label: string, icon: string, depth = 1) => (
    <div
      className={`wmp-tree-item ${view === id ? "selected" : ""}`}
      style={{ paddingLeft: 8 + depth * 14 }}
      onClick={() => onSelect(id)}
    >
      <img src={iconUrl(icon)} alt="" draggable={false} />
      <span className="truncate">{label}</span>
      {counts[id] > 0 && <span className="wmp-tree-count">{counts[id]}</span>}
    </div>
  );

  return (
    <nav className="wmp-tree">
      <div className="wmp-tree-root">
        <img src={iconUrl("places/documents")} alt="" draggable={false} />
        <span>Library</span>
      </div>
      {item("playlists", "Playlists", "apps/mediaplayer")}
      {item("music", "Music", "places/music")}
      <div className="wmp-tree-sub">Artist</div>
      <div className="wmp-tree-sub">Album</div>
      <div className="wmp-tree-sub">Genre</div>
      {item("videos", "Videos", "places/video")}
      <div className="wmp-tree-item disabled" style={{ paddingLeft: 22 }}>
        <img src={iconUrl("places/pictures")} alt="" draggable={false} />
        <span>Pictures</span>
      </div>
      <div className="wmp-tree-sep" />
      <div className="wmp-tree-root muted">
        <img src={iconUrl("places/thispc")} alt="" draggable={false} />
        <span>Other Libraries</span>
      </div>
    </nav>
  );
}
