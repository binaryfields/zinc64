// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::rc::Rc;

use cgmath::{Matrix4, SquareMatrix};

use crate::gfx::gl::{self, FrontFace, GlDevice};
use crate::gfx::rect::RectI;
use crate::gfx::{Color, Rect};
use cgmath::num_traits::zero;

pub const DEFAULT_VERTEX_SHADER: &str = r#"
    #version 330 core

    layout (location = 0) in vec2 a_position;
    layout (location = 1) in vec2 a_uv;
    layout (location = 2) in vec4 a_color;

    out vec2 v_uv;
    out vec4 v_color;

    uniform mat4 u_projection;

    void main() {
        v_uv = a_uv;
        v_color = a_color;
        gl_Position = u_projection * vec4(a_position, 0.0, 1.0);
    }
    "#;

pub const DEFAULT_FRAGMENT_SHADER: &str = r#"
    #version 330 core

    in vec2 v_uv;
    in vec4 v_color;
    out vec4 o_color;
    uniform sampler2D u_texture;

    void main() {
        o_color = v_color * texture(u_texture, v_uv);
    }
    "#;

const INDICES_PER_SPRITE: usize = 6;
const VERTICES_PER_SPRITE: usize = 4;
const VERTEX_SIZE: usize = 8;

const SPRITE_INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

pub struct Batch {
    // Configuration
    max_sprites: usize,
    projection: Matrix4<f32>,
    render_options: gl::RenderOptions,
    viewport: RectI,
    // Resources
    #[allow(unused)]
    default_shader: Rc<gl::Shader>,
    shader: Rc<gl::Shader>,
    target: Option<Rc<gl::Framebuffer>>,
    texture: Option<Rc<gl::Texture>>,
    vertex_array: gl::VertexArray,
    vertex_buffer: gl::Buffer,
    #[allow(unused)]
    index_buffer: gl::Buffer,
    // State
    count: usize,
    vertices: Vec<f32>,
}

impl Batch {
    pub fn new(gl: &mut GlDevice, max_sprites: usize) -> Result<Self, String> {
        let vertices = Vec::with_capacity(max_sprites * VERTICES_PER_SPRITE * VERTEX_SIZE);
        let indices: Vec<u32> = SPRITE_INDICES
            .iter()
            .cycle()
            .take(max_sprites * INDICES_PER_SPRITE)
            .enumerate()
            .map(|(i, vertex)| ((i / INDICES_PER_SPRITE) * VERTICES_PER_SPRITE) as u32 + *vertex)
            .collect();
        // Resources
        let default_shader = Rc::new(Batch::default_shader(gl)?);
        let shader = default_shader.clone();

        let vertex_array = gl.create_vertex_array()?;
        let vertex_buffer = gl.create_buffer(
            gl::BufferType::Float,
            max_sprites * VERTICES_PER_SPRITE * VERTEX_SIZE,
            gl::BufferTarget::Vertex,
            gl::BufferUsage::Dynamic,
        )?;
        gl.bind_buffer(&vertex_array, &vertex_buffer, gl::BufferTarget::Vertex);
        gl.set_vertex_attr(
            &vertex_array,
            0,
            &gl::VertexAttrDescriptor {
                ty: gl::BufferType::Float,
                size: 2,
                stride: VERTEX_SIZE,
                offset: 0,
            },
        );
        gl.set_vertex_attr(
            &vertex_array,
            1,
            &gl::VertexAttrDescriptor {
                ty: gl::BufferType::Float,
                size: 2,
                stride: VERTEX_SIZE,
                offset: 2,
            },
        );
        gl.set_vertex_attr(
            &vertex_array,
            2,
            &gl::VertexAttrDescriptor {
                ty: gl::BufferType::Float,
                size: 4,
                stride: VERTEX_SIZE,
                offset: 4,
            },
        );
        let index_buffer = gl.create_buffer(
            gl::BufferType::UInt,
            indices.len(),
            gl::BufferTarget::Index,
            gl::BufferUsage::Static,
        )?;
        gl.bind_buffer(&vertex_array, &index_buffer, gl::BufferTarget::Index);
        gl.set_buffer_data(&index_buffer, gl::BufferTarget::Index, &indices);

        let projection = Matrix4::identity();
        let render_options = gl::RenderOptions {
            blend_func: Some(gl::BlendFunc::SrcAlphaOneMinusSrcAlpha),
            front_face: Some(gl::FrontFace::CounterClockwise),
        };
        Ok(Batch {
            max_sprites,
            projection,
            render_options,
            viewport: RectI::new(zero(), zero()),
            default_shader,
            shader,
            target: None,
            texture: None,
            vertex_array,
            index_buffer,
            vertex_buffer,
            count: 0,
            vertices,
        })
    }

