//! Small, state-safe OpenGL 1.x/2.x compatibility surface.
//!
//! WebWINE's renderer is driven by the browser, so these calls currently keep
//! an application's OpenGL control flow alive and expose the conventional
//! capability queries.  The state is per guest process, not global.
use super::{ApiContext, Handled, WinApiRegistry};

fn ret(c: &mut ApiContext, value: u32, args: u32) -> Handled {
    c.ret_stdcall(value, args);
    Handled::Ok
}

fn noop0(c: &mut ApiContext) -> Handled {
    ret(c, 0, 0)
}
fn noop1(c: &mut ApiContext) -> Handled {
    ret(c, 0, 1)
}
fn noop2(c: &mut ApiContext) -> Handled {
    ret(c, 0, 2)
}
fn noop3(c: &mut ApiContext) -> Handled {
    ret(c, 0, 3)
}
fn noop4(c: &mut ApiContext) -> Handled {
    ret(c, 0, 4)
}
fn noop6(c: &mut ApiContext) -> Handled {
    ret(c, 0, 6)
}
fn noop7(c: &mut ApiContext) -> Handled {
    ret(c, 0, 7)
}
fn noop8(c: &mut ApiContext) -> Handled {
    ret(c, 0, 8)
}
fn noop9(c: &mut ApiContext) -> Handled {
    ret(c, 0, 9)
}

fn gl_get_error(c: &mut ApiContext) -> Handled {
    ret(c, 0, 0)
}

// OpenGL immediate-mode state. The payload is guest memory so it survives API
// calls without adding host-side global state. Offsets: hwnd, begin mode,
// rgba, uv, bound texture, vertex buffer, vertex count.
const S_HWND: u32 = 0;
const S_MODE: u32 = 4;
const S_COLOR: u32 = 8;
const S_U: u32 = 12;
const S_V: u32 = 16;
const S_TEX: u32 = 20;
const S_VERTS: u32 = 24;
const S_COUNT: u32 = 28;
const S_CLEAR: u32 = 32;
const VERT_SIZE: u32 = 32;
const MAX_VERTS: u32 = 4096;

fn state(c: &mut ApiContext) -> u32 {
    if let Some(&p) = c.dll_state.get("opengl.state") {
        return p;
    }
    let p = c.heap_alloc(64);
    let verts = c.heap_alloc(VERT_SIZE * MAX_VERTS);
    if p != 0 {
        let _ = c.memory.write_u32(p + S_VERTS, verts);
        let _ = c.memory.write_u32(p + S_COLOR, 0x3F80_0000); // r=1; others set below
        let _ = c.memory.write_u32(p + S_COLOR + 4, 0x3F80_0000);
        let _ = c.memory.write_u32(p + S_COLOR + 8, 0x3F80_0000);
        let _ = c.memory.write_u32(p + S_COLOR + 12, 0x3F80_0000);
        let _ = c.memory.write_u32(p + S_CLEAR, 0);
        c.dll_state.insert("opengl.state".into(), p);
    }
    p
}

fn gl_begin(c: &mut ApiContext) -> Handled {
    let s = state(c);
    if s != 0 {
        let _ = c.memory.write_u32(s + S_MODE, c.arg(0));
        let _ = c.memory.write_u32(s + S_COUNT, 0);
    }
    ret(c, 0, 1)
}

fn gl_clear_color(c: &mut ApiContext) -> Handled {
    let s = state(c);
    if s != 0 {
        for i in 0..4 {
            let _ = c.memory.write_u32(s + S_CLEAR + i * 4, c.arg(i));
        }
    }
    ret(c, 0, 4)
}

fn gl_clear(c: &mut ApiContext) -> Handled {
    let s = state(c);
    if s != 0 {
        let r = f32::from_bits(c.memory.read_u32(s + S_CLEAR).unwrap_or(0)).clamp(0.0, 1.0);
        let g = f32::from_bits(c.memory.read_u32(s + S_CLEAR + 4).unwrap_or(0)).clamp(0.0, 1.0);
        let b = f32::from_bits(c.memory.read_u32(s + S_CLEAR + 8).unwrap_or(0)).clamp(0.0, 1.0);
        let a = f32::from_bits(c.memory.read_u32(s + S_CLEAR + 12).unwrap_or(0)).clamp(0.0, 1.0);
        let color = ((a * 255.0) as u32) << 24
            | ((r * 255.0) as u32) << 16
            | ((g * 255.0) as u32) << 8
            | (b * 255.0) as u32;
        c.ui_events
            .push(webwine_api::vm::process::UiEvent::GpuClear {
                hwnd: c.memory.read_u32(s + S_HWND).unwrap_or(0),
                color,
            });
    }
    ret(c, 0, 1)
}

