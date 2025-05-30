use std::any::Any;
use std::cell::{RefCell, UnsafeCell};
use std::ffi::{c_void, CStr};
use std::hint::unreachable_unchecked;
use std::mem;
use std::mem::MaybeUninit;
use std::ptr::null;
use std::sync::Arc;

use imgui::{DrawCmd, DrawCmdParams, DrawData, DrawIdx, DrawVert, TextureId, Ui};

use crate::common::{Color, Rect};
use crate::framework::backend::{BackendRenderer, BackendShader, BackendTexture, SpriteBatchCommand, VertexData};
use crate::framework::context::Context;
use crate::framework::error::GameError;
use crate::framework::error::GameError::RenderError;
use crate::framework::error::GameResult;
use crate::framework::gl;
use crate::framework::gl::types::*;
use crate::framework::graphics::{BlendMode, VSyncMode};
use crate::framework::util::{field_offset, return_param};
use crate::game::GAME_SUSPENDED;

pub struct GLContext {
    pub gles2_mode: bool,
    pub is_sdl: bool,
    pub get_proc_address: unsafe fn(user_data: &mut *mut c_void, name: &str) -> *const c_void,
    pub swap_buffers: unsafe fn(user_data: &mut *mut c_void),
    pub user_data: *mut c_void,
    pub ctx: *mut Context,
}

pub struct OpenGLTexture {
    width: u16,
    height: u16,
    texture_id: u32,
    framebuffer_id: u32,
    shader: RenderShader,
    vbo: GLuint,
    vertices: Vec<VertexData>,
    context_active: Arc<RefCell<bool>>,
}