    pub fn default_shader(gl: &mut GlDevice) -> Result<gl::Shader, String> {
        gl.create_shader(DEFAULT_VERTEX_SHADER, DEFAULT_FRAGMENT_SHADER)
    }

    pub fn begin(&mut self, _gl: &mut GlDevice, texture: Option<Rc<gl::Texture>>) {
        self.count = 0;
        self.texture = texture;
    }

    pub fn end(&mut self, gl: &mut GlDevice) {
        self.flush(gl);
    }

    pub fn flush(&mut self, gl: &mut GlDevice) {
        if self.count > 0 {
            gl.set_buffer_data(
                &self.vertex_buffer,
                gl::BufferTarget::Vertex,
                &self.vertices,
            );
            let target = self
                .target
                .as_ref()
                .map(|fb| gl::RenderTarget::Framebuffer(fb))
                .unwrap_or(gl::RenderTarget::Default);
            gl.draw_elements(
                self.count * INDICES_PER_SPRITE,
                0,
                &gl::RenderState {
                    target: &target,
                    viewport: self.viewport,
                    shader: &self.shader,
                    vertex_array: &self.vertex_array,
                    uniforms: &[
                        (
                            &gl.get_uniform(&self.shader, "u_projection"),
                            gl::UniformData::Mat4(self.projection.clone()),
                        ),
                        (
                            &gl.get_uniform(&self.shader, "u_texture"),
                            gl::UniformData::Int(0),
                        ),
                    ],
                    textures: &[self.texture.as_ref().unwrap().as_ref()],
                    options: self.render_options,
                },
            );
            self.vertices.clear();
            self.count = 0;
        }
    }

    pub fn push(&mut self, gl: &mut GlDevice, dst: Rect, src: Rect, color: Color) {
        self.push_raw(
            gl, dst.p1.x, dst.p1.y, dst.p2.x, dst.p2.y, src.p1.x, src.p1.y, src.p2.x, src.p2.y,
            color,
        );
    }

    pub fn push_raw(
        &mut self,
        gl: &mut GlDevice,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        u1: f32,
        v1: f32,
        u2: f32,
        v2: f32,
        color: Color,
    ) {
        if self.count == self.max_sprites {
            self.flush(gl);
        }
        self.vertices.extend_from_slice(&[
            x1,
            y1,
            u1,
            v2,
            color.r(),
            color.g(),
            color.b(),
            color.a(), // v1 - lower left
            x2,
            y1,
            u2,
            v2,
            color.r(),
            color.g(),
            color.b(),
            color.a(), // v2 - lower right
            x2,
            y2,
            u2,
            v1,
            color.r(),
            color.g(),
            color.b(),
            color.a(), // v3 - top right
            x1,
            y2,
            u1,
            v1,
            color.r(),
            color.g(),
            color.b(),
            color.a(), // v4 -- top left
        ]);
        self.count += 1;
    }

    #[allow(unused)]
    pub fn reset_target(&mut self, gl: &mut GlDevice) {
        if self.target.is_some() {
            self.flush(gl);
            self.target = None;
        }
    }

    pub fn set_projection(&mut self, gl: &mut GlDevice, view: Rect, flip: bool) {
        let projection = if !flip {
            cgmath::ortho(view.p1.x, view.p2.x, view.p1.y, view.p2.y, -1.0, 1.0)
        } else {
            cgmath::ortho(view.p1.x, view.p2.x, view.p2.y, view.p1.y, -1.0, 1.0)
        };
        let dirty = projection != self.projection;
        if dirty {
            self.flush(gl);
            self.projection = projection;
            self.render_options.front_face = if !flip {
                Some(FrontFace::CounterClockwise)
            } else {
                Some(FrontFace::Clockwise)
            };
        }
    }

    #[allow(unused)]
    pub fn set_shader(&mut self, gl: &mut GlDevice, shader: Rc<gl::Shader>) {
        let dirty = shader.as_ref() != self.shader.as_ref();
        if dirty {
            self.flush(gl);
            self.shader = shader;
        }
    }

    #[allow(unused)]
    pub fn set_target(&mut self, gl: &mut GlDevice, target: Rc<gl::Framebuffer>) {
        let dirty = match self.target.as_ref() {
            Some(fb) => target != *fb,
            None => true,
        };
        if dirty {
            self.flush(gl);
            self.target = Some(target);
        }
    }

    #[allow(unused)]
    pub fn set_texture(&mut self, gl: &mut GlDevice, texture: Rc<gl::Texture>) {
        let dirty = match self.texture.as_ref() {
            Some(tex_ref) => texture != *tex_ref,
            None => true,
        };
        if dirty {
            self.flush(gl);
            self.texture = Some(texture);
        }
    }

    pub fn set_viewport(&mut self, gl: &mut GlDevice, viewport: RectI) {
        let dirty = viewport != self.viewport;
        if dirty {
            self.flush(gl);
            self.viewport = viewport;
        }
    }
}