fn gl_color4f(c: &mut ApiContext) -> Handled {
    let s = state(c);
    if s != 0 {
        for i in 0..4 {
            let _ = c.memory.write_u32(s + S_COLOR + i * 4, c.arg(i));
        }
    }
    ret(c, 0, 4)
}

fn gl_texcoord2f(c: &mut ApiContext) -> Handled {
    let s = state(c);
    if s != 0 {
        let _ = c.memory.write_u32(s + S_U, c.arg(0));
        let _ = c.memory.write_u32(s + S_V, c.arg(1));
    }
    ret(c, 0, 2)
}

fn gl_bind_texture(c: &mut ApiContext) -> Handled {
    let s = state(c);
    if s != 0 {
        let _ = c.memory.write_u32(s + S_TEX, c.arg(1));
    }
    ret(c, 0, 2)
}

fn gl_tex_image_2d(c: &mut ApiContext) -> Handled {
    let s = state(c);
    let w = c.arg(3).min(4096);
    let h = c.arg(4).min(4096);
    let pixels = c.arg(8);
    let tex = if s != 0 {
        c.memory.read_u32(s + S_TEX).unwrap_or(0)
    } else {
        0
    };
    let hwnd = if s != 0 {
        c.memory.read_u32(s + S_HWND).unwrap_or(0)
    } else {
        0
    };
    if tex != 0 && hwnd != 0 && w != 0 && h != 0 && pixels != 0 {
        let size = (w as usize).saturating_mul(h as usize).saturating_mul(4);
        if let Ok(src) = c.memory.read_bytes(pixels, size) {
            let format = c.arg(6);
            let ty = c.arg(7);
            if ty == 0x1401 {
                // GL_UNSIGNED_BYTE
                let mut rgba = src;
                // OpenGL's common BGRA upload format is byte-swapped for WebGL.
                if format == 0x80E1 {
                    // GL_BGRA
                    for p in rgba.chunks_exact_mut(4) {
                        p.swap(0, 2);
                    }
                }
                c.ui_events
                    .push(webwine_api::vm::process::UiEvent::GpuTexture {
                        hwnd,
                        id: tex,
                        w,
                        h,
                        pixels: rgba,
                    });
            }
        }
    }
    ret(c, 0, 9)
}

fn gl_vertex(c: &mut ApiContext, x: f32, y: f32, z: f32) -> Handled {
    let s = state(c);
    if s != 0 {
        let n = c.memory.read_u32(s + S_COUNT).unwrap_or(0);
        let ptr = c.memory.read_u32(s + S_VERTS).unwrap_or(0);
        if n < MAX_VERTS && ptr != 0 {
            let p = ptr + n * VERT_SIZE;
            let _ = z; // depth is not part of the screen-space browser stream
            for (o, v) in [
                (0, x),
                (4, y),
                (8, f32::from_bits(c.memory.read_u32(s + S_U).unwrap_or(0))),
                (12, f32::from_bits(c.memory.read_u32(s + S_V).unwrap_or(0))),
                (
                    16,
                    f32::from_bits(c.memory.read_u32(s + S_COLOR).unwrap_or(0)),
                ),
                (
                    20,
                    f32::from_bits(c.memory.read_u32(s + S_COLOR + 4).unwrap_or(0)),
                ),
                (
                    24,
                    f32::from_bits(c.memory.read_u32(s + S_COLOR + 8).unwrap_or(0)),
                ),
                (
                    28,
                    f32::from_bits(c.memory.read_u32(s + S_COLOR + 12).unwrap_or(0)),
                ),
            ] {
                let _ = c.memory.write_u32(p + o, v.to_bits());
            }
            // The remaining color channels are read from the state when ending.
            let _ = c.memory.write_u32(s + S_COUNT, n + 1);
        }
    }
    ret(c, 0, 3)
}

fn gl_vertex2f(c: &mut ApiContext) -> Handled {
    gl_vertex(c, f32::from_bits(c.arg(0)), f32::from_bits(c.arg(1)), 0.0)
}
fn gl_vertex3f(c: &mut ApiContext) -> Handled {
    gl_vertex(
        c,
        f32::from_bits(c.arg(0)),
        f32::from_bits(c.arg(1)),
        f32::from_bits(c.arg(2)),
    )
}

