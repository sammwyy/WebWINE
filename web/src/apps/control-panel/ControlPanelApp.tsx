/**
 * Control Panel — WebWINE settings: themes (personalization), desktop
 * background, sounds, icon packs, desktop icons, taskbar, default programs
 * and system info. Wallpapers, sound packs and icon packs come from the
 * registries in src/registry.
 *
 * Drawn on the extended frame (`extendGlass`): navigation buttons, address
 * bar and search box sit on the glass, the pages on an opaque body.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { useWindowStore } from "@/state/windowStore";
import { useSettingsStore, type WallpaperFit } from "@/state/settingsStore";
import { WALLPAPERS } from "@/registry/wallpapers";
import { NO_SOUNDS, SOUND_EVENTS, SOUND_PACKS, type SoundId } from "@/registry/sound-packs";
import { DEFAULT_ICON_PACK, ICON_PACKS } from "@/registry/icon-packs";
import {
  effectiveWallpaperId,
  iconUrl,
  playSound,
  useIconPack,
  wallpaperStyle,
  wallpaperThumbUrl,
  wallpaperUrl,
} from "@/shared/lib/personalization";
import { useDesktopStore, type DesktopIconSize } from "@/state/desktopStore";
import { setTheme, useCurrentTheme, useThemes } from "@/shared/lib/theme-registry";
import { AppRegistry } from "@/shared/lib/app-registry";
import { launchGuestPath } from "@/shared/lib/guest-launch";
import { formatSize } from "@/shared/lib/utils";
import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";
import type { SystemMemoryInfo } from "@/core/wasm/worker";
import { GlassPanel, WindowBody, useWindow } from "@/modules/windows";
import {
  AddressBar,
  Button,
  Checkbox,
  DialogFooter,
  GroupBox,
  Icon,
  Input,
  ListView,
  NavButtons,
  NavLink,
  NavPane,
  NavSection,
  Radio,
  SearchBox,
  Select,
  Slider,
  ThemeSwitcher,
} from "@/shared/ui";
import { SystemIcon } from "@/shared/icons";
import type { SystemIconName } from "@/shared/icons";

export type ControlPanelPage =
  | "home"
  | "personalization"
  | "background"
  | "sounds"
  | "iconpack"
  | "desktop"
  | "taskbar"
  | "programs"
  | "system";

const PAGES: Record<ControlPanelPage, { title: string; icon: SystemIconName }> = {
  home: { title: "Control Panel", icon: "preferences-system" },
  personalization: { title: "Personalization", icon: "preferences-desktop-theme" },
  background: { title: "Desktop Background", icon: "preferences-desktop-wallpaper" },
  sounds: { title: "Sounds", icon: "tools-media-optical-burn" },
  iconpack: { title: "Icon Pack", icon: "folder-image" },
  desktop: { title: "Desktop Icons", icon: "preferences-desktop-display" },
  taskbar: { title: "Taskbar and Start Menu", icon: "preferences-system-windows" },
  programs: { title: "Default Programs", icon: "applications-system" },
  system: { title: "System", icon: "computer" },
};

/** Open a Control Panel window on `page`. */
export function openControlPanel(runtime: RuntimeBridge, page: ControlPanelPage = "home") {
  useWindowStore.getState().openWindow({
    title: PAGES[page].title,
    icon: iconUrl("apps/controlpanel"),
    width: 780,
    height: 520,
    minWidth: 520,
    minHeight: 340,
    extendGlass: true,
    content: <ControlPanelApp page={page} runtime={runtime} />,
  });
}


interface Category {
  page: ControlPanelPage;
  title: string;
  links: { label: string; page?: ControlPanelPage; run?: (runtime: RuntimeBridge) => void }[];
}

