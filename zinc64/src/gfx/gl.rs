// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::rc::Rc;

use cgmath::{Matrix4, Vector2};
use glow::{self, HasContext};

use crate::gfx::rect::RectI;
use crate::gfx::Color;

// Design:
//      Thin ergonomic wrapper around GL with resource lifecycle and cleanup managed by rust.
//      RenderState abstraction based on pathfinder/gpu/src/lib.rs

pub struct GlDevice {
    gl: Rc<glow::Context>,
}

impl GlDevice {
    pub fn new(gl: glow::Context) -> Self {
        GlDevice { gl: Rc::new(gl) }
    }

    pub fn create_buffer(
        &mut self,
        ty: BufferType,
        count: usize,
        target: BufferTarget,
        usage: BufferUsage,
    ) -> Result<Buffer, String> {
        unsafe {
            let id = self
                .gl
                .create_buffer()
                .map_err(|_| "failed to create buffer".to_owned())?;
            self.gl.bind_buffer(target.into(), Some(id));
            self.gl.buffer_data_size(
                target.into(),
                (count * ty.element_size()) as i32,
                usage.into(),
            );
            self.gl.bind_buffer(target.into(), None);
            Ok(Buffer {
                gl: self.gl.clone(),
                id,
                ty,
            })
        }
    }

