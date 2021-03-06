use super::meshes;
use crate::{
    glm, Camera, GBuf, IndexBuf, ShaderBuilder, ShaderProgram, Texture, Transform, VertArray,
    VertBasic, VertBuf, VertTrans,
};
use gl;
use rayon::prelude::*;

#[allow(dead_code)]
pub struct Renderer {
    cube_shader: ShaderProgram,
    cube_vao: VertArray,
    cube_vbo: VertBuf<VertBasic>,
    cube_trans_vbo: VertBuf<VertTrans>,
    cube_tex: Texture,
    light_shader: ShaderProgram,
    light_vao: VertArray,
    light_vbo: VertBuf<VertBasic>,
    light_trans_vbo: VertBuf<VertTrans>,
    g_buf: GBuf,
    lit_def_geo: ShaderProgram,
    lit_def_light: ShaderProgram,
    ndc_quad_vbo: VertBuf<VertBasic>,
    ndc_quad_vao: VertArray,
    deferred: bool,
}

impl Renderer {
    pub fn new(resolution: (u32, u32), max_cubes: usize, max_lights: usize) -> Self {
        gl_call!(gl::Enable(gl::DEPTH_TEST));

        let cube_shader = ShaderBuilder::new(
            include_str!("shaders/lit_for.vert"),
            include_str!("shaders/lit_for.frag"),
        )
        .with_float4("u_color", glm::vec4(1.0, 1.0, 1.0, 1.0))
        .build();
        let img_path = crate::assets_path().join("tile_bookcaseFull.png");
        let cube_tex = Texture::new(&img_path);
        // TODO: check in draw functions if overflowing buffer, if so, draw (flush and reset).
        let cube_vbo = VertBuf::<VertBasic>::new(meshes::cube::tex_cube_verts());
        let cube_trans_vbo = VertBuf::<VertTrans>::new(Vec::with_capacity(max_cubes));
        let ibo = IndexBuf::new(meshes::cube::tex_cube_inds());
        let cube_vao = VertArray::new(&[&cube_vbo, &cube_trans_vbo], ibo);

        let light_shader = ShaderBuilder::new(
            include_str!("shaders/unlit_for.vert"),
            include_str!("shaders/unlit_for.frag"),
        )
        .with_float4("u_color", glm::vec4(1.0, 1.0, 1.0, 1.0))
        .build();
        // TODO: check in draw functions if overflowing buffer, if so, draw (flush and reset).
        let light_vbo = VertBuf::<VertBasic>::new(meshes::cube::tex_cube_verts());
        let light_trans_vbo = VertBuf::<VertTrans>::new(Vec::with_capacity(max_lights));
        let ibo = IndexBuf::new(meshes::cube::tex_cube_inds());
        let light_vao = VertArray::new(&[&light_vbo, &light_trans_vbo], ibo);

        let lit_def_geo = ShaderBuilder::new(
            include_str!("shaders/lit_def_geo.vert"),
            include_str!("shaders/lit_def_geo.frag"),
        )
        .with_float4("u_color", glm::vec4(1.0, 1.0, 1.0, 1.0))
        .build();

        let lit_def_light = ShaderBuilder::new(
            include_str!("shaders/lit_def_light.vert"),
            include_str!("shaders/lit_def_light.frag"),
        )
        .build();
        lit_def_light.set_int("u_tex_pos", 0);
        lit_def_light.set_int("u_tex_norm", 1);
        lit_def_light.set_int("u_tex_alb_spec", 2);

        let ndc_quad_vbo = VertBuf::new(meshes::quad::ndc_quad_verts());
        let ndc_quad_vao = VertArray::new(&[&ndc_quad_vbo], IndexBuf::new(vec![]));

        Renderer {
            cube_shader,
            cube_vao,
            cube_vbo,
            cube_trans_vbo,
            cube_tex,
            light_shader,
            light_vao,
            light_vbo,
            light_trans_vbo,
            g_buf: GBuf::new(resolution.0, resolution.1),
            lit_def_geo,
            lit_def_light,
            ndc_quad_vbo,
            ndc_quad_vao,
            deferred: false,
        }
    }

    pub fn deferred(&self) -> bool {
        self.deferred
    }

    pub fn set_deferred(&mut self, val: bool) {
        self.deferred = val;
    }

    pub fn cube_shader(&self) -> &ShaderProgram {
        &self.cube_shader
    }

    pub fn clear(&self) {
        // gl_call!(gl::ClearColor(
        //     20.0 / 255.0,
        //     24.0 / 255.0,
        //     82.0 / 255.0,
        //     1.0
        // ));
        gl_call!(gl::ClearColor(0.0, 0.0, 0.0, 1.0));
        gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.g_buf.resize(width, height)
    }

