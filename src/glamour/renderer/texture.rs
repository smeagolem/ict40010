pub struct Texture {
    id: u32,
}

impl Texture {
    pub fn new(file_path: &std::path::Path) -> Self {
        let image = image::open(file_path).unwrap();
        let image_buf = image.flipv().into_rgb();
        let (width, height) = image_buf.dimensions();
        let mut id = 0;
        gl_call!(gl::GenTextures(1, &mut id));
        gl_call!(gl::BindTexture(gl::TEXTURE_2D, id));
        gl_call!(gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_S,
            gl::REPEAT as gl::types::GLint
        ));
        gl_call!(gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_T,
            gl::REPEAT as gl::types::GLint
        ));
        gl_call!(gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR as gl::types::GLint
        ));
        gl_call!(gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MAG_FILTER,
            gl::LINEAR as gl::types::GLint
        ));
        gl_call!(gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGB as gl::types::GLint,
            width as gl::types::GLint,
            height as gl::types::GLint,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            image_buf.as_ptr() as *const gl::types::GLvoid
        ));
        gl_call!(gl::GenerateMipmap(gl::TEXTURE_2D));
        gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
        // Texture { id, width, height }
        Texture { id }
    }

    pub fn bind(&self) {
        gl_call!(gl::ActiveTexture(gl::TEXTURE0));
        gl_call!(gl::BindTexture(gl::TEXTURE_2D, self.id));
    }
    pub fn unbind(&self) {
        gl_call!(gl::ActiveTexture(gl::TEXTURE0));
        gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        gl_call!(gl::DeleteTextures(1, &self.id));
    }
}