const CATEGORIES: Category[] = [
  {
    page: "personalization",
    title: "Appearance and Personalization",
    links: [
      { label: "Change the theme", page: "personalization" },
      { label: "Change desktop background", page: "background" },
      { label: "Change system sounds", page: "sounds" },
      { label: "Change the icon pack", page: "iconpack" },
      { label: "Adjust desktop icons", page: "desktop" },
    ],
  },
  {
    page: "taskbar",
    title: "Taskbar and Start Menu",
    links: [
      { label: "Customize the taskbar", page: "taskbar" },
      { label: "Show or hide the system log", page: "taskbar" },
    ],
  },
  {
    page: "programs",
    title: "Programs",
    links: [
      { label: "Default programs and file types", page: "programs" },
      { label: "Open Registry Editor", run: (rt) => void launchGuestPath("C:\\Windows\\System32\\regedit.exe", rt) },
    ],
  },
  {
    page: "system",
    title: "System and Maintenance",
    links: [
      { label: "View basic information about your computer", page: "system" },
      { label: "Open Task Manager", run: (rt) => void launchGuestPath("C:\\Windows\\System32\\Taskmgr.exe", rt) },
    ],
  },
];

function Home({
  go,
  runtime,
  query,
}: {
  go: (page: ControlPanelPage) => void;
  runtime: RuntimeBridge;
  query: string;
}) {
  const q = query.trim().toLowerCase();
  const categories = q
    ? CATEGORIES.map((cat) => ({
        ...cat,
        links: cat.links.filter((l) => `${cat.title} ${l.label}`.toLowerCase().includes(q)),
      })).filter((cat) => cat.links.length > 0)
    : CATEGORIES;

  return (
    <div className="p-5">
      <h1 className="ww-page-title mb-4">Adjust your computer's settings</h1>
      {categories.length === 0 && <p className="ww-muted">No settings match "{query}".</p>}
      <div className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-x-6 gap-y-5">
        {categories.map((cat) => (
          <div key={cat.title} className="flex gap-3">
            <Icon icon={<SystemIcon name={PAGES[cat.page].icon} />} size={48} />
            <div className="flex flex-col gap-0.5">
              <span className="ww-link text-[14px]" onClick={() => go(cat.page)}>
                {cat.title}
              </span>
              {cat.links.map((link) => (
                <span
                  key={link.label}
                  className="ww-link"
                  onClick={() => (link.page ? go(link.page) : link.run?.(runtime))}
                >
                  {link.label}
                </span>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

/** Page scaffold: scrollable content + optional dialog footer. */
function Page({ title, intro, children, footer }: { title: string; intro?: ReactNode; children: ReactNode; footer?: ReactNode }) {
  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 min-h-0 overflow-auto p-5">
        <h1 className="ww-page-title">{title}</h1>
        {intro && <p className="ww-muted mb-4">{intro}</p>}
        {children}
      </div>
      {footer && <DialogFooter>{footer}</DialogFooter>}
    </div>
  );
}

function Personalization({ onDone }: { onDone: () => void }) {
  const themes = useThemes();
  const current = useCurrentTheme();
  const [selected, setSelected] = useState(current?.id ?? themes[0]?.id);
  const selectedTheme = themes.find((t) => t.id === selected);

  return (
    <Page
      title="Choose a theme"
      intro="A theme changes the look of windows, controls, the taskbar, the Start menu and the desktop."
      footer={
        <>
          <Button
            isDefault
            onClick={() => {
              if (selected) setTheme(selected);
              onDone();
            }}
          >
            OK
          </Button>
          <Button onClick={onDone}>Cancel</Button>
          <Button disabled={selected === current?.id} onClick={() => selected && setTheme(selected)}>
            Apply
          </Button>
        </>
      }
    >
      <ThemeSwitcher
        className="cp-theme-grid"
        value={selected}
        onChange={setSelected}
        onActivate={(id) => {
          setTheme(id);
          onDone();
        }}
      />
      {selectedTheme && (
        <p className="mt-4 max-w-[520px]">
          <b>{selectedTheme.name}</b>
          {selectedTheme.id === current?.id && <span className="ww-muted"> (current)</span>}
          {selectedTheme.description && <> — {selectedTheme.description}</>}
        </p>
      )}
    </Page>
  );
}

const COLOR_PRESETS = ["#3a6ea5", "#008080", "#004e8c", "#1b1b1b", "#5c2d91", "#107c10", "#a4262c", "#ca5010"];

const FITS: { id: WallpaperFit; label: string }[] = [
  { id: "fill", label: "Fill" },
  { id: "fit", label: "Fit" },
  { id: "stretch", label: "Stretch" },
  { id: "center", label: "Center" },
  { id: "tile", label: "Tile" },
];

/** Thumbnail with fallbacks: `_thumb` → full image → plain tile. */
function WallpaperThumb({ src, full }: { src: string; full?: string }) {
  return (
    <img
      className="cp-wallpaper-img"
      src={src}
      alt=""
      draggable={false}
      onError={(e) => {
        const img = e.currentTarget;
        if (full && img.src !== new URL(full, location.href).href) img.src = full;
        else img.style.visibility = "hidden";
      }}
    />
  );
}

function Background() {
  const settings = useSettingsStore();
  const theme = useCurrentTheme();
  const fileRef = useRef<HTMLInputElement>(null);
  const [error, setError] = useState<string | null>(null);
  const activeId = settings.wallpaperMode === "image" ? effectiveWallpaperId(settings, theme) : null;

  const pickFile = (file: File) => {
    if (file.size > 3 * 1024 * 1024) {
      setError("That picture is too large to remember (max. 3 MB).");
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      setError(null);
      settings.set({ wallpaperMode: "image", wallpaperId: "custom", wallpaperCustom: String(reader.result) });
    };
    reader.readAsDataURL(file);
  };

  return (
    <Page title="Choose your desktop background" intro="Click a picture to make it your desktop background, or pick a solid colour.">
      <div className="flex gap-6 flex-wrap items-start">
        <div className="cp-monitor">
          <div className="cp-monitor-screen" style={{ background: theme?.preview.desktop }}>
            <div className="absolute inset-0" style={wallpaperStyle(settings, theme) ?? undefined} />
          </div>
        </div>
        <div className="flex flex-col gap-2 min-w-[200px]">
          <label className="flex items-center gap-2">
            Picture position:
            <Select value={settings.wallpaperFit} onChange={(e) => settings.set({ wallpaperFit: e.target.value as WallpaperFit })}>
              {FITS.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.label}
                </option>
              ))}
            </Select>
          </label>
          <div className="flex items-center gap-2">
            <Button onClick={() => fileRef.current?.click()}>Browse…</Button>
            <Button
              disabled={settings.wallpaperId === null && settings.wallpaperMode === "image"}
              onClick={() => settings.set({ wallpaperMode: "image", wallpaperId: null })}
              title={`Use the wallpaper of ${theme?.name ?? "the current theme"}`}
            >
              Theme default
            </Button>
          </div>
          <input
            ref={fileRef}
            type="file"
            accept="image/*"
            hidden
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) pickFile(file);
              e.target.value = "";
            }}
          />
          {error && <span className="text-[#c42b1c]">{error}</span>}
        </div>
      </div>

      <h2 className="cp-section">Pictures</h2>
      <div className="cp-wallpaper-grid">
        {WALLPAPERS.map((w) => (
          <button
            key={w.id}
            type="button"
            className={`cp-wallpaper ${activeId === w.id ? "selected" : ""}`}
            title={w.name}
            onClick={() => settings.set({ wallpaperMode: "image", wallpaperId: w.id })}
          >
            <span className="cp-wallpaper-frame">
              <WallpaperThumb src={wallpaperThumbUrl(w)} full={wallpaperUrl(w)} />
            </span>
            <span>
              {w.name}
              {w.id === theme?.preferredWallpaper && <span className="ww-muted"> (theme)</span>}
            </span>
          </button>
        ))}
        {settings.wallpaperCustom && (
          <button
            type="button"
            className={`cp-wallpaper ${activeId === "custom" ? "selected" : ""}`}
            onClick={() => settings.set({ wallpaperMode: "image", wallpaperId: "custom" })}
          >
            <span className="cp-wallpaper-frame">
              <WallpaperThumb src={settings.wallpaperCustom} />
            </span>
            <span>Your picture</span>
          </button>
        )}
      </div>

      <h2 className="cp-section">Solid colours</h2>
      <div className="flex gap-1.5 items-center flex-wrap">
        {COLOR_PRESETS.map((c) => (
          <button
            key={c}
            type="button"
            className={`cp-swatch ${settings.wallpaperMode === "color" && settings.wallpaperColor === c ? "selected" : ""}`}
            style={{ background: c }}
            title={c}
            onClick={() => settings.set({ wallpaperMode: "color", wallpaperColor: c })}
          />
        ))}
        <Input
          type="color"
          className="w-[42px] !p-0"
          value={settings.wallpaperColor}
          title="More colours…"
          onChange={(e) => settings.set({ wallpaperMode: "color", wallpaperColor: e.target.value })}
        />
      </div>
    </Page>
  );
}

function Sounds() {
  const settings = useSettingsStore();
  const theme = useCurrentTheme();
  const activePack = settings.soundPack ?? theme?.preferredSoundPack ?? NO_SOUNDS;
  const [selectedSound, setSelectedSound] = useState<SoundId>("default");
  const sounds = Object.entries(SOUND_EVENTS) as [SoundId, string][];

  return (
    <Page
      title="Sounds"
      intro="A sound scheme is a set of sounds applied to events in WebWINE. Themes pick their own scheme when applied."
    >
      <div className="flex flex-col gap-3 max-w-[560px]">
        <label className="flex items-center gap-2 flex-wrap">
          Sound scheme:
          <Select
            className="min-w-[200px]"
            value={activePack}
            onChange={(e) => settings.set({ soundPack: e.target.value })}
          >
            {SOUND_PACKS.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
                {p.id === theme?.preferredSoundPack ? " (theme)" : ""}
              </option>
            ))}
          </Select>
          <Button disabled={settings.soundPack === null} onClick={() => settings.set({ soundPack: null })}>
            Theme default
          </Button>
        </label>

        <label className="flex items-center gap-2">
          Volume:
          <Slider
            className="flex-1"
            min={0}
            max={100}
            value={Math.round(settings.soundVolume * 100)}
            onChange={(e) => settings.set({ soundVolume: Number(e.target.value) / 100 })}
          />
          <span className="w-9 text-right">{Math.round(settings.soundVolume * 100)}%</span>
        </label>

        <div className="ww-body h-[230px]">
          <ListView
            items={sounds}
            getKey={([id]) => id}
            selectedKey={selectedSound}
            onSelect={(item) => item && setSelectedSound(item[0])}
            onActivate={([id]) => playSound(id, activePack)}
            columns={[
              { key: "event", label: "Program events", width: 260, render: ([, label]) => label },
              { key: "file", label: "File", width: 160, muted: true, render: ([id]) => `${id}` },
            ]}
          />
        </div>

        <div className="flex gap-2">
          <Button disabled={activePack === NO_SOUNDS} onClick={() => playSound(selectedSound, activePack)}>
            ▶ Test
          </Button>
          <span className="ww-muted self-center">Double-click an event to hear it.</span>
        </div>
      </div>
    </Page>
  );
}

