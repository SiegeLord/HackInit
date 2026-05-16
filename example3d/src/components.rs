use allegro::*;
use nalgebra::{Point3, UnitQuaternion, Vector3};
use rand::prelude::*;
use slhack::sprite;

#[derive(Debug, Copy, Clone)]
pub struct Light
{
	pub color: Color,
	pub intensity: f32,
	pub static_: bool,
}

#[derive(Debug, Copy, Clone)]
pub struct Position
{
	pub pos: Point3<f32>,
	old_pos: Point3<f32>,

	pub rot: UnitQuaternion<f32>,
	old_rot: UnitQuaternion<f32>,

	pub scale: Vector3<f32>,
	old_scale: Vector3<f32>,
}

impl Position
{
	pub fn new(pos: Point3<f32>) -> Self
	{
		let rot = UnitQuaternion::identity();
		let scale = Vector3::new(1., 1., 1.);
		Self {
			pos: pos,
			old_pos: pos,
			rot: rot,
			old_rot: rot,
			scale: scale,
			old_scale: scale,
		}
	}

	pub fn set_rot(&mut self, rot: UnitQuaternion<f32>) -> &mut Self
	{
		self.rot = rot;
		self.old_rot = rot;
		self
	}

	pub fn set_scale(&mut self, scale: Vector3<f32>) -> &mut Self
	{
		self.scale = scale;
		self.old_scale = scale;
		self
	}

	pub fn snapshot(&mut self)
	{
		self.old_pos = self.pos;
		self.old_rot = self.rot;
		self.old_scale = self.scale;
	}

	pub fn draw_pos(&self, alpha: f32) -> Point3<f32>
	{
		self.pos + alpha * (self.pos - self.old_pos)
	}

	pub fn draw_rot(&self, alpha: f32) -> UnitQuaternion<f32>
	{
		self.old_rot.slerp(&self.rot, alpha)
	}

	pub fn draw_scale(&self, alpha: f32) -> Vector3<f32>
	{
		self.scale + alpha * (self.scale - self.old_scale)
	}
}

#[derive(Debug, Clone)]
pub struct Scene
{
	pub scene: String,
	pub color: Color,
}

impl Scene
{
	pub fn new(scene: &str) -> Self
	{
		Self {
			scene: scene.to_string(),
			color: Color::from_rgb_f(1., 1., 1.),
		}
	}

	pub fn set_color(&mut self, color: Color) -> &mut Self
	{
		self.color = color;
		self
	}
}