fn gl_end(c: &mut ApiContext) -> Handled {
    let s = state(c);
    if s != 0 {
        let mode = c.memory.read_u32(s + S_MODE).unwrap_or(0);
        let n = c.memory.read_u32(s + S_COUNT).unwrap_or(0) as usize;
        let ptr = c.memory.read_u32(s + S_VERTS).unwrap_or(0);
        let mut verts = Vec::new();
        let tri = |i: usize, out: &mut Vec<f32>, c: &ApiContext| {
            let p = ptr + i as u32 * VERT_SIZE;
            let x = f32::from_bits(c.memory.read_u32(p).unwrap_or(0));
            let y = f32::from_bits(c.memory.read_u32(p + 4).unwrap_or(0));
            let u = f32::from_bits(c.memory.read_u32(p + 8).unwrap_or(0));
            let v = f32::from_bits(c.memory.read_u32(p + 12).unwrap_or(0));
            let r = f32::from_bits(c.memory.read_u32(p + 16).unwrap_or(0));
            let g = f32::from_bits(c.memory.read_u32(p + 20).unwrap_or(0));
            let b = f32::from_bits(c.memory.read_u32(p + 24).unwrap_or(0));
            let a = f32::from_bits(c.memory.read_u32(p + 28).unwrap_or(0));
            out.extend([x, y, u, v, r, g, b, a]);
        };
        if ptr != 0 {
            match mode {
                4 => {
                    for i in (0..n.saturating_sub(2)).step_by(3) {
                        tri(i, &mut verts, c);
                        tri(i + 1, &mut verts, c);
                        tri(i + 2, &mut verts, c);
                    }
                }
                5 => {
                    for i in 0..n.saturating_sub(2) {
                        tri(i, &mut verts, c);
                        tri(i + 1, &mut verts, c);
                        tri(i + 2, &mut verts, c);
                    }
                }
                6 => {
                    for i in 0..n.saturating_sub(2) {
                        tri(0, &mut verts, c);
                        tri(i + 1, &mut verts, c);
                        tri(i + 2, &mut verts, c);
                    }
                }
                _ => {}
            }
        }
        if !verts.is_empty() {
            c.ui_events
                .push(webwine_api::vm::process::UiEvent::GpuDrawTris {
                    hwnd: c.memory.read_u32(s + S_HWND).unwrap_or(0),
                    texture: c.memory.read_u32(s + S_TEX).unwrap_or(0),
                    blend: 1,
                    verts,
                });
        }
    }
    ret(c, 0, 0)
}

fn wgl_make_current(c: &mut ApiContext) -> Handled {
    let s = state(c);
    if s != 0 && c.arg(0) != 0 {
        let _ = c.memory.write_u32(s + S_HWND, c.arg(0));
        c.dll_state.insert("opengl.context".into(), c.arg(1));
    } else {
        c.dll_state.remove("opengl.context");
    }
    ret(c, 1, 2)
}

fn wgl_create_context(c: &mut ApiContext) -> Handled {
    let hdc = c.arg(0);
    let p = c.heap_alloc(4);
    if p != 0 {
        let _ = c.memory.write_u32(p, hdc);
    }
    ret(c, p, 1)
}

fn wgl_get_current_context(c: &mut ApiContext) -> Handled {
    ret(
        c,
        c.dll_state.get("opengl.context").copied().unwrap_or(0),
        0,
    )
}

fn wgl_get_proc_address(c: &mut ApiContext) -> Handled {
    let name = c.cstr(c.arg(0));
    let va = c.api_trampoline_va("opengl32.dll", &name);
    ret(c, va, 1)
}

fn gl_get_string(c: &mut ApiContext) -> Handled {
    let name = c.arg(0);
    let text = match name {
        0x1F00 => "WebWINE",        // GL_VENDOR
        0x1F01 => "WebWINE OpenGL", // GL_RENDERER
        0x1F02 => "1.1 WebWINE",    // GL_VERSION
        0x1F03 => "GL_EXT_texture", // GL_EXTENSIONS
        _ => "",
    };
    let key = format!("opengl.string.{name}");
    let ptr = if let Some(&p) = c.dll_state.get(&key) {
        p
    } else {
        let p = c.heap_alloc(text.len() as u32 + 1);
        if p != 0 {
            let mut bytes = text.as_bytes().to_vec();
            bytes.push(0);
            let _ = c.memory.write_bytes(p, &bytes);
            c.dll_state.insert(key, p);
        }
        p
    };
    ret(c, ptr, 1)
}