    pub fn handle_event(&mut self, event: &glutin::event::Event<()>) {
        match event {
            glutin::event::Event::WindowEvent {
                event: glutin::event::WindowEvent::Resized(physical_size),
                ..
            } => self.resize(physical_size.width, physical_size.height),
            _ => (),
        }
    }

    pub fn begin_draw(&self, camera: &Camera) {
        let vp_mat = camera.view_projection_matrix();

        self.light_shader.set_mat4("u_view_projection", &vp_mat);

        self.cube_shader.set_mat4("u_view_projection", &vp_mat);
        self.cube_shader.set_float3("u_view_pos", &camera.position);

        self.lit_def_geo.set_mat4("u_view_projection", &vp_mat);

        self.lit_def_light
            .set_float3("u_view_pos", &camera.position);
    }

    pub fn end_draw(&mut self) {
        self.clear();
        if self.deferred {
            self.draw_cubes_def();
        } else {
            self.draw_cubes();
        }
        self.draw_lights();
    }

    pub fn set_vert_trans(vertices: &mut Vec<VertTrans>, transforms: &[Transform]) {
        vertices.resize_with(transforms.len(), std::default::Default::default);
        vertices
            .par_iter_mut()
            .zip(transforms.par_iter())
            .for_each(|(v, t)| v.set(t));
    }

    // ~25% slower
    #[deprecated]
    pub fn set_vert_trans2(vertices: &mut Vec<VertTrans>, transforms: &[Transform]) {
        vertices.clear();
        vertices.append(
            &mut transforms
                .par_iter()
                .map(VertTrans::from_transform)
                .collect(),
        );
    }

    pub fn set_cubes(&mut self, transforms: &[Transform]) {
        let vertices = self.cube_trans_vbo.vertices_mut();
        Renderer::set_vert_trans(vertices, transforms);
    }

    pub fn set_lights(&mut self, transforms: &[Transform]) {
        let vertices = self.light_trans_vbo.vertices_mut();
        Renderer::set_vert_trans(vertices, transforms);
        transforms.iter().enumerate().for_each(|(i, t)| {
            let name = format!("u_point_lights[{}].position", i);
            self.cube_shader.set_float3(&name, &t.position);
            self.lit_def_light.set_float3(&name, &t.position);
        });
        self.cube_shader
            .set_int("u_point_lights_count", transforms.len() as i32);
        self.lit_def_light
            .set_int("u_point_lights_count", transforms.len() as i32);
    }

    fn draw_cubes(&self) {
        self.cube_trans_vbo.set_data();
        self.cube_shader.bind();
        self.cube_tex.bind();
        self.cube_vao.bind();
        gl_call!(gl::DrawElementsInstanced(
            gl::TRIANGLES,
            self.cube_vao.index_buf().len() as i32,
            gl::UNSIGNED_INT,
            std::ptr::null(),
            self.cube_trans_vbo.vertices().len() as i32,
        ));
        self.cube_vao.unbind();
        self.cube_tex.unbind();
        self.cube_shader.unbind();
    }

    fn draw_lights(&self) {
        self.light_trans_vbo.set_data();
        self.light_shader.bind();
        self.light_vao.bind();
        gl_call!(gl::DrawElementsInstanced(
            gl::TRIANGLES,
            self.light_vao.index_buf().len() as i32,
            gl::UNSIGNED_INT,
            std::ptr::null(),
            self.light_trans_vbo.vertices().len() as i32,
        ));
        self.light_vao.unbind();
        self.light_shader.unbind();
    }

    fn draw_cubes_def(&mut self) {
        self.cube_trans_vbo.set_data();

        // goemetry pass (must be cleared black beforehand)
        self.g_buf.bind();
        {
            gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
            self.lit_def_geo.bind();
            self.cube_tex.bind();
            self.cube_vao.bind();
            gl_call!(gl::DrawElementsInstanced(
                gl::TRIANGLES,
                self.cube_vao.index_buf().len() as i32,
                gl::UNSIGNED_INT,
                std::ptr::null(),
                self.cube_trans_vbo.vertices().len() as i32,
            ));
            self.cube_vao.unbind();
            self.cube_tex.unbind();
            self.lit_def_geo.unbind();
        }
        self.g_buf.unbind();

        // lighting pass
        gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
        {
            self.lit_def_light.bind();
            self.g_buf.bind_bufs();
            self.ndc_quad_vao.bind();
            gl_call!(gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4));
            self.ndc_quad_vao.unbind();
            self.g_buf.unbind_bufs();
            self.lit_def_light.unbind();
        }

        // blit depth buffer
        self.g_buf.blit_depth();
    }

    pub fn draw_quad(&mut self, _transform: &Transform) {
        todo!()
    }

    pub fn draw_triangle(&mut self, _transform: &Transform) {
        todo!()
    }
}