    #[allow(unused)]
    pub fn create_framebuffer(&mut self, texture: Texture) -> Result<Framebuffer, String> {
        unsafe {
            let id = self
                .gl
                .create_framebuffer()
                .map_err(|_| "failed to create framebuffer".to_owned())?;
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(id));
            self.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture.id),
                0,
            );
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            Ok(Framebuffer {
                gl: self.gl.clone(),
                id,
                texture: Rc::new(texture),
            })
        }
    }

    pub fn create_shader(
        &mut self,
        vertex_shader_source: &str,
        fragment_shader_source: &str,
    ) -> Result<Shader, String> {
        unsafe {
            let program_id = self
                .gl
                .create_program()
                .map_err(|_| "failed to create program".to_owned())?;
            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];
            let mut shaders = Vec::with_capacity(shader_sources.len());
            for (shader_type, shader_source) in &shader_sources {
                let shader_id = self
                    .gl
                    .create_shader(*shader_type)
                    .map_err(|_| "failed to create shader".to_owned())?;
                self.gl.shader_source(shader_id, shader_source);
                self.gl.compile_shader(shader_id);
                if !self.gl.get_shader_compile_status(shader_id) {
                    return Err(self.gl.get_shader_info_log(shader_id));
                }
                self.gl.attach_shader(program_id, shader_id);
                shaders.push(shader_id);
            }
            self.gl.link_program(program_id);
            if !self.gl.get_program_link_status(program_id) {
                return Err(self.gl.get_program_info_log(program_id));
            }
            for shader_id in shaders {
                self.gl.detach_shader(program_id, shader_id);
                self.gl.delete_shader(shader_id);
            }
            Ok(Shader {
                gl: self.gl.clone(),
                id: program_id,
            })
        }
    }

    pub fn create_texture(&mut self, size: Vector2<i32>) -> Result<Texture, String> {
        unsafe {
            let id = self
                .gl
                .create_texture()
                .map_err(|_| "failed to create texture".to_owned())?;
            self.gl.bind_texture(glow::TEXTURE_2D, Some(id));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                size.x,
                size.y,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            // Apply parameters
            self.gl
                .tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            self.gl
                .tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            self.gl.bind_texture(glow::TEXTURE_2D, None);
            Ok(Texture {
                gl: self.gl.clone(),
                id,
                size,
            })
        }
    }

    pub fn create_vertex_array(&mut self) -> Result<VertexArray, String> {
        unsafe {
            let id = self
                .gl
                .create_vertex_array()
                .map_err(|_| "failed to create vertex array")?;
            Ok(VertexArray {
                gl: self.gl.clone(),
                id,
            })
        }
    }

    pub fn bind_buffer(
        &mut self,
        vertex_array: &VertexArray,
        buffer: &Buffer,
        target: BufferTarget,
    ) {
        self.bind_vertex_array(Some(vertex_array));
        unsafe {
            self.gl.bind_buffer(target.into(), Some(buffer.id));
        }
        self.bind_vertex_array(None);
    }

    fn bind_framebuffer(&mut self, framebuffer: Option<&Framebuffer>) {
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, framebuffer.map(|v| v.id));
        }
    }

    fn bind_texture(&mut self, texture: Option<&Texture>, unit: u32) {
        unsafe {
            self.gl.active_texture(glow::TEXTURE0 + unit);
            self.gl
                .bind_texture(glow::TEXTURE_2D, texture.map(|v| v.id));
        }
    }

    fn bind_vertex_array(&mut self, vertex_array: Option<&VertexArray>) {
        unsafe {
            self.gl.bind_vertex_array(vertex_array.map(|v| v.id));
        }
    }

    pub fn get_uniform(&self, shader: &Shader, name: &str) -> Uniform {
        unsafe {
            let location = self.gl.get_uniform_location(shader.id, name);
            Uniform { location }
        }
    }

    pub fn set_buffer_data<T: Sized>(&mut self, buffer: &Buffer, target: BufferTarget, data: &[T]) {
        self.set_buffer_sub_data(buffer, target, data)
    }

    fn set_buffer_sub_data<T: Sized>(&mut self, buffer: &Buffer, target: BufferTarget, data: &[T]) {
        unsafe {
            let len = data.len() * core::mem::size_of::<T>();
            let bytes = core::slice::from_raw_parts(data.as_ptr() as *const u8, len);
            self.gl.bind_buffer(target.into(), Some(buffer.id));
            self.gl.buffer_sub_data_u8_slice(target.into(), 0, bytes);
            self.gl.bind_buffer(target.into(), None)
        }
    }

    pub fn set_texture_data(&mut self, texture: &Texture, data: &[u8]) {
        self.set_texture_sub_data(texture, 0, 0, texture.size.x, texture.size.y, data);
    }

    fn set_texture_sub_data(
        &mut self,
        texture: &Texture,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        data: &[u8],
    ) {
        self.bind_texture(Some(texture), 0);
        unsafe {
            // self.gl.tex_sub_image_2d(target, level, x_offset, y_offset, width, height, format, ty, pixels)
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                x,
                y,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(data),
            )
        }
        self.bind_texture(None, 0)
    }

    fn set_uniform(&mut self, uniform: &Uniform, data: &UniformData) {
        unsafe {
            let location = uniform.location.clone();
            match data {
                UniformData::Float(x) => self.gl.uniform_1_f32(location.as_ref(), *x),
                UniformData::Int(x) => self.gl.uniform_1_i32(location.as_ref(), *x),
                UniformData::Mat4(value) => {
                    let value_ref: &[f32; 16] = value.as_ref();
                    self.gl
                        .uniform_matrix_4_f32_slice(location.as_ref(), false, value_ref)
                }
            }
        }
    }

    pub fn set_vertex_attr(
        &mut self,
        vertex_array: &VertexArray,
        location: u32,
        descriptor: &VertexAttrDescriptor,
    ) {
        self.bind_vertex_array(Some(vertex_array));
        unsafe {
            let element_size = descriptor.ty.element_size();
            self.gl.vertex_attrib_pointer_f32(
                location,
                descriptor.size as i32,
                descriptor.ty.into(),
                false,
                (descriptor.stride * element_size) as i32,
                (descriptor.offset * element_size) as i32,
            );
            self.gl.enable_vertex_attrib_array(location);
        }
        self.bind_vertex_array(None);
    }

    fn use_shader(&mut self, shader: &Shader) {
        unsafe {
            self.gl.use_program(Some(shader.id));
        }
    }

    // -- Drawing Ops

    pub fn clear(&mut self, color: Color) {
        unsafe {
            self.gl
                .clear_color(color.r(), color.g(), color.b(), color.a());
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    pub fn draw_elements(&mut self, count: usize, offset: usize, render_state: &RenderState) {
        self.set_render_state(render_state);
        unsafe {
            self.gl.draw_elements(
                glow::TRIANGLES,
                count as i32,
                BufferType::UInt.into(),
                (offset * BufferType::UInt.element_size()) as i32,
            );
        }
        self.reset_render_state(render_state);
    }

    fn set_render_state(&mut self, render_state: &RenderState) {
        match render_state.target {
            RenderTarget::Default => self.bind_framebuffer(None),
            RenderTarget::Framebuffer(fb) => self.bind_framebuffer(Some(fb)),
        }
        unsafe {
            let (origin, size) = (render_state.viewport.origin(), render_state.viewport.size());
            self.gl.viewport(origin.x, origin.y, size.x, size.y);
        }
        self.use_shader(render_state.shader);
        self.bind_vertex_array(Some(render_state.vertex_array));
        for (uniform, data) in render_state.uniforms {
            self.set_uniform(*uniform, data);
        }
        for (unit, texture) in render_state.textures.iter().enumerate() {
            self.bind_texture(Some(*texture), unit as u32);
        }
        unsafe {
            match render_state.options.blend_func {
                None => self.gl.disable(glow::BLEND),
                Some(ref blend_func) => {
                    self.gl.enable(glow::BLEND);
                    match blend_func {
                        BlendFunc::SrcAlphaOneMinusSrcAlpha => {
                            self.gl.blend_func_separate(
                                glow::SRC_ALPHA,
                                glow::ONE_MINUS_SRC_ALPHA,
                                glow::ONE,
                                glow::ONE_MINUS_SRC_ALPHA,
                            );
                        }
                    }
                }
            }
            match render_state.options.front_face {
                None => self.gl.disable(glow::CULL_FACE),
                Some(ref front_face) => {
                    self.gl.enable(glow::CULL_FACE);
                    self.gl.front_face((*front_face).into());
                }
            }
        }
    }

    fn reset_render_state(&mut self, render_state: &RenderState) {
        // FIXME reset rendertarget
        unsafe {
            self.gl.use_program(None);
        }
        self.bind_vertex_array(None);
        for (unit, _texture) in render_state.textures.iter().enumerate() {
            self.bind_texture(None, unit as u32);
        }
        unsafe {
            if render_state.options.blend_func.is_some() {
                self.gl.disable(glow::BLEND);
            }
            if render_state.options.front_face.is_some() {
                self.gl.disable(glow::CULL_FACE);
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum BlendFunc {
    SrcAlphaOneMinusSrcAlpha,
}

#[derive(Copy, Clone)]
pub enum BufferTarget {
    Vertex,
    Index,
}

impl Into<u32> for BufferTarget {
    fn into(self) -> u32 {
        match self {
            BufferTarget::Vertex => glow::ARRAY_BUFFER,
            BufferTarget::Index => glow::ELEMENT_ARRAY_BUFFER,
        }
    }
}

#[allow(unused)]
#[derive(Copy, Clone)]
pub enum BufferType {
    Byte,
    Short,
    UByte,
    UShort,
    UInt,
    Float,
}

impl BufferType {
    pub fn element_size(self) -> usize {
        match self {
            BufferType::Byte => core::mem::size_of::<i8>(),
            BufferType::Short => core::mem::size_of::<i16>(),
            BufferType::UByte => core::mem::size_of::<u8>(),
            BufferType::UShort => core::mem::size_of::<u16>(),
            BufferType::UInt => core::mem::size_of::<u32>(),
            BufferType::Float => core::mem::size_of::<f32>(),
        }
    }
}

impl Into<u32> for BufferType {
    fn into(self) -> u32 {
        match self {
            BufferType::Byte => glow::BYTE,
            BufferType::Short => glow::SHORT,
            BufferType::UByte => glow::UNSIGNED_BYTE,
            BufferType::UShort => glow::UNSIGNED_SHORT,
            BufferType::UInt => glow::UNSIGNED_INT,
            BufferType::Float => glow::FLOAT,
        }
    }
}

#[derive(Copy, Clone)]
pub enum BufferUsage {
    Dynamic,
    Static,
}

impl Into<u32> for BufferUsage {
    fn into(self) -> u32 {
        match self {
            BufferUsage::Dynamic => glow::DYNAMIC_DRAW,
            BufferUsage::Static => glow::STATIC_DRAW,
        }
    }
}

#[derive(Copy, Clone)]
pub enum FrontFace {
    Clockwise,
    CounterClockwise,
}

impl Into<u32> for FrontFace {
    fn into(self) -> u32 {
        match self {
            FrontFace::Clockwise => glow::CW,
            FrontFace::CounterClockwise => glow::CCW,
        }
    }
}

#[derive(Copy, Clone)]
pub struct RenderState<'a> {
    pub target: &'a RenderTarget<'a>,
    pub viewport: RectI,
    pub shader: &'a Shader,
    pub vertex_array: &'a VertexArray,
    pub uniforms: &'a [(&'a Uniform, UniformData)],
    pub textures: &'a [&'a Texture],
    pub options: RenderOptions,
}

#[derive(Copy, Clone)]
pub struct RenderOptions {
    pub blend_func: Option<BlendFunc>,
    pub front_face: Option<FrontFace>,
}

#[derive(Copy, Clone)]
pub enum RenderTarget<'a> {
    Default,
    Framebuffer(&'a Framebuffer),
}

#[derive(Copy, Clone)]
pub struct VertexAttrDescriptor {
    pub ty: BufferType,
    pub size: usize,
    pub stride: usize,
    pub offset: usize,
}

#[derive(Copy, Clone)]
pub struct Uniform {
    location: Option<<glow::Context as HasContext>::UniformLocation>,
}

#[allow(unused)]
#[derive(Clone, Copy)]
pub enum UniformData {
    Float(f32),
    Int(i32),
    Mat4(Matrix4<f32>),
}

// -- Resources

pub struct Buffer {
    gl: Rc<glow::Context>,
    id: <glow::Context as HasContext>::Buffer,
    pub ty: BufferType,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_buffer(self.id);
        }
    }
}

impl PartialEq for Buffer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct Framebuffer {
    gl: Rc<glow::Context>,
    id: <glow::Context as HasContext>::Framebuffer,
    pub texture: Rc<Texture>,
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_framebuffer(self.id);
        }
    }
}

impl PartialEq for Framebuffer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct Shader {
    gl: Rc<glow::Context>,
    id: <glow::Context as HasContext>::Program,
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.id);
        }
    }
}

impl PartialEq for Shader {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct Texture {
    gl: Rc<glow::Context>,
    id: <glow::Context as HasContext>::Texture,
    pub size: Vector2<i32>,
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_texture(self.id);
        }
    }
}

impl PartialEq for Texture {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct VertexArray {
    gl: Rc<glow::Context>,
    id: <glow::Context as HasContext>::VertexArray,
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_vertex_array(self.id);
        }
    }
}

impl PartialEq for VertexArray {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