fn gl_get_integerv(c: &mut ApiContext) -> Handled {
    let pname = c.arg(0);
    let out = c.arg(1);
    let value = match pname {
        0x0D33 => 4096, // GL_MAX_TEXTURE_SIZE
        0x0D3A => 8,    // GL_SUBPIXEL_BITS
        0x0D52 => 1,    // GL_AUX_BUFFERS
        _ => 0,
    };
    if out != 0 {
        let _ = c.memory.write_u32(out, value);
    }
    ret(c, 0, 2)
}

fn gl_gen_textures(c: &mut ApiContext) -> Handled {
    let n = c.arg(0).min(1024);
    let out = c.arg(1);
    let next = c.dll_state.entry("opengl.next_texture".into()).or_insert(1);
    for i in 0..n {
        if out != 0 {
            let _ = c.memory.write_u32(out + i * 4, *next);
        }
        *next = next.wrapping_add(1);
    }
    ret(c, 0, 2)
}

pub fn register(r: &mut WinApiRegistry) {
    // name, stdcall argument count.  The fixed-function entry points are the
    // surface used by legacy SDL/Cocos/Geometry-Dash-era OpenGL programs.
    let zero: &[(&str, super::HandlerFn)] = &[
        ("glFlush", noop0),
        ("glFinish", noop0),
        ("glLoadIdentity", noop0),
        ("glPopMatrix", noop0),
        ("glPushMatrix", noop0),
        ("glEndList", noop0),
        ("glCallList", noop1),
        ("glEnable", noop1),
        ("glDisable", noop1),
        ("glTexImage2D", noop9),
        ("glTexParameteri", noop3),
        ("glTexEnvi", noop3),
        ("glBlendFunc", noop2),
        ("glAlphaFunc", noop2),
        ("glDepthFunc", noop1),
        ("glDepthMask", noop1),
        ("glCullFace", noop1),
        ("glShadeModel", noop1),
        ("glMatrixMode", noop1),
        ("glViewport", noop4),
        ("glColor3f", noop3),
        ("glColor4ub", noop4),
        ("glNormal3f", noop3),
        ("glVertex3fv", noop1),
        ("glTranslatef", noop3),
        ("glScalef", noop3),
        ("glRotatef", noop4),
        ("glOrtho", noop6),
        ("glClearDepth", noop1),
        ("glLineWidth", noop1),
        ("glPointSize", noop1),
        ("glDeleteTextures", noop2),
        ("glTexImage1D", noop8),
        ("glTexSubImage2D", noop9),
        ("glReadPixels", noop7),
        ("glPixelStorei", noop2),
        ("glHint", noop2),
        ("glListBase", noop1),
        ("glCallLists", noop3),
    ];
    for &(name, f) in zero {
        r.add("opengl32.dll", name, f);
    }
    r.add("opengl32.dll", "glBegin", gl_begin);
    r.add("opengl32.dll", "glEnd", gl_end);
    r.add("opengl32.dll", "glClear", gl_clear);
    r.add("opengl32.dll", "glClearColor", gl_clear_color);
    r.add("opengl32.dll", "glColor4f", gl_color4f);
    r.add("opengl32.dll", "glTexCoord2f", gl_texcoord2f);
    r.add("opengl32.dll", "glBindTexture", gl_bind_texture);
    r.add("opengl32.dll", "glTexImage2D", gl_tex_image_2d);
    r.add("opengl32.dll", "glVertex2f", gl_vertex2f);
    r.add("opengl32.dll", "glVertex3f", gl_vertex3f);
    r.add("opengl32.dll", "glGetError", gl_get_error);
    r.add("opengl32.dll", "glGetString", gl_get_string);
    r.add("opengl32.dll", "glGetIntegerv", gl_get_integerv);
    r.add("opengl32.dll", "glGenTextures", gl_gen_textures);
    r.add("opengl32.dll", "wglCreateContext", wgl_create_context);
    r.add("opengl32.dll", "wglDeleteContext", noop1);
    r.add("opengl32.dll", "wglMakeCurrent", wgl_make_current);
    r.add("opengl32.dll", "wglGetCurrentContext", wgl_get_current_context);
    r.add("opengl32.dll", "wglGetCurrentDC", noop0);
    r.add("opengl32.dll", "wglGetProcAddress", wgl_get_proc_address);
    r.add("opengl32.dll", "wglSwapLayerBuffers", noop2);
}
