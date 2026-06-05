/**
 * VideoDriver — host-side backend for the guest's Direct3D8 command stream.
 *
 * The Rust `webwine-api-directx` crate is a D3D8 *state tracker*: it translates
 * the guest's COM calls (Clear/SetTexture/DrawPrimitive/Present...) into the
 * backend-agnostic `GpuCommand`s below, which flow out as `UiEvent`s. A
 * `VideoDriver` consumes them and draws with a real GPU API. This is the
 * DXVK/wined3d split: D3D front-end (Rust) → command stream → GPU back-end (here).
 *
 * `WebGLVideoDriver` (WebGL2) is the cross-browser default; a WebGPU backend can
 * implement the same interface later. `NullVideoDriver` is the headless no-op.
 */

export type GpuCommand =
  | { kind: "gpu_clear"; hwnd: number; color: number }
  | { kind: "gpu_texture"; hwnd: number; id: number; w: number; h: number; pixels: number[] | Uint8Array }
  | { kind: "gpu_draw_tris"; hwnd: number; texture: number; blend: number; verts: number[] | Float32Array }
  | { kind: "gpu_present"; hwnd: number };

export interface VideoDriver {
  /** Apply one GPU command. */
  submit(cmd: GpuCommand): void;
  /** Release GPU resources. */
  dispose(): void;
}

/** Headless no-op driver (tests / when no canvas is available). */
export class NullVideoDriver implements VideoDriver {
  submit(): void {}
  dispose(): void {}
}

// ── WebGL2 backend ───────────────────────────────────────────────────────────

const VERT_SRC = `#version 300 es
precision highp float;
// Screen-space position (pixels), uv, and rgba (all 0..1 for color).
layout(location=0) in vec2 a_pos;
layout(location=1) in vec2 a_uv;
layout(location=2) in vec4 a_col;
uniform vec2 u_viewport; // width,height in px
out vec2 v_uv;
out vec4 v_col;
void main() {
  // Pixel space -> NDC, with Y flipped (D3D origin is top-left).
  vec2 ndc = vec2(a_pos.x / u_viewport.x * 2.0 - 1.0,
                  1.0 - a_pos.y / u_viewport.y * 2.0);
  gl_Position = vec4(ndc, 0.0, 1.0);
  v_uv = a_uv;
  v_col = a_col;
}`;

const FRAG_SRC = `#version 300 es
precision highp float;
in vec2 v_uv;
in vec4 v_col;
uniform sampler2D u_tex;
uniform int u_textured;
out vec4 fragColor;
void main() {
  vec4 base = u_textured == 1 ? texture(u_tex, v_uv) : vec4(1.0);
  fragColor = base * v_col;
}`;

const FLOATS_PER_VERT = 8; // x,y,u,v,r,g,b,a

export class WebGLVideoDriver implements VideoDriver {
  private gl: WebGL2RenderingContext;
  private program: WebGLProgram;
  private vbo: WebGLBuffer;
  private vao: WebGLVertexArrayObject;
  private uViewport: WebGLUniformLocation | null;
  private uTextured: WebGLUniformLocation | null;
  private textures = new Map<number, WebGLTexture>();
  private width: number;
  private height: number;

