use crate::error::Result;
use crate::utils;
use nalgebra::{Matrix4, Point3};

use allegro::*;
use allegro_primitives::*;

pub struct GBuffer
{
	pub frame_buffer: u32,
	pub position_tex: u32,
	pub normal_tex: u32,
	pub albedo_tex: u32,
	pub light_tex: u32,
	pub depth_render_buffer: u32,
	rect_vertex_buffer: VertexBuffer<Vertex>,
}

impl GBuffer
{
	pub fn new(
		display: &mut Display, prim: &PrimitivesAddon, buffer_width: i32, buffer_height: i32,
	) -> Result<Self>
	{
		let mut frame_buffer = 0;
		let mut position_tex = 0;
		let mut normal_tex = 0;
		let mut albedo_tex = 0;
		let mut light_tex = 0;
		let mut depth_render_buffer = 0;

		unsafe {
			gl::GenFramebuffers(1, &mut frame_buffer);
			gl::BindFramebuffer(gl::FRAMEBUFFER, frame_buffer);

			gl::GenTextures(1, &mut position_tex);
			gl::BindTexture(gl::TEXTURE_2D, position_tex);
			gl::TexImage2D(
				gl::TEXTURE_2D,
				0,
				gl::RGBA16F as i32,
				buffer_width,
				buffer_height,
				0,
				gl::RGBA,
				gl::FLOAT,
				std::ptr::null(),
			);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
			gl::FramebufferTexture2D(
				gl::FRAMEBUFFER,
				gl::COLOR_ATTACHMENT0,
				gl::TEXTURE_2D,
				position_tex,
				0,
			);

			gl::GenTextures(1, &mut normal_tex);
			gl::BindTexture(gl::TEXTURE_2D, normal_tex);
			gl::TexImage2D(
				gl::TEXTURE_2D,
				0,
				gl::RGBA16F as i32,
				buffer_width,
				buffer_height,
				0,
				gl::RGBA,
				gl::FLOAT,
				std::ptr::null(),
			);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
			gl::FramebufferTexture2D(
				gl::FRAMEBUFFER,
				gl::COLOR_ATTACHMENT1,
				gl::TEXTURE_2D,
				normal_tex,
				0,
			);

			gl::GenTextures(1, &mut albedo_tex);
			gl::BindTexture(gl::TEXTURE_2D, albedo_tex);
			gl::TexImage2D(
				gl::TEXTURE_2D,
				0,
				gl::RGBA as i32,
				buffer_width,
				buffer_height,
				0,
				gl::RGBA,
				gl::UNSIGNED_BYTE,
				std::ptr::null(),
			);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
			gl::FramebufferTexture2D(
				gl::FRAMEBUFFER,
				gl::COLOR_ATTACHMENT2,
				gl::TEXTURE_2D,
				albedo_tex,
				0,
			);

			gl::GenTextures(1, &mut light_tex);
			gl::BindTexture(gl::TEXTURE_2D, light_tex);
			gl::TexImage2D(
				gl::TEXTURE_2D,
				0,
				gl::RGBA16F as i32,
				buffer_width,
				buffer_height,
				0,
				gl::RGBA,
				gl::FLOAT,
				std::ptr::null(),
			);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
			gl::FramebufferTexture2D(
				gl::FRAMEBUFFER,
				gl::COLOR_ATTACHMENT3,
				gl::TEXTURE_2D,
				light_tex,
				0,
			);

			let attachments = [
				gl::COLOR_ATTACHMENT0,
				gl::COLOR_ATTACHMENT1,
				gl::COLOR_ATTACHMENT2,
				gl::COLOR_ATTACHMENT3,
			];
			gl::DrawBuffers(attachments.len() as i32, attachments.as_ptr());
			gl::GenRenderbuffers(1, &mut depth_render_buffer);
			gl::BindRenderbuffer(gl::RENDERBUFFER, depth_render_buffer);
			gl::RenderbufferStorage(
				gl::RENDERBUFFER,
				gl::DEPTH_COMPONENT16,
				buffer_width,
				buffer_height,
			);
			gl::FramebufferRenderbuffer(
				gl::FRAMEBUFFER,
				gl::DEPTH_ATTACHMENT,
				gl::RENDERBUFFER,
				depth_render_buffer,
			);
			if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE
			{
				return Err("Framebuffer not complete".to_string())?;
			}
			gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
		}

		let vtx = Vertex {
			x: 0.0,
			y: 0.0,
			z: 0.0,
			u: 0.0,
			v: 0.0,
			color: Color::from_rgb_f(1., 1., 1.),
		};
		let rect_vertex_buffer = VertexBuffer::new(
			display,
			&prim,
			Some(&[
				Vertex {
					x: 0.0,
					y: 0.0,
					u: 0.0,
					v: 1.0,
					..vtx
				},
				Vertex {
					x: buffer_width as f32,
					y: 0.0,
					u: 1.0,
					v: 1.0,
					..vtx
				},
				Vertex {
					x: buffer_width as f32,
					y: buffer_height as f32,
					u: 1.0,
					v: 0.0,
					..vtx
				},
				Vertex {
					x: 0.0,
					y: buffer_height as f32,
					u: 0.0,
					v: 0.0,
					..vtx
				},
			]),
			4,
			BUFFER_STATIC,
		)
		.unwrap();

		Ok(Self {
			frame_buffer: frame_buffer,
			position_tex: position_tex,
			normal_tex: normal_tex,
			albedo_tex: albedo_tex,
			light_tex: light_tex,
			depth_render_buffer: depth_render_buffer,
			rect_vertex_buffer: rect_vertex_buffer,
		})
	}
}

