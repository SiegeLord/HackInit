use crate::error::Result;
use crate::utils;
use allegro::*;
use na::{Point2, Vector2};
use nalgebra as na;
use serde_derive::{Deserialize, Serialize};
use slhack::atlas;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct AnimationDesc
{
	frames: Vec<i32>,
	#[serde(default)]
	frame_ms: Vec<f64>,
	#[serde(default)]
	active_frames: HashMap<String, i32>,
}

fn default_false() -> bool
{
	false
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct SpriteDesc
{
	bitmap: String,
	#[serde(default)]
	width: i32,
	#[serde(default)]
	height: i32,
	#[serde(default)]
	center_offt_x: f32,
	#[serde(default)]
	center_offt_y: f32,
	#[serde(default)]
	animations: HashMap<String, AnimationDesc>,
}

struct Animation
{
	frames: Vec<atlas::AtlasBitmap>,
	duration_ms: f64,
}

pub struct Sprite
{
	desc: SpriteDesc,
	animations: HashMap<String, Animation>,
}

impl Sprite
{
	pub fn load(filename: &str, core: &Core, atlas: &mut atlas::Atlas) -> Result<Self>
	{
		let mut desc: SpriteDesc = utils::load_config(filename)?;

		let bitmap = utils::load_bitmap(&core, &desc.bitmap)?;
		if desc.width == 0
		{
			desc.width = bitmap.get_width();
		}
		if desc.height == 0
		{
			desc.height = bitmap.get_height();
		}

		let num_frames_y = bitmap.get_height() / desc.height;
		let num_frames_x = bitmap.get_width() / desc.width;
		let num_frames = num_frames_x * num_frames_y;
		let mut frames = Vec::with_capacity(num_frames as usize);
		for y in 0..num_frames_y
		{
			for x in 0..num_frames_x
			{
				frames.push(
					atlas.insert(
						&core,
						&*bitmap
							.create_sub_bitmap(
								x * desc.width,
								y * desc.height,
								desc.width,
								desc.height,
							)
							.map_err(|_| "Couldn't create sub-bitmap?".to_string())?
							.upgrade()
							.unwrap(),
					)?,
				)
			}
		}

		if !desc.animations.contains_key("Default")
		{
			desc.animations.insert(
				"Default".to_string(),
				AnimationDesc {
					frames: (0..frames.len()).map(|i| i as i32 + 1).collect(),
					frame_ms: vec![],
					active_frames: HashMap::new(),
				},
			);
		}

		let mut animations = HashMap::new();
		for (name, animation_desc) in &mut desc.animations
		{
			if animation_desc.frame_ms.is_empty()
			{
				animation_desc.frame_ms.push(100.);
			}
			while animation_desc.frame_ms.len() < animation_desc.frames.len()
			{
				animation_desc
					.frame_ms
					.push(*animation_desc.frame_ms.last().unwrap());
			}
			let animation = Animation {
				frames: animation_desc
					.frames
					.iter()
					.map(|&i| frames[(i - 1) as usize].clone())
					.collect(),
				duration_ms: animation_desc.frame_ms.iter().sum(),
			};
			animations.insert(name.to_string(), animation);
		}

		Ok(Sprite {
			desc: desc,
			animations: animations,
		})
	}

	pub fn draw_frame_from_state(
		&self, pos: Point2<f32>, animation_state: &AnimationState, core: &Core,
		atlas: &atlas::Atlas,
	)
	{
		self.draw_frame(
			pos,
			&animation_state.animation_name,
			animation_state.frame_idx,
			core,
			atlas,
		);
	}

	pub fn draw_frame(
		&self, pos: Point2<f32>, animation_name: &str, frame_idx: i32, core: &Core,
		atlas: &atlas::Atlas,
	)
	{
		let w = self.desc.width as f32;
		let h = self.desc.height as f32;
		let animation = &self.animations[animation_name];
		let atlas_bmp = &animation.frames[frame_idx as usize];

		core.draw_bitmap_region(
			&atlas.pages[atlas_bmp.page].bitmap,
			atlas_bmp.start.x,
			atlas_bmp.start.y,
			w,
			h,
			pos.x - w / 2. - self.desc.center_offt_x,
			pos.y - h / 2. - self.desc.center_offt_y,
			Flag::zero(),
		);
	}

	pub fn get_frame_from_state(
		&self, animation_state: &AnimationState,
	) -> (atlas::AtlasBitmap, Vector2<f32>)
	{
		self.get_frame(&animation_state.animation_name, animation_state.frame_idx)
	}

	pub fn get_frame(
		&self, animation_name: &str, frame_idx: i32,
	) -> (atlas::AtlasBitmap, Vector2<f32>)
	{
		let w = self.desc.width as f32;
		let h = self.desc.height as f32;
		let animation = &self.animations[animation_name];
		let atlas_bmp = &animation.frames[frame_idx as usize];

		(
			*atlas_bmp,
			Vector2::new(
				-w / 2. - self.desc.center_offt_x,
				-h / 2. - self.desc.center_offt_y,
			),
		)
	}

	pub fn advance_state(&self, state: &mut AnimationState, amount: f64)
	{
		for (_, frame_idx) in state.num_activations.iter_mut()
		{
			*frame_idx = 0;
		}
		state.num_loops = 0;

		let reset_activations = state.animation_name != state.new_animation_name;
		if reset_activations
		{
			state.animation_name = state.new_animation_name.clone();
			state.frame_idx = 0;
			// TODO: Why aren't we resetting frame_progress?
		}
		let animation_desc = &self
			.desc
			.animations
			.get(&state.animation_name)
			.expect(&format!(
				"Could not find animation '{}'",
				state.animation_name
			));
		if reset_activations
		{
			for (active_frame, _) in animation_desc.active_frames.iter()
			{
				state
					.num_activations
					.entry(active_frame.clone())
					.or_insert(0);
			}
		}

		state.frame_progress += amount * 1000.;
		while state.frame_progress > animation_desc.frame_ms[state.frame_idx as usize]
		{
			state.frame_progress -= animation_desc.frame_ms[state.frame_idx as usize];
			state.frame_idx = (state.frame_idx + 1) % animation_desc.frames.len() as i32;

			for (active_frame, num_activations) in state.num_activations.iter_mut()
			{
				if state.frame_idx == animation_desc.active_frames[active_frame]
				{
					*num_activations += 1;
				}
			}
			if state.frame_idx == animation_desc.frames.len() as i32 - 1
			{
				state.num_loops += 1;
			}
		}
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnimationState
{
	animation_name: String,
	new_animation_name: String,
	frame_progress: f64,
	pub frame_idx: i32,
	pub num_activations: HashMap<String, i32>,
	num_loops: i32,
}

impl AnimationState
{
	pub fn new(animation_name: &str) -> Self
	{
		Self {
			animation_name: animation_name.to_string(),
			new_animation_name: animation_name.to_string(),
			frame_progress: 0.,
			frame_idx: 0,
			num_activations: HashMap::new(),
			num_loops: 0,
		}
	}

	pub fn set_new_animation(&mut self, animation_name: impl Into<String>)
	{
		self.new_animation_name = animation_name.into();
	}

	pub fn get_num_activations(&mut self, active_frame: &str) -> i32
	{
		self.num_activations
			.get(active_frame)
			.map(|v| *v)
			.unwrap_or(0)
	}

	pub fn get_num_loops(&mut self) -> i32
	{
		self.num_loops
	}
}