const PREVIEW_ICONS = [
  "places/thispc",
  "shell/folder",
  "places/documents",
  "places/pictures",
  "places/music",
  "apps/explorer",
  "apps/notepad",
  "apps/taskmgr",
  "exts/txt",
  "shell/drive_main",
];

function IconPacks() {
  const settings = useSettingsStore();
  const theme = useCurrentTheme();
  const activePack = useIconPack();

  return (
    <Page title="Icon pack" intro="Choose the icons used for files, folders and programs. Themes pick their own pack when applied.">
      <div className="flex items-center gap-2 mb-4">
        <Button disabled={settings.iconPack === null} onClick={() => settings.set({ iconPack: null })}>
          Theme default
        </Button>
      </div>
      <div className="flex flex-col gap-3">
        {ICON_PACKS.map((pack) => (
          <button
            key={pack.id}
            type="button"
            className={`cp-iconpack ${activePack === pack.id ? "selected" : ""}`}
            onClick={() => settings.set({ iconPack: pack.id })}
          >
            <span className="font-semibold">
              {pack.name}
              {pack.id === (theme?.preferredIconPack ?? DEFAULT_ICON_PACK) && <span className="ww-muted"> (theme)</span>}
            </span>
            <span className="flex gap-2 flex-wrap">
              {PREVIEW_ICONS.map((id) => (
                <Icon key={id} icon={iconUrl(id, pack.id)} size={32} />
              ))}
            </span>
          </button>
        ))}
      </div>
    </Page>
  );
}