impl Drop for GBuffer
{
	fn drop(&mut self)
	{
		unsafe {
			gl::DeleteTextures(1, &self.position_tex);
			gl::DeleteTextures(1, &self.normal_tex);
			gl::DeleteTextures(1, &self.albedo_tex);
			gl::DeleteTextures(1, &self.light_tex);
			gl::DeleteRenderbuffers(1, &self.depth_render_buffer);
			gl::DeleteFramebuffers(1, &self.frame_buffer);
		}
	}
}

pub struct DeferredRenderer
{
	pub g_buffer: GBuffer,
	pub width: i32,
	pub height: i32,
}

impl DeferredRenderer
{
	pub fn new(
		display: &mut Display, prim: &PrimitivesAddon, width: i32, height: i32,
	) -> Result<Self>
	{
		Ok(Self {
			g_buffer: GBuffer::new(display, prim, width, height)?,
			width: width,
			height: height,
		})
	}

	pub fn begin_forward_pass(&mut self, core: &Core) -> Result<()>
	{
		unsafe {
			gl::BindFramebuffer(gl::FRAMEBUFFER, self.g_buffer.frame_buffer);
			let attachments = [
				gl::COLOR_ATTACHMENT0,
				gl::COLOR_ATTACHMENT1,
				gl::COLOR_ATTACHMENT2,
				gl::COLOR_ATTACHMENT3,
			];
			gl::DrawBuffers(attachments.len() as i32, attachments.as_ptr());
			gl::Enable(gl::CULL_FACE);
			gl::CullFace(gl::BACK);
			gl::DepthMask(gl::TRUE);
		}
		core.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::Zero);
		core.set_depth_test(Some(DepthFunction::Less));
		core.clear_depth_buffer(1.);
		core.clear_to_color(Color::from_rgba_f(0., 0., 0., 0.));
		Ok(())
	}

	pub fn begin_light_pass(
		&mut self, core: &Core, light_shader: &Shader, projection: &Transform,
		camera_pos: Point3<f32>,
	) -> Result<()>
	{
		unsafe {
			gl::BindFramebuffer(gl::FRAMEBUFFER, self.g_buffer.frame_buffer);
			let attachments = [gl::COLOR_ATTACHMENT3];
			gl::DrawBuffers(attachments.len() as i32, attachments.as_ptr());
		}

		core.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::Zero);
		// Last component is specular.

		core.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::One);
		core.use_projection_transform(projection);

		core.set_depth_test(None);
		unsafe {
			gl::Enable(gl::CULL_FACE);
			gl::DepthMask(gl::FALSE);
			gl::CullFace(gl::FRONT);
		}

		core.use_shader(Some(light_shader)).unwrap();

		core.set_shader_uniform("position_buffer", &[0_i32][..])
			.ok(); //unwrap();

		core.set_shader_uniform("normal_buffer", &[1_i32][..]).ok(); //unwrap();

		core.set_shader_uniform(
			"buffer_size",
			&[[self.width as f32, self.height as f32]][..],
		)
		.ok(); //.unwrap();

		core.set_shader_uniform(
			"camera_pos",
			&[[camera_pos.x, camera_pos.y, camera_pos.z]][..],
		)
		.ok(); //.unwrap();

		unsafe {
			gl::ActiveTexture(gl::TEXTURE0);
			gl::BindTexture(gl::TEXTURE_2D, self.g_buffer.position_tex);
			gl::ActiveTexture(gl::TEXTURE1);
			gl::BindTexture(gl::TEXTURE_2D, self.g_buffer.normal_tex);
		}
		Ok(())
	}

	pub fn final_pass(
		&mut self, core: &Core, prim: &PrimitivesAddon, final_shader: &Shader, buffer: &Bitmap,
	) -> Result<()>
	{
		core.set_target_bitmap(Some(buffer));
		core.clear_to_color(Color::from_rgb_f(0., 0.3, 0.0));
		core.set_depth_test(None);

		core.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::Zero);
		// Copy depth buffer.
		unsafe {
			gl::BindFramebuffer(gl::READ_FRAMEBUFFER, self.g_buffer.frame_buffer);
			gl::BlitFramebuffer(
				0,
				0,
				buffer.get_width() as i32,
				buffer.get_height() as i32,
				0,
				0,
				buffer.get_width() as i32,
				buffer.get_height() as i32,
				gl::DEPTH_BUFFER_BIT,
				gl::NEAREST,
			);
		}

		let ortho_mat = Matrix4::new_orthographic(
			0.,
			buffer.get_width() as f32,
			buffer.get_height() as f32,
			0.,
			-1.,
			1.,
		);

		core.use_projection_transform(&utils::mat4_to_transform(ortho_mat));
		core.use_transform(&Transform::identity());

		core.use_shader(Some(final_shader)).unwrap();

		core.set_shader_uniform("position_buffer", &[1_i32][..])
			.ok();
		core.set_shader_uniform("normal_buffer", &[2_i32][..]).ok();
		core.set_shader_uniform("albedo_buffer", &[3_i32][..]).ok();
		core.set_shader_uniform("light_buffer", &[4_i32][..]).ok();
		unsafe {
			gl::Disable(gl::CULL_FACE);
			gl::ActiveTexture(gl::TEXTURE1);
			gl::BindTexture(gl::TEXTURE_2D, self.g_buffer.position_tex);
			gl::ActiveTexture(gl::TEXTURE2);
			gl::BindTexture(gl::TEXTURE_2D, self.g_buffer.normal_tex);
			gl::ActiveTexture(gl::TEXTURE3);
			gl::BindTexture(gl::TEXTURE_2D, self.g_buffer.albedo_tex);
			gl::ActiveTexture(gl::TEXTURE4);
			gl::BindTexture(gl::TEXTURE_2D, self.g_buffer.light_tex);
		}
		prim.draw_vertex_buffer(
			&self.g_buffer.rect_vertex_buffer,
			Option::<&Bitmap>::None,
			0,
			4,
			PrimType::TriangleFan,
		);

		Ok(())
	}
}