  constructor(canvas: HTMLCanvasElement | OffscreenCanvas, width = 640, height = 480) {
    const gl = canvas.getContext("webgl2", { alpha: false, premultipliedAlpha: false }) as WebGL2RenderingContext | null;
    if (!gl) throw new Error("WebGL2 not available");
    this.gl = gl;
    this.width = width;
    this.height = height;

    this.program = this.link(VERT_SRC, FRAG_SRC);
    this.uViewport = gl.getUniformLocation(this.program, "u_viewport");
    this.uTextured = gl.getUniformLocation(this.program, "u_textured");

    this.vao = gl.createVertexArray()!;
    this.vbo = gl.createBuffer()!;
    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);
    const stride = FLOATS_PER_VERT * 4;
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, stride, 0);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 2, gl.FLOAT, false, stride, 8);
    gl.enableVertexAttribArray(2);
    gl.vertexAttribPointer(2, 4, gl.FLOAT, false, stride, 16);
    gl.bindVertexArray(null);

    gl.viewport(0, 0, width, height);
    gl.disable(gl.DEPTH_TEST);
    gl.clearColor(0, 0, 0, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
  }

  submit(cmd: GpuCommand): void {
    switch (cmd.kind) {
      case "gpu_clear": {
        const { r, g, b } = argb(cmd.color);
        this.gl.clearColor(r, g, b, 1);
        this.gl.clear(this.gl.COLOR_BUFFER_BIT);
        break;
      }
      case "gpu_texture":
        this.uploadTexture(cmd.id, cmd.w, cmd.h, cmd.pixels);
        break;
      case "gpu_draw_tris":
        this.drawTris(cmd.texture, cmd.blend, cmd.verts);
        break;
      case "gpu_present":
        this.gl.flush();
        break;
    }
  }

  private uploadTexture(id: number, w: number, h: number, pixels: number[] | Uint8Array): void {
    const gl = this.gl;
    let tex = this.textures.get(id);
    if (!tex) {
      tex = gl.createTexture()!;
      this.textures.set(id, tex);
    }
    gl.bindTexture(gl.TEXTURE_2D, tex);
    const data = pixels instanceof Uint8Array ? pixels : new Uint8Array(pixels);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, data);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  }

  private drawTris(textureId: number, blend: number, verts: number[] | Float32Array): void {
    const gl = this.gl;
    if (verts.length < FLOATS_PER_VERT * 3) return;
    const data = verts instanceof Float32Array ? verts : new Float32Array(verts);

    gl.useProgram(this.program);
    gl.uniform2f(this.uViewport, this.width, this.height);

    // Blend mode: 0 none, 1 alpha, 2 additive.
    if (blend === 0) {
      gl.disable(gl.BLEND);
    } else {
      gl.enable(gl.BLEND);
      if (blend === 2) gl.blendFunc(gl.SRC_ALPHA, gl.ONE);
      else gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    }

    const tex = this.textures.get(textureId);
    gl.uniform1i(this.uTextured, tex ? 1 : 0);
    if (tex) {
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, tex);
    }

    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);
    gl.bufferData(gl.ARRAY_BUFFER, data, gl.STREAM_DRAW);
    gl.drawArrays(gl.TRIANGLES, 0, data.length / FLOATS_PER_VERT);
    gl.bindVertexArray(null);
  }

  dispose(): void {
    const gl = this.gl;
    for (const t of this.textures.values()) gl.deleteTexture(t);
    this.textures.clear();
    gl.deleteBuffer(this.vbo);
    gl.deleteVertexArray(this.vao);
    gl.deleteProgram(this.program);
  }

  private link(vs: string, fs: string): WebGLProgram {
    const gl = this.gl;
    const compile = (type: number, src: string) => {
      const sh = gl.createShader(type)!;
      gl.shaderSource(sh, src);
      gl.compileShader(sh);
      if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
        throw new Error("shader compile: " + gl.getShaderInfoLog(sh));
      }
      return sh;
    };
    const prog = gl.createProgram()!;
    gl.attachShader(prog, compile(gl.VERTEX_SHADER, vs));
    gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, fs));
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      throw new Error("program link: " + gl.getProgramInfoLog(prog));
    }
    return prog;
  }
}

/** Decode a D3DCOLOR (0xAARRGGBB) into 0..1 floats. */
function argb(c: number): { r: number; g: number; b: number; a: number } {
  return {
    a: ((c >>> 24) & 0xff) / 255,
    r: ((c >>> 16) & 0xff) / 255,
    g: ((c >>> 8) & 0xff) / 255,
    b: (c & 0xff) / 255,
  };
}
