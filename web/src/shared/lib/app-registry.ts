import { RuntimeBridge } from "@/core/bridge/runtime-bridge";

export interface AppRegistration {
  name: string;
  exePath: string;
  icon: string;
  action: string;
  extensions?: string[];
}

class AppRegistryImpl {
  private registry: AppRegistration[] = [];

  /**
   * Registers an application in the system.
   * If it's a virtual application, it creates a placeholder .exe file with a "special:" marker.
   * It also creates a Start Menu shortcut for the application.
   */
  async registerApp(app: AppRegistration, runtime?: RuntimeBridge) {
    this.registry.push(app);

    if (runtime) {
      await runtime.registerApp(app);
    }
  }

  getAppByExe(exePath: string): AppRegistration | undefined {
    const lower = exePath.toLowerCase();
    return this.registry.find((a) => a.exePath.toLowerCase() === lower);
  }

  getAppByName(name: string): AppRegistration | undefined {
    const lower = name.toLowerCase();
    return this.registry.find((a) => a.name.toLowerCase() === lower);
  }

  getAppByAction(action: string): AppRegistration | undefined {
    return this.registry.find((a) => a.action === action);
  }

  getAppsForExtension(ext: string): AppRegistration[] {
    const targetExt = ext.toLowerCase();
    return this.registry.filter((a) => {
      if (a.extensions && a.extensions.map(e => e.toLowerCase()).includes(targetExt)) {
        return true;
      }
      if (!a.action.startsWith("ext:")) return false;
      const exts = a.action.substring(4).split(",").map((e) => e.trim().toLowerCase());
      return exts.includes(targetExt);
    });
  }

  getAll(): AppRegistration[] {
    return this.registry;
  }
}

export const AppRegistry = new AppRegistryImpl();

// Register built-in applications
AppRegistry.registerApp(
  {
    name: "File Explorer",
    exePath: "C:\\Windows\\System32\\WWExplorer.exe",
    icon: "apps/explorer.webp",
    action: "explorer",
  },
  undefined,
);

AppRegistry.registerApp(
  {
    name: "Text Editor",
    exePath: "C:\\Windows\\System32\\WWEditor.exe",
    icon: "apps/notepad.webp",
    action: "editor",
  },
  undefined,
);

AppRegistry.registerApp(
  {
    name: "Registry Editor",
    exePath: "C:\\Windows\\System32\\regedit.exe",
    icon: "apps/regedit.webp",
    action: "regedit",
  },
  undefined,
);

AppRegistry.registerApp(
  {
    name: "Upload File",
    exePath: "C:\\Windows\\System32\\uploadfile.exe",
    icon: "apps/upload-file.webp",
    action: "upload-file",
  },
  undefined,
);

AppRegistry.registerApp(
  {
    name: "Upload Folder",
    exePath: "C:\\Windows\\System32\\uploadfolder.exe",
    icon: "apps/upload-folder.webp",
    action: "upload-folder",
  },
  undefined,
);

AppRegistry.registerApp(
  {
    name: "Photo Viewer",
    exePath: "C:\\Windows\\System32\\WWPhotoViewer.exe",
    icon: "places/pictures.webp",
    action: "photo-viewer",
    extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "svg"],
  },
  undefined,
);

AppRegistry.registerApp(
  {
    name: "Media Player",
    exePath: "C:\\Windows\\System32\\WWMediaPlayer.exe",
    icon: "places/video.webp",
    action: "media-player",
    extensions: ["mp3", "mp4", "webm", "ogg", "wav"],
  },
  undefined,
);