const ICON_SIZES: { id: DesktopIconSize; label: string }[] = [
  { id: "large", label: "Large icons" },
  { id: "medium", label: "Medium icons" },
  { id: "small", label: "Small icons" },
];

function DesktopIcons() {
  const { iconSize, setIconSize } = useDesktopStore();
  const settings = useSettingsStore();

  return (
    <Page title="Desktop icons" intro="Choose how icons on the desktop are shown.">
      <div className="flex flex-col gap-3 max-w-[420px]">
        <GroupBox title="Icon size">
          <div className="flex flex-col gap-2 pt-1">
            {ICON_SIZES.map((s) => (
              <Radio
                key={s.id}
                name="icon-size"
                label={s.label}
                checked={iconSize === s.id}
                onChange={() => setIconSize(s.id)}
              />
            ))}
          </div>
        </GroupBox>
        <GroupBox title="Visibility">
          <div className="pt-1">
            <Checkbox
              label="Show desktop icons"
              checked={settings.showDesktopIcons}
              onChange={(e) => settings.set({ showDesktopIcons: e.target.checked })}
            />
          </div>
        </GroupBox>
      </div>
    </Page>
  );
}

function Taskbar() {
  const settings = useSettingsStore();
  const theme = useCurrentTheme();

  return (
    <Page
      title="Taskbar and Start menu"
      intro="Changes are applied immediately."
      footer={
        <Button onClick={() => settings.reset(["taskbarLabels", "clockSeconds", "showLogPanel"])}>
          Restore defaults
        </Button>
      }
    >
      <div className="flex flex-col gap-3 max-w-[460px]">
        <GroupBox title="Taskbar appearance">
          <div className="flex flex-col gap-2 pt-1">
            <Checkbox
              label="Show window titles on taskbar buttons"
              checked={settings.taskbarLabels}
              onChange={(e) => settings.set({ taskbarLabels: e.target.checked })}
            />
            <Checkbox
              label="Show seconds on the clock"
              checked={settings.clockSeconds}
              onChange={(e) => settings.set({ clockSeconds: e.target.checked })}
            />
          </div>
        </GroupBox>
        <GroupBox title="Start menu">
          <p className="pt-1">
            Style: <b>{theme?.startMenu === "classic" ? "Classic (two columns)" : "Modern (app list and tiles)"}</b>
            <br />
            <span className="ww-muted">The Start menu style follows the current theme.</span>
          </p>
        </GroupBox>
        <GroupBox title="Developer">
          <div className="pt-1">
            <Checkbox
              label="Show the system log panel"
              checked={settings.showLogPanel}
              onChange={(e) => settings.set({ showLogPanel: e.target.checked })}
            />
          </div>
        </GroupBox>
      </div>
    </Page>
  );
}

