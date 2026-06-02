let nextZIndex = 100;
let nextWindowId = 1;

interface WindowOptions {
  title: string;
  width?: number;
  height?: number;
  x?: number;
  y?: number;
}

export function createWindow(opts: WindowOptions): {
  el: HTMLElement;
  body: HTMLElement;
  setTitle: (t: string) => void;
  close: () => void;
} {
  const layer = document.getElementById("window-layer")!;
  const id = `win-${nextWindowId++}`;
  const w = opts.width ?? 600;
  const h = opts.height ?? 420;
  const x = opts.x ?? 80 + (nextWindowId % 6) * 30;
  const y = opts.y ?? 60 + (nextWindowId % 6) * 20;

  const el = document.createElement("div");
  el.className = "float-window";
  el.id = id;
  el.style.cssText = `width:${w}px;height:${h}px;left:${x}px;top:${y}px;z-index:${nextZIndex++}`;

  const titleBar = document.createElement("div");
  titleBar.className = "float-window-titlebar";

  const titleSpan = document.createElement("span");
  titleSpan.className = "float-window-title";
  titleSpan.textContent = opts.title;

  const closeBtn = document.createElement("button");
  closeBtn.className = "float-window-close";
  closeBtn.textContent = "✕";
  closeBtn.onclick = () => el.remove();

  titleBar.append(titleSpan, closeBtn);

  const body = document.createElement("div");
  body.className = "float-window-body";

  el.append(titleBar, body);
  layer.appendChild(el);

  makeDraggable(el, titleBar);
  el.addEventListener("mousedown", () => {
    el.style.zIndex = String(nextZIndex++);
  });

  return {
    el,
    body,
    setTitle: (t) => { titleSpan.textContent = t; },
    close: () => el.remove(),
  };
}

function makeDraggable(el: HTMLElement, handle: HTMLElement) {
  let ox = 0, oy = 0;

  handle.addEventListener("mousedown", (e) => {
    e.preventDefault();
    ox = e.clientX - el.offsetLeft;
    oy = e.clientY - el.offsetTop;

    function onMove(e: MouseEvent) {
      el.style.left = `${e.clientX - ox}px`;
      el.style.top = `${e.clientY - oy}px`;
    }

    function onUp() {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    }

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  });
}