impl BackendTexture for OpenGLTexture {
    fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn add(&mut self, command: SpriteBatchCommand) {
        let (tex_scale_x, tex_scale_y) = (1.0 / self.width as f32, 1.0 / self.height as f32);

        match command {
            SpriteBatchCommand::DrawRect(src, dest) => {
                let vertices = [
                    VertexData {
                        position: (dest.left, dest.bottom),
                        uv: (src.left * tex_scale_x, src.bottom * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                    VertexData {
                        position: (dest.left, dest.top),
                        uv: (src.left * tex_scale_x, src.top * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                    VertexData {
                        position: (dest.right, dest.top),
                        uv: (src.right * tex_scale_x, src.top * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                    VertexData {
                        position: (dest.left, dest.bottom),
                        uv: (src.left * tex_scale_x, src.bottom * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                    VertexData {
                        position: (dest.right, dest.top),
                        uv: (src.right * tex_scale_x, src.top * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                    VertexData {
                        position: (dest.right, dest.bottom),
                        uv: (src.right * tex_scale_x, src.bottom * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                ];
                self.vertices.extend_from_slice(&vertices);
            }
            SpriteBatchCommand::DrawRectFlip(mut src, dest, flip_x, flip_y) => {
                if flip_x {
                    std::mem::swap(&mut src.left, &mut src.right);
                }

                if flip_y {
                    std::mem::swap(&mut src.top, &mut src.bottom);
                }

                let vertices = [
                    VertexData {
                        position: (dest.left, dest.bottom),
                        uv: (src.left * tex_scale_x, src.bottom * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                    VertexData {
                        position: (dest.left, dest.top),
                        uv: (src.left * tex_scale_x, src.top * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                    VertexData {
                        position: (dest.right, dest.top),
                        uv: (src.right * tex_scale_x, src.top * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                    VertexData {
                        position: (dest.left, dest.bottom),
                        uv: (src.left * tex_scale_x, src.bottom * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                    VertexData {
                        position: (dest.right, dest.top),
                        uv: (src.right * tex_scale_x, src.top * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                    VertexData {
                        position: (dest.right, dest.bottom),
                        uv: (src.right * tex_scale_x, src.bottom * tex_scale_y),
                        color: (255, 255, 255, 255),
                    },
                ];
                self.vertices.extend_from_slice(&vertices);
            }
            SpriteBatchCommand::DrawRectTinted(src, dest, color) => {
                let color = color.to_rgba();
                let vertices = [
                    VertexData {
                        position: (dest.left, dest.bottom),
                        uv: (src.left * tex_scale_x, src.bottom * tex_scale_y),
                        color,
                    },
                    VertexData {
                        position: (dest.left, dest.top),
                        uv: (src.left * tex_scale_x, src.top * tex_scale_y),
                        color,
                    },
                    VertexData {
                        position: (dest.right, dest.top),
                        uv: (src.right * tex_scale_x, src.top * tex_scale_y),
                        color,
                    },
                    VertexData {
                        position: (dest.left, dest.bottom),
                        uv: (src.left * tex_scale_x, src.bottom * tex_scale_y),
                        color,
                    },
                    VertexData {
                        position: (dest.right, dest.top),
                        uv: (src.right * tex_scale_x, src.top * tex_scale_y),
                        color,
                    },
                    VertexData {
                        position: (dest.right, dest.bottom),
                        uv: (src.right * tex_scale_x, src.bottom * tex_scale_y),
                        color,
                    },
                ];
                self.vertices.extend_from_slice(&vertices);
            }
            SpriteBatchCommand::DrawRectFlipTinted(mut src, dest, flip_x, flip_y, color) => {
                if flip_x {
                    std::mem::swap(&mut src.left, &mut src.right);
                }

                if flip_y {
                    std::mem::swap(&mut src.top, &mut src.bottom);
                }

                let color = color.to_rgba();

                let vertices = [
                    VertexData {
                        position: (dest.left, dest.bottom),
                        uv: (src.left * tex_scale_x, src.bottom * tex_scale_y),
                        color,
                    },
                    VertexData {
                        position: (dest.left, dest.top),
                        uv: (src.left * tex_scale_x, src.top * tex_scale_y),
                        color,
                    },
                    VertexData {
                        position: (dest.right, dest.top),
                        uv: (src.right * tex_scale_x, src.top * tex_scale_y),
                        color,
                    },
                    VertexData {
                        position: (dest.left, dest.bottom),
                        uv: (src.left * tex_scale_x, src.bottom * tex_scale_y),
                        color,
                    },
                    VertexData {
                        position: (dest.right, dest.top),
                        uv: (src.right * tex_scale_x, src.top * tex_scale_y),
                        color,
                    },
                    VertexData {
                        position: (dest.right, dest.bottom),
                        uv: (src.right * tex_scale_x, src.bottom * tex_scale_y),
                        color,
                    },
                ];
                self.vertices.extend_from_slice(&vertices);
            }
        }
    }

    fn clear(&mut self) {
        self.vertices.clear();
    }

    fn draw(&mut self) -> GameResult {
        unsafe {
            if let Some(gl) = &GL_PROC {
                if self.texture_id == 0 {
                    return Ok(());
                }

                if gl.gl.BindSampler.is_loaded() {
                    gl.gl.BindSampler(0, 0);
                }

                gl.gl.Enable(gl::TEXTURE_2D);
                gl.gl.Enable(gl::BLEND);
                gl.gl.Disable(gl::DEPTH_TEST);

                self.shader.bind_attrib_pointer(gl, self.vbo);

                gl.gl.BindTexture(gl::TEXTURE_2D, self.texture_id);
                gl.gl.BufferData(
                    gl::ARRAY_BUFFER,
                    (self.vertices.len() * mem::size_of::<VertexData>()) as _,
                    self.vertices.as_ptr() as _,
                    gl::STREAM_DRAW,
                );

                gl.gl.DrawArrays(gl::TRIANGLES, 0, self.vertices.len() as _);

                gl.gl.BindTexture(gl::TEXTURE_2D, 0);
                gl.gl.BindBuffer(gl::ARRAY_BUFFER, 0);

                Ok(())
            } else {
                Err(RenderError("No OpenGL context available!".to_string()))
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for OpenGLTexture {
    fn drop(&mut self) {
        if *self.context_active.as_ref().borrow() {
            unsafe {
                if let Some(gl) = &GL_PROC {
                    if self.texture_id != 0 {
                        let texture_id = &self.texture_id;
                        gl.gl.DeleteTextures(1, texture_id as *const _);
                    }

                    if self.framebuffer_id != 0 {}
                }
            }
        }
    }
}

fn check_shader_compile_status(shader: u32, gl: &Gl) -> GameResult {
    unsafe {
        let mut status: GLint = 0;
        gl.gl.GetShaderiv(shader, gl::COMPILE_STATUS, (&mut status) as *mut _);

        if status == (gl::FALSE as GLint) {
            let mut max_length: GLint = 0;
            let mut msg_length: GLsizei = 0;
            gl.gl.GetShaderiv(shader, gl::INFO_LOG_LENGTH, (&mut max_length) as *mut _);

            let mut data: Vec<u8> = vec![0; max_length as usize];
            gl.gl.GetShaderInfoLog(
                shader,
                max_length as GLsizei,
                (&mut msg_length) as *mut _,
                data.as_mut_ptr() as *mut _,
            );

            let data = String::from_utf8_lossy(&data);
            return Err(GameError::RenderError(format!("Failed to compile shader {}: {}", shader, data)));
        }
    }

    Ok(())
}

const VERTEX_SHADER_BASIC: &str = include_str!("shaders/opengl/vertex_basic_110.glsl");
const FRAGMENT_SHADER_TEXTURED: &str = include_str!("shaders/opengl/fragment_textured_110.glsl");
const FRAGMENT_SHADER_COLOR: &str = include_str!("shaders/opengl/fragment_color_110.glsl");
const FRAGMENT_SHADER_WATER: &str = include_str!("shaders/opengl/fragment_water_110.glsl");

const VERTEX_SHADER_BASIC_GLES: &str = include_str!("shaders/opengles/vertex_basic_100.glsl");
const FRAGMENT_SHADER_TEXTURED_GLES: &str = include_str!("shaders/opengles/fragment_textured_100.glsl");
const FRAGMENT_SHADER_COLOR_GLES: &str = include_str!("shaders/opengles/fragment_color_100.glsl");

#[derive(Copy, Clone)]
struct RenderShader {
    program_id: GLuint,
    texture: GLint,
    proj_mtx: GLint,
    scale: GLint,
    time: GLint,
    frame_offset: GLint,
    position: GLuint,
    uv: GLuint,
    color: GLuint,
}

impl Default for RenderShader {
    fn default() -> Self {
        Self {
            program_id: 0,
            texture: 0,
            proj_mtx: 0,
            scale: 0,
            time: 0,
            frame_offset: 0,
            position: 0,
            uv: 0,
            color: 0,
        }
    }
}

impl RenderShader {
    fn compile(gl: &Gl, vertex_shader: &str, fragment_shader: &str) -> GameResult<RenderShader> {
        let mut shader = RenderShader::default();
        unsafe {
            shader.program_id = gl.gl.CreateProgram();

            unsafe fn cleanup(shader: &mut RenderShader, gl: &Gl, vert: GLuint, frag: GLuint) {
                if vert != 0 {
                    gl.gl.DeleteShader(vert);
                }

                if frag != 0 {
                    gl.gl.DeleteShader(frag);
                }

                if shader.program_id != 0 {
                    gl.gl.DeleteProgram(shader.program_id);
                    shader.program_id = 0;
                }

                *shader = RenderShader::default();
            }

            let vert_shader = gl.gl.CreateShader(gl::VERTEX_SHADER);
            let frag_shader = gl.gl.CreateShader(gl::FRAGMENT_SHADER);

            let vert_sources = [vertex_shader.as_ptr() as *const GLchar];
            let frag_sources = [fragment_shader.as_ptr() as *const GLchar];
            let vert_sources_len = [vertex_shader.len() as GLint - 1];
            let frag_sources_len = [fragment_shader.len() as GLint - 1];

            gl.gl.ShaderSource(vert_shader, 1, vert_sources.as_ptr(), vert_sources_len.as_ptr());
            gl.gl.ShaderSource(frag_shader, 1, frag_sources.as_ptr(), frag_sources_len.as_ptr());

            gl.gl.CompileShader(vert_shader);
            gl.gl.CompileShader(frag_shader);

            if let Err(e) = check_shader_compile_status(vert_shader, gl) {
                cleanup(&mut shader, gl, vert_shader, frag_shader);
                return Err(e);
            }

            if let Err(e) = check_shader_compile_status(frag_shader, gl) {
                cleanup(&mut shader, gl, vert_shader, frag_shader);
                return Err(e);
            }

            gl.gl.AttachShader(shader.program_id, vert_shader);
            gl.gl.AttachShader(shader.program_id, frag_shader);
            gl.gl.LinkProgram(shader.program_id);

            shader.texture = gl.gl.GetUniformLocation(shader.program_id, b"Texture\0".as_ptr() as _);
            shader.proj_mtx = gl.gl.GetUniformLocation(shader.program_id, b"ProjMtx\0".as_ptr() as _);
            shader.scale = gl.gl.GetUniformLocation(shader.program_id, b"Scale\0".as_ptr() as _) as _;
            shader.time = gl.gl.GetUniformLocation(shader.program_id, b"Time\0".as_ptr() as _) as _;
            shader.frame_offset = gl.gl.GetUniformLocation(shader.program_id, b"FrameOffset\0".as_ptr() as _) as _;
            shader.position = gl.gl.GetAttribLocation(shader.program_id, b"Position\0".as_ptr() as _) as _;
            shader.uv = gl.gl.GetAttribLocation(shader.program_id, b"UV\0".as_ptr() as _) as _;
            shader.color = gl.gl.GetAttribLocation(shader.program_id, b"Color\0".as_ptr() as _) as _;
        }

        Ok(shader)
    }

    unsafe fn bind_attrib_pointer(&self, gl: &Gl, vbo: GLuint) -> GameResult {
        gl.gl.UseProgram(self.program_id);
        gl.gl.BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl.gl.EnableVertexAttribArray(self.position);
        gl.gl.EnableVertexAttribArray(self.uv);
        gl.gl.EnableVertexAttribArray(self.color);

        gl.gl.VertexAttribPointer(
            self.position,
            2,
            gl::FLOAT,
            gl::FALSE,
            mem::size_of::<VertexData>() as _,
            field_offset::<VertexData, _, _>(|v| &v.position) as _,
        );

        gl.gl.VertexAttribPointer(
            self.uv,
            2,
            gl::FLOAT,
            gl::FALSE,
            mem::size_of::<VertexData>() as _,
            field_offset::<VertexData, _, _>(|v| &v.uv) as _,
        );

        gl.gl.VertexAttribPointer(
            self.color,
            4,
            gl::UNSIGNED_BYTE,
            gl::TRUE,
            mem::size_of::<VertexData>() as _,
            field_offset::<VertexData, _, _>(|v| &v.color) as _,
        );

        Ok(())
    }
}

struct RenderData {
    initialized: bool,
    tex_shader: RenderShader,
    fill_shader: RenderShader,
    fill_water_shader: RenderShader,
    vbo: GLuint,
    ebo: GLuint,
    font_texture: GLuint,
    font_tex_size: (f32, f32),
    surf_framebuffer: GLuint,
    surf_texture: GLuint,
    last_size: (u32, u32),
}

impl RenderData {
    fn new() -> Self {
        RenderData {
            initialized: false,
            tex_shader: RenderShader::default(),
            fill_shader: RenderShader::default(),
            fill_water_shader: RenderShader::default(),
            vbo: 0,
            ebo: 0,
            font_texture: 0,
            font_tex_size: (1.0, 1.0),
            surf_framebuffer: 0,
            surf_texture: 0,
            last_size: (320, 240),
        }
    }

    fn init(&mut self, gles2_mode: bool, imgui: &mut imgui::Context, gl: &Gl) {
        self.initialized = true;

        let vshdr_basic = if gles2_mode { VERTEX_SHADER_BASIC_GLES } else { VERTEX_SHADER_BASIC };
        let fshdr_tex = if gles2_mode { FRAGMENT_SHADER_TEXTURED_GLES } else { FRAGMENT_SHADER_TEXTURED };
        let fshdr_fill = if gles2_mode { FRAGMENT_SHADER_COLOR_GLES } else { FRAGMENT_SHADER_COLOR };
        let fshdr_fill_water = if gles2_mode { FRAGMENT_SHADER_COLOR_GLES } else { FRAGMENT_SHADER_WATER };

        unsafe {
            self.tex_shader =
                RenderShader::compile(gl, vshdr_basic, fshdr_tex).unwrap_or_else(|_| RenderShader::default());
            self.fill_shader =
                RenderShader::compile(gl, vshdr_basic, fshdr_fill).unwrap_or_else(|_| RenderShader::default());
            self.fill_water_shader =
                RenderShader::compile(gl, vshdr_basic, fshdr_fill_water).unwrap_or_else(|_| RenderShader::default());

            self.vbo = return_param(|x| gl.gl.GenBuffers(1, x));
            self.ebo = return_param(|x| gl.gl.GenBuffers(1, x));

            self.font_texture = return_param(|x| gl.gl.GenTextures(1, x));
            gl.gl.BindTexture(gl::TEXTURE_2D, self.font_texture);
            gl.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
            gl.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);

            {
                let mut atlas = imgui.fonts();

                let texture = atlas.build_rgba32_texture();
                self.font_tex_size = (texture.width as _, texture.height as _);

                gl.gl.TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as _,
                    texture.width as _,
                    texture.height as _,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    texture.data.as_ptr() as _,
                );

                atlas.tex_id = (self.font_texture as usize).into();
            }

            let texture_id = return_param(|x| gl.gl.GenTextures(1, x));

            gl.gl.BindTexture(gl::TEXTURE_2D, texture_id);
            gl.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
            gl.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
            gl.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
            gl.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);

            gl.gl.TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as _,
                320 as _,
                240 as _,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                null() as _,
            );

            gl.gl.BindTexture(gl::TEXTURE_2D, 0 as _);

            self.surf_texture = texture_id;

            let framebuffer_id = return_param(|x| gl.gl.GenFramebuffers(1, x));

            gl.gl.BindFramebuffer(gl::FRAMEBUFFER, framebuffer_id);
            gl.gl.FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, texture_id, 0);
            let draw_buffers = [gl::COLOR_ATTACHMENT0];
            gl.gl.DrawBuffers(1, draw_buffers.as_ptr() as _);

            self.surf_framebuffer = framebuffer_id;
        }
    }
}

pub struct Gl {
    pub gl: gl::Gles2,
}

static mut GL_PROC: Option<Gl> = None;

pub fn load_gl(gl_context: &mut GLContext) -> &'static Gl {
    unsafe {
        if let Some(gl) = &GL_PROC {
            return gl;
        }

        let gl = gl::Gles2::load_with(|ptr| (gl_context.get_proc_address)(&mut gl_context.user_data, ptr));

        let version = {
            let p = gl.GetString(gl::VERSION);
            if p.is_null() {
                "unknown".to_owned()
            } else {
                let data = CStr::from_ptr(p as *const _).to_bytes().to_vec();
                String::from_utf8(data).unwrap()
            }
        };

        log::info!("OpenGL version {}", version);

        GL_PROC = Some(Gl { gl });
        GL_PROC.as_ref().unwrap()
    }
}

pub struct OpenGLRenderer {
    refs: GLContext,
    imgui: UnsafeCell<imgui::Context>,
    render_data: RenderData,
    context_active: Arc<RefCell<bool>>,
    def_matrix: [[f32; 4]; 4],
    curr_matrix: [[f32; 4]; 4],
}

impl OpenGLRenderer {
    pub fn new(refs: GLContext, imgui: UnsafeCell<imgui::Context>) -> OpenGLRenderer {
        OpenGLRenderer {
            refs,
            imgui,
            render_data: RenderData::new(),
            context_active: Arc::new(RefCell::new(true)),
            def_matrix: [[0.0; 4]; 4],
            curr_matrix: [[0.0; 4]; 4],
        }
    }

    fn get_context(&mut self) -> Option<(&mut GLContext, &'static Gl)> {
        let imgui = unsafe { &mut *self.imgui.get() };

        let gles2 = self.refs.gles2_mode;
        let gl = load_gl(&mut self.refs);

        if !self.render_data.initialized {
            self.render_data.init(gles2, imgui, gl);
        }

        Some((&mut self.refs, gl))
    }
}

impl BackendRenderer for OpenGLRenderer {
    fn renderer_name(&self) -> String {
        if self.refs.gles2_mode {
            "OpenGL ES 2.0".to_string()
        } else {
            "OpenGL 2.1".to_string()
        }
    }

    fn clear(&mut self, color: Color) {
        if let Some((_, gl)) = self.get_context() {
            unsafe {
                gl.gl.ClearColor(color.r, color.g, color.b, color.a);
                gl.gl.Clear(gl::COLOR_BUFFER_BIT);
            }
        }
    }

    fn present(&mut self) -> GameResult {
        {
            let mutex = GAME_SUSPENDED.lock().unwrap();
            if *mutex {
                return Ok(());
            }
        }

        unsafe {
            if let Some((_, gl)) = self.get_context() {
                gl.gl.BindFramebuffer(gl::FRAMEBUFFER, 0);
                gl.gl.ClearColor(0.0, 0.0, 0.0, 1.0);
                gl.gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                let matrix =
                    [[2.0f32, 0.0, 0.0, 0.0], [0.0, -2.0, 0.0, 0.0], [0.0, 0.0, -1.0, 0.0], [-1.0, 1.0, 0.0, 1.0]];

                self.render_data.tex_shader.bind_attrib_pointer(gl, self.render_data.vbo);
                gl.gl.UniformMatrix4fv(self.render_data.tex_shader.proj_mtx, 1, gl::FALSE, matrix.as_ptr() as _);

                let color = (255, 255, 255, 255);
                let vertices = [
                    VertexData { position: (0.0, 1.0), uv: (0.0, 0.0), color },
                    VertexData { position: (0.0, 0.0), uv: (0.0, 1.0), color },
                    VertexData { position: (1.0, 0.0), uv: (1.0, 1.0), color },
                    VertexData { position: (0.0, 1.0), uv: (0.0, 0.0), color },
                    VertexData { position: (1.0, 0.0), uv: (1.0, 1.0), color },
                    VertexData { position: (1.0, 1.0), uv: (1.0, 0.0), color },
                ];

                self.draw_arrays_tex_id(
                    gl::TRIANGLES,
                    &vertices,
                    self.render_data.surf_texture,
                    BackendShader::Texture,
                )?;
            }

            if let Some((context, _)) = self.get_context() {
                (context.swap_buffers)(&mut context.user_data);
            }
        }

        Ok(())
    }

    fn set_vsync_mode(&mut self, mode: VSyncMode) -> GameResult {
        if !self.refs.is_sdl {
            return Ok(());
        }

        #[cfg(feature = "backend-sdl")]
            unsafe {
            let ctx = &mut *self.refs.ctx;

            match mode {
                VSyncMode::Uncapped => {
                    sdl2_sys::SDL_GL_SetSwapInterval(0);
                }
                VSyncMode::VSync => {
                    sdl2_sys::SDL_GL_SetSwapInterval(1);
                }
                _ => {
                    if sdl2_sys::SDL_GL_SetSwapInterval(-1) == -1 {
                        log::warn!("Failed to enable variable refresh rate, falling back to non-V-Sync.");
                        sdl2_sys::SDL_GL_SetSwapInterval(0);
                    }
                }
            }
        }

        Ok(())
    }

    fn prepare_draw(&mut self, width: f32, height: f32) -> GameResult {
        if let Some((_, gl)) = self.get_context() {
            unsafe {
                let (width_u, height_u) = (width as u32, height as u32);
                if self.render_data.last_size != (width_u, height_u) {
                    self.render_data.last_size = (width_u, height_u);
                    gl.gl.BindFramebuffer(gl::FRAMEBUFFER, 0);
                    gl.gl.BindTexture(gl::TEXTURE_2D, self.render_data.surf_texture);

                    gl.gl.TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        gl::RGBA as _,
                        width_u as _,
                        height_u as _,
                        0,
                        gl::RGBA,
                        gl::UNSIGNED_BYTE,
                        null() as _,
                    );

                    gl.gl.BindTexture(gl::TEXTURE_2D, 0 as _);
                }

                gl.gl.BindFramebuffer(gl::FRAMEBUFFER, self.render_data.surf_framebuffer);
                gl.gl.ClearColor(0.0, 0.0, 0.0, 0.0);
                gl.gl.Clear(gl::COLOR_BUFFER_BIT);

                gl.gl.ActiveTexture(gl::TEXTURE0);
                gl.gl.BlendEquation(gl::FUNC_ADD);
                gl.gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

                gl.gl.Viewport(0, 0, width_u as _, height_u as _);

                self.def_matrix = [
                    [2.0 / width, 0.0, 0.0, 0.0],
                    [0.0, 2.0 / -height, 0.0, 0.0],
                    [0.0, 0.0, -1.0, 0.0],
                    [-1.0, 1.0, 0.0, 1.0],
                ];
                self.curr_matrix = self.def_matrix;

                gl.gl.BindBuffer(gl::ARRAY_BUFFER, 0);
                gl.gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
                gl.gl.UseProgram(self.render_data.fill_shader.program_id);
                gl.gl.UniformMatrix4fv(
                    self.render_data.fill_shader.proj_mtx,
                    1,
                    gl::FALSE,
                    self.curr_matrix.as_ptr() as _,
                );
                gl.gl.UseProgram(self.render_data.fill_water_shader.program_id);
                gl.gl.Uniform1i(self.render_data.fill_water_shader.texture, 0);
                gl.gl.UniformMatrix4fv(
                    self.render_data.fill_water_shader.proj_mtx,
                    1,
                    gl::FALSE,
                    self.curr_matrix.as_ptr() as _,
                );
                gl.gl.UseProgram(self.render_data.tex_shader.program_id);
                gl.gl.Uniform1i(self.render_data.tex_shader.texture, 0);
                gl.gl.UniformMatrix4fv(
                    self.render_data.tex_shader.proj_mtx,
                    1,
                    gl::FALSE,
                    self.curr_matrix.as_ptr() as _,
                );
            }

            Ok(())
        } else {
            Err(RenderError("No OpenGL context available!".to_string()))
        }
    }

    fn create_texture_mutable(&mut self, width: u16, height: u16) -> GameResult<Box<dyn BackendTexture>> {
        if let Some((_, gl)) = self.get_context() {
            unsafe {
                let current_texture_id = return_param(|x| gl.gl.GetIntegerv(gl::TEXTURE_BINDING_2D, x)) as u32;
                let texture_id = return_param(|x| gl.gl.GenTextures(1, x));

                gl.gl.BindTexture(gl::TEXTURE_2D, texture_id);
                gl.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
                gl.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);

                gl.gl.TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as _,
                    width as _,
                    height as _,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    null() as _,
                );

                gl.gl.BindTexture(gl::TEXTURE_2D, current_texture_id);

                let framebuffer_id = return_param(|x| gl.gl.GenFramebuffers(1, x));

                gl.gl.BindFramebuffer(gl::FRAMEBUFFER, framebuffer_id);
                gl.gl.FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, texture_id, 0);
                let draw_buffers = [gl::COLOR_ATTACHMENT0];
                gl.gl.DrawBuffers(1, draw_buffers.as_ptr() as _);

                gl.gl.Viewport(0, 0, width as _, height as _);
                gl.gl.ClearColor(0.0, 0.0, 0.0, 0.0);
                gl.gl.Clear(gl::COLOR_BUFFER_BIT);
                gl.gl.BindFramebuffer(gl::FRAMEBUFFER, 0);

                // todo error checking: glCheckFramebufferStatus()

                Ok(Box::new(OpenGLTexture {
                    texture_id,
                    framebuffer_id,
                    width,
                    height,
                    vertices: Vec::new(),
                    shader: self.render_data.tex_shader,
                    vbo: self.render_data.vbo,
                    context_active: self.context_active.clone(),
                }))
            }
        } else {
            Err(RenderError("No OpenGL context available!".to_string()))
        }
    }

    fn create_texture(&mut self, width: u16, height: u16, data: &[u8]) -> GameResult<Box<dyn BackendTexture>> {
        if let Some((_, gl)) = self.get_context() {
            unsafe {
                let current_texture_id = return_param(|x| gl.gl.GetIntegerv(gl::TEXTURE_BINDING_2D, x)) as u32;
                let texture_id = return_param(|x| gl.gl.GenTextures(1, x));
                gl.gl.BindTexture(gl::TEXTURE_2D, texture_id);
                gl.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
                gl.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);

                gl.gl.TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as _,
                    width as _,
                    height as _,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    data.as_ptr() as _,
                );

                gl.gl.BindTexture(gl::TEXTURE_2D, current_texture_id);

                Ok(Box::new(OpenGLTexture {
                    texture_id,
                    framebuffer_id: 0,
                    width,
                    height,
                    vertices: Vec::new(),
                    shader: self.render_data.tex_shader,
                    vbo: self.render_data.vbo,
                    context_active: self.context_active.clone(),
                }))
            }
        } else {
            Err(RenderError("No OpenGL context available!".to_string()))
        }
    }

    fn set_blend_mode(&mut self, blend: BlendMode) -> GameResult {
        if let Some((_, gl)) = self.get_context() {
            match blend {
                BlendMode::Add => unsafe {
                    gl.gl.Enable(gl::BLEND);
                    gl.gl.BlendEquation(gl::FUNC_ADD);
                    gl.gl.BlendFunc(gl::ONE, gl::ONE);
                },
                BlendMode::Alpha => unsafe {
                    gl.gl.Enable(gl::BLEND);
                    gl.gl.BlendEquation(gl::FUNC_ADD);
                    gl.gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                },
                BlendMode::Multiply => unsafe {
                    gl.gl.Enable(gl::BLEND);
                    gl.gl.BlendEquation(gl::FUNC_ADD);
                    gl.gl.BlendFuncSeparate(gl::ZERO, gl::SRC_COLOR, gl::ZERO, gl::SRC_ALPHA);
                },
                BlendMode::None => unsafe {
                    gl.gl.Disable(gl::BLEND);
                },
            }

            Ok(())
        } else {
            Err(RenderError("No OpenGL context available!".to_string()))
        }
    }

    fn set_render_target(&mut self, texture: Option<&Box<dyn BackendTexture>>) -> GameResult {
        if let Some((_, gl)) = self.get_context() {
            unsafe {
                if let Some(texture) = texture {
                    let gl_texture = texture
                        .as_any()
                        .downcast_ref::<OpenGLTexture>()
                        .ok_or_else(|| RenderError("This texture was not created by OpenGL backend.".to_string()))?;

                    self.curr_matrix = [
                        [2.0 / (gl_texture.width as f32), 0.0, 0.0, 0.0],
                        [0.0, 2.0 / (gl_texture.height as f32), 0.0, 0.0],
                        [0.0, 0.0, -1.0, 0.0],
                        [-1.0, -1.0, 0.0, 1.0],
                    ];

                    gl.gl.UseProgram(self.render_data.fill_shader.program_id);
                    gl.gl.UniformMatrix4fv(
                        self.render_data.fill_shader.proj_mtx,
                        1,
                        gl::FALSE,
                        self.curr_matrix.as_ptr() as _,
                    );
                    gl.gl.UseProgram(self.render_data.fill_water_shader.program_id);
                    gl.gl.UniformMatrix4fv(
                        self.render_data.fill_water_shader.proj_mtx,
                        1,
                        gl::FALSE,
                        self.curr_matrix.as_ptr() as _,
                    );
                    gl.gl.UseProgram(self.render_data.tex_shader.program_id);
                    gl.gl.Uniform1i(self.render_data.tex_shader.texture, 0);
                    gl.gl.UniformMatrix4fv(
                        self.render_data.tex_shader.proj_mtx,
                        1,
                        gl::FALSE,
                        self.curr_matrix.as_ptr() as _,
                    );

                    gl.gl.BindFramebuffer(gl::FRAMEBUFFER, gl_texture.framebuffer_id);
                    gl.gl.Viewport(0, 0, gl_texture.width as _, gl_texture.height as _);
                } else {
                    self.curr_matrix = self.def_matrix;

                    gl.gl.UseProgram(self.render_data.fill_shader.program_id);
                    gl.gl.UniformMatrix4fv(
                        self.render_data.fill_shader.proj_mtx,
                        1,
                        gl::FALSE,
                        self.curr_matrix.as_ptr() as _,
                    );
                    gl.gl.UseProgram(self.render_data.fill_water_shader.program_id);
                    gl.gl.UniformMatrix4fv(
                        self.render_data.fill_water_shader.proj_mtx,
                        1,
                        gl::FALSE,
                        self.curr_matrix.as_ptr() as _,
                    );
                    gl.gl.UseProgram(self.render_data.tex_shader.program_id);
                    gl.gl.Uniform1i(self.render_data.tex_shader.texture, 0);
                    gl.gl.UniformMatrix4fv(
                        self.render_data.tex_shader.proj_mtx,
                        1,
                        gl::FALSE,
                        self.curr_matrix.as_ptr() as _,
                    );
                    gl.gl.BindFramebuffer(gl::FRAMEBUFFER, self.render_data.surf_framebuffer);
                    gl.gl.Viewport(0, 0, self.render_data.last_size.0 as _, self.render_data.last_size.1 as _);
                }
            }

            Ok(())
        } else {
            Err(RenderError("No OpenGL context available!".to_string()))
        }
    }

    fn draw_rect(&mut self, rect: Rect<isize>, color: Color) -> GameResult {
        unsafe {
            if let Some(gl) = &GL_PROC {
                let color = color.to_rgba();
                let mut uv = self.render_data.font_tex_size;
                uv.0 = 0.0 / uv.0;
                uv.1 = 0.0 / uv.1;

                let vertices = [
                    VertexData { position: (rect.left as _, rect.bottom as _), uv, color },
                    VertexData { position: (rect.left as _, rect.top as _), uv, color },
                    VertexData { position: (rect.right as _, rect.top as _), uv, color },
                    VertexData { position: (rect.left as _, rect.bottom as _), uv, color },
                    VertexData { position: (rect.right as _, rect.top as _), uv, color },
                    VertexData { position: (rect.right as _, rect.bottom as _), uv, color },
                ];

                self.render_data.fill_shader.bind_attrib_pointer(gl, self.render_data.vbo);

                gl.gl.BindTexture(gl::TEXTURE_2D, self.render_data.font_texture);
                gl.gl.BindBuffer(gl::ARRAY_BUFFER, self.render_data.vbo);
                gl.gl.BufferData(
                    gl::ARRAY_BUFFER,
                    (vertices.len() * mem::size_of::<VertexData>()) as _,
                    vertices.as_ptr() as _,
                    gl::STREAM_DRAW,
                );

                gl.gl.DrawArrays(gl::TRIANGLES, 0, vertices.len() as _);

                gl.gl.BindTexture(gl::TEXTURE_2D, 0);
                gl.gl.BindBuffer(gl::ARRAY_BUFFER, 0);

                Ok(())
            } else {
                Err(RenderError("No OpenGL context available!".to_string()))
            }
        }
    }

    fn draw_outline_rect(&mut self, _rect: Rect<isize>, _line_width: usize, _color: Color) -> GameResult {
        Ok(())
    }

    fn set_clip_rect(&mut self, rect: Option<Rect>) -> GameResult {
        if let Some((_, gl)) = self.get_context() {
            unsafe {
                if let Some(rect) = &rect {
                    gl.gl.Enable(gl::SCISSOR_TEST);
                    gl.gl.Scissor(
                        rect.left as GLint,
                        self.render_data.last_size.1 as GLint - rect.bottom as GLint,
                        rect.width() as GLint,
                        rect.height() as GLint,
                    );
                } else {
                    gl.gl.Disable(gl::SCISSOR_TEST);
                }
            }

            Ok(())
        } else {
            Err(RenderError("No OpenGL context available!".to_string()))
        }
    }

    fn imgui(&self) -> GameResult<&mut imgui::Context> {
        unsafe { Ok(&mut *self.imgui.get()) }
    }

    fn imgui_texture_id(&self, texture: &Box<dyn BackendTexture>) -> GameResult<TextureId> {
        let gl_texture = texture
            .as_any()
            .downcast_ref::<OpenGLTexture>()
            .ok_or_else(|| RenderError("This texture was not created by OpenGL backend.".to_string()))?;

        Ok(TextureId::new(gl_texture.texture_id as usize))
    }

    fn prepare_imgui(&mut self, _ui: &Ui) -> GameResult {
        Ok(())
    }

    fn render_imgui(&mut self, draw_data: &DrawData) -> GameResult {
        // https://github.com/michaelfairley/rust-imgui-opengl-renderer
        if let Some((_, gl)) = self.get_context() {
            unsafe {
                gl.gl.ActiveTexture(gl::TEXTURE0);
                gl.gl.Enable(gl::BLEND);
                gl.gl.BlendEquation(gl::FUNC_ADD);
                gl.gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl.gl.Disable(gl::CULL_FACE);
                gl.gl.Disable(gl::DEPTH_TEST);
                gl.gl.Enable(gl::SCISSOR_TEST);

                let imgui = self.imgui()?;
                let [width, height] = imgui.io().display_size;
                let [scale_w, scale_h] = imgui.io().display_framebuffer_scale;

                let fb_width = width * scale_w;
                let fb_height = height * scale_h;

                gl.gl.Viewport(0, 0, fb_width as _, fb_height as _);
                let matrix = [
                    [2.0 / width as f32, 0.0, 0.0, 0.0],
                    [0.0, 2.0 / -(height as f32), 0.0, 0.0],
                    [0.0, 0.0, -1.0, 0.0],
                    [-1.0, 1.0, 0.0, 1.0],
                ];

                gl.gl.UseProgram(self.render_data.tex_shader.program_id);
                gl.gl.Uniform1i(self.render_data.tex_shader.texture, 0);
                gl.gl.UniformMatrix4fv(self.render_data.tex_shader.proj_mtx, 1, gl::FALSE, matrix.as_ptr() as _);

                if gl.gl.BindSampler.is_loaded() {
                    gl.gl.BindSampler(0, 0);
                }

                gl.gl.BindBuffer(gl::ARRAY_BUFFER, self.render_data.vbo);
                gl.gl.EnableVertexAttribArray(self.render_data.tex_shader.position);
                gl.gl.EnableVertexAttribArray(self.render_data.tex_shader.uv);
                gl.gl.EnableVertexAttribArray(self.render_data.tex_shader.color);

                gl.gl.VertexAttribPointer(
                    self.render_data.tex_shader.position,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    mem::size_of::<DrawVert>() as _,
                    field_offset::<DrawVert, _, _>(|v| &v.pos) as _,
                );

                gl.gl.VertexAttribPointer(
                    self.render_data.tex_shader.uv,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    mem::size_of::<DrawVert>() as _,
                    field_offset::<DrawVert, _, _>(|v| &v.uv) as _,
                );

                gl.gl.VertexAttribPointer(
                    self.render_data.tex_shader.color,
                    4,
                    gl::UNSIGNED_BYTE,
                    gl::TRUE,
                    mem::size_of::<DrawVert>() as _,
                    field_offset::<DrawVert, _, _>(|v| &v.col) as _,
                );

                for draw_list in draw_data.draw_lists() {
                    let vtx_buffer = draw_list.vtx_buffer();
                    let idx_buffer = draw_list.idx_buffer();

                    gl.gl.BindBuffer(gl::ARRAY_BUFFER, self.render_data.vbo);
                    gl.gl.BufferData(
                        gl::ARRAY_BUFFER,
                        (vtx_buffer.len() * mem::size_of::<DrawVert>()) as _,
                        vtx_buffer.as_ptr() as _,
                        gl::STREAM_DRAW,
                    );

                    gl.gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.render_data.ebo);
                    gl.gl.BufferData(
                        gl::ELEMENT_ARRAY_BUFFER,
                        (idx_buffer.len() * mem::size_of::<DrawIdx>()) as _,
                        idx_buffer.as_ptr() as _,
                        gl::STREAM_DRAW,
                    );

                    for cmd in draw_list.commands() {
                        match cmd {
                            DrawCmd::Elements {
                                count,
                                cmd_params: DrawCmdParams { clip_rect: [x, y, z, w], texture_id, idx_offset, .. },
                            } => {
                                gl.gl.BindTexture(gl::TEXTURE_2D, texture_id.id() as _);

                                gl.gl.Scissor(
                                    (x * scale_w) as GLint,
                                    (fb_height - w * scale_h) as GLint,
                                    ((z - x) * scale_w) as GLint,
                                    ((w - y) * scale_h) as GLint,
                                );

                                let idx_size =
                                    if mem::size_of::<DrawIdx>() == 2 { gl::UNSIGNED_SHORT } else { gl::UNSIGNED_INT };

                                gl.gl.DrawElements(
                                    gl::TRIANGLES,
                                    count as _,
                                    idx_size,
                                    (idx_offset * mem::size_of::<DrawIdx>()) as _,
                                );
                            }
                            DrawCmd::ResetRenderState => {}
                            DrawCmd::RawCallback { .. } => {}
                        }
                    }
                }

                gl.gl.Disable(gl::SCISSOR_TEST);
            }
        }

        Ok(())
    }

    fn supports_vertex_draw(&self) -> bool {
        true
    }

    fn draw_triangle_list(
        &mut self,
        vertices: &[VertexData],
        texture: Option<&Box<dyn BackendTexture>>,
        shader: BackendShader,
    ) -> GameResult<()> {
        self.draw_arrays(gl::TRIANGLES, vertices, texture, shader)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl OpenGLRenderer {
    fn draw_arrays(
        &mut self,
        vert_type: GLenum,
        vertices: &[VertexData],
        texture: Option<&Box<dyn BackendTexture>>,
        shader: BackendShader,
    ) -> GameResult<()> {
        if vertices.is_empty() {
            return Ok(());
        }

        let texture_id = if let Some(texture) = texture {
            let gl_texture = texture
                .as_any()
                .downcast_ref::<OpenGLTexture>()
                .ok_or_else(|| RenderError("This texture was not created by OpenGL backend.".to_string()))?;

            gl_texture.texture_id
        } else {
            0
        };

        unsafe { self.draw_arrays_tex_id(vert_type, vertices, texture_id, shader) }
    }

    unsafe fn draw_arrays_tex_id(
        &mut self,
        vert_type: GLenum,
        vertices: &[VertexData],
        mut texture: u32,
        shader: BackendShader,
    ) -> GameResult<()> {
        if let Some(gl) = &GL_PROC {
            match shader {
                BackendShader::Fill => {
                    self.render_data.fill_shader.bind_attrib_pointer(gl, self.render_data.vbo)?;
                }
                BackendShader::Texture => {
                    self.render_data.tex_shader.bind_attrib_pointer(gl, self.render_data.vbo)?;
                }
                BackendShader::WaterFill(scale, t, frame_pos) => {
                    self.render_data.fill_water_shader.bind_attrib_pointer(gl, self.render_data.vbo)?;
                    gl.gl.Uniform1f(self.render_data.fill_water_shader.scale, scale);
                    gl.gl.Uniform1f(self.render_data.fill_water_shader.time, t);
                    gl.gl.Uniform2f(self.render_data.fill_water_shader.frame_offset, frame_pos.0, frame_pos.1);
                    texture = self.render_data.surf_texture;
                }
            }

            gl.gl.BindTexture(gl::TEXTURE_2D, texture);
            gl.gl.BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<VertexData>()) as _,
                vertices.as_ptr() as _,
                gl::STREAM_DRAW,
            );

            gl.gl.DrawArrays(vert_type, 0, vertices.len() as _);

            gl.gl.BindTexture(gl::TEXTURE_2D, 0);
            gl.gl.BindBuffer(gl::ARRAY_BUFFER, 0);

            Ok(())
        } else {
            Err(RenderError("No OpenGL context available!".to_string()))
        }
    }
}

impl Drop for OpenGLRenderer {
    fn drop(&mut self) {
        *self.context_active.as_ref().borrow_mut() = false;
    }
}