function Programs() {
  const apps = AppRegistry.getAll();
  const [selected, setSelected] = useState<string | null>(null);

  return (
    <Page title="Default programs" intro="Built-in programs and the file types they open.">
      <div className="ww-body h-[300px]">
        <ListView
          items={apps}
          getKey={(a) => a.exePath}
          selectedKey={selected}
          onSelect={(a) => setSelected(a?.exePath ?? null)}
          sortedBy="name"
          columns={[
            {
              key: "name",
              label: "Name",
              width: 200,
              render: (a) => (
                <span className="flex items-center gap-1.5">
                  <Icon icon={iconUrl(a.icon)} size={16} />
                  {a.name}
                </span>
              ),
            },
            {
              key: "ext",
              label: "File types",
              width: 220,
              muted: true,
              render: (a) => (a.extensions?.length ? a.extensions.map((e) => `.${e}`).join(", ") : "—"),
            },
            { key: "path", label: "Location", width: 280, muted: true, render: (a) => a.exePath },
          ]}
        />
      </div>
    </Page>
  );
}

function System({ runtime }: { runtime: RuntimeBridge }) {
  const [mem, setMem] = useState<SystemMemoryInfo | null>(null);
  const theme = useCurrentTheme();

  useEffect(() => {
    let alive = true;
    runtime
      .getSystemMemory()
      .then((m) => alive && setMem(m))
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [runtime]);

  const webgl = useMemo(() => {
    try {
      return Boolean(document.createElement("canvas").getContext("webgl"));
    } catch {
      return false;
    }
  }, []);

  const rows: [string, ReactNode][] = [
    ["Edition", "WebWINE"],
    ["Theme", theme?.name ?? "—"],
    ["Browser", navigator.userAgent],
    ["Logical processors", navigator.hardwareConcurrency || "Unknown"],
    ["Cross-origin isolated", window.crossOriginIsolated ? "Yes (SharedArrayBuffer available)" : "No"],
    ["WebGL", webgl ? "Available" : "Not available"],
    ["Guest processes", mem ? mem.processes.length : "…"],
    ["Guest heap in use", mem ? formatSize(mem.total_heap_used) : "…"],
    ["Guest memory mapped", mem ? formatSize(mem.total_mapped) : "…"],
  ];

  return (
    <Page title="View basic information about your computer">
      <div className="flex gap-4 items-start">
        <Icon icon={<SystemIcon name="computer" />} size={64} />
        <div className="ww-propgrid flex-1 !grid-cols-[170px_1fr]">
          {rows.map(([k, v]) => (
            <div key={k} className="contents">
              <span className="ww-muted">{k}:</span>
              <span>{v}</span>
            </div>
          ))}
        </div>
      </div>
    </Page>
  );
}


function ControlPanelApp({ page = "home", runtime }: { page?: ControlPanelPage; runtime: RuntimeBridge }) {
  const win = useWindow();
  const [history, setHistory] = useState({ stack: [page], index: 0 });
  const [query, setQuery] = useState("");
  const current = history.stack[history.index];

  const show = (index: number, stack = history.stack) => {
    setHistory({ stack, index });
    win.setTitle(PAGES[stack[index]].title);
  };

  const go = (next: ControlPanelPage) => {
    if (next !== current) show(history.index + 1, [...history.stack.slice(0, history.index + 1), next]);
  };

  const content = {
    home: <Home go={go} runtime={runtime} query={query} />,
    personalization: <Personalization onDone={() => go("home")} />,
    background: <Background />,
    sounds: <Sounds />,
    iconpack: <IconPacks />,
    desktop: <DesktopIcons />,
    taskbar: <Taskbar />,
    programs: <Programs />,
    system: <System runtime={runtime} />,
  }[current];

  const link = (p: ControlPanelPage, label = PAGES[p].title) => (
    <NavLink icon={<SystemIcon name={PAGES[p].icon} />} selected={current === p} onClick={() => go(p)}>
      {label}
    </NavLink>
  );

  return (
    <>
      <GlassPanel>
        <NavButtons
          canGoBack={history.index > 0}
          canGoForward={history.index < history.stack.length - 1}
          onBack={() => show(history.index - 1)}
          onForward={() => show(history.index + 1)}
          canGoUp={current !== "home"}
          onUp={() => go("home")}
        />
        <AddressBar
          icon={<SystemIcon name="preferences-system" />}
          path={[
            { label: "Control Panel", icon: <SystemIcon name="preferences-system" />, onClick: () => go("home") },
            ...(current === "home"
              ? []
              : [{ label: PAGES[current].title, icon: <SystemIcon name={PAGES[current].icon} /> }]),
          ]}
        />
        <SearchBox
          className="w-[190px] shrink-0"
          placeholder="Search Control Panel"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            if (current !== "home") go("home");
          }}
        />
      </GlassPanel>

      <WindowBody>
        <div className="flex flex-1 min-h-0">
          <NavPane width={200}>
            <NavSection title="Control Panel">
              <NavLink selected={current === "home"} onClick={() => go("home")}>
                Control Panel Home
              </NavLink>
            </NavSection>
            <NavSection title="Appearance">
              {link("personalization")}
              {link("background")}
              {link("sounds")}
              {link("iconpack")}
              {link("desktop")}
            </NavSection>
            <NavSection title="System">
              {link("taskbar", "Taskbar")}
              {link("programs")}
              {link("system")}
            </NavSection>
          </NavPane>
          <div className="flex-1 min-w-0">{content}</div>
        </div>
      </WindowBody>
    </>
  );
}
