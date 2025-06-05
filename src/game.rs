use crate::error::Result;
use crate::utils::DT;
use crate::{astar, components as comps, controls, game_state, mesh, sprite, ui, utils};
use allegro::*;
use allegro_font::*;
use na::{
	Isometry3, Matrix4, Perspective3, Point2, Point3, Quaternion, RealField, Rotation2, Rotation3,
	Similarity3, Unit, Vector2, Vector3, Vector4,
};
use nalgebra as na;
use rand::prelude::*;

use std::collections::HashMap;
use std::f32::consts::PI;

pub struct Game
{
	map: Map,
	subscreens: ui::SubScreens,
}

impl Game
{
	pub fn new(state: &mut game_state::GameState) -> Result<Self>
	{
		Ok(Self {
			map: Map::new(state)?,
			subscreens: ui::SubScreens::new(state),
		})
	}

	pub fn logic(
		&mut self, state: &mut game_state::GameState,
	) -> Result<Option<game_state::NextScreen>>
	{
		if self.subscreens.is_empty()
		{
			self.map.logic(state)
		}
		else
		{
			Ok(None)
		}
	}

	pub fn input(
		&mut self, event: &Event, state: &mut game_state::GameState,
	) -> Result<Option<game_state::NextScreen>>
	{
		match *event
		{
			Event::MouseAxes { x, y, .. } =>
			{
				if state.track_mouse
				{
					let (x, y) = state.transform_mouse(x as f32, y as f32);
					state.mouse_pos = Point2::new(x as i32, y as i32);
				}
			}
			_ => (),
		}
		if self.subscreens.is_empty()
		{
			let mut in_game_menu = false;
			let handled = false; // In case there's other in-game UI to handle this.
			if state
				.game_ui_controls
				.get_action_state(controls::Action::UICancel)
				> 0.5
			{
				in_game_menu = true;
			}
			else if !handled
			{
				let res = self.map.input(event, state);
				if let Ok(Some(game_state::NextScreen::InGameMenu)) = res
				{
					in_game_menu = true;
				}
				else
				{
					return res;
				}
			}
			if in_game_menu
			{
				self.subscreens
					.push(ui::SubScreen::InGameMenu(ui::InGameMenu::new(state)));
				self.subscreens.reset_transition(state);
			}
		}
		else
		{
			if let Some(action) = self.subscreens.input(state, event)?
			{
				match action
				{
					ui::Action::MainMenu =>
					{
						return Ok(Some(game_state::NextScreen::Menu));
					}
					_ => (),
				}
			}
			if self.subscreens.is_empty()
			{
				state.controls.clear_action_states();
			}
		}
		Ok(None)
	}

	pub fn draw(&mut self, state: &mut game_state::GameState) -> Result<()>
	{
		if !self.subscreens.is_empty()
		{
			state.core.clear_to_color(Color::from_rgb_f(0.0, 0.0, 0.0));
			self.subscreens.draw(state);
		}
		else
		{
			self.map.draw(state)?;
		}
		Ok(())
	}

	pub fn resize(&mut self, state: &game_state::GameState)
	{
		self.subscreens.resize(state);
	}
}

pub fn spawn_obj(pos: Point2<f32>, world: &mut hecs::World) -> Result<hecs::Entity>
{
	let entity = world.spawn((comps::Position::new(pos),));
	Ok(entity)
}

struct Map
{
	world: hecs::World,
}

impl Map
{
	fn new(state: &mut game_state::GameState) -> Result<Self>
	{
		let mut world = hecs::World::new();
		spawn_obj(Point2::new(100., 100.), &mut world)?;
		game_state::cache_mesh(state, "data/test_level_sprytile.glb")?;
		game_state::cache_mesh(state, "data/sphere.glb")?;

		Ok(Self { world: world })
	}

	fn logic(&mut self, state: &mut game_state::GameState)
		-> Result<Option<game_state::NextScreen>>
	{
		let mut to_die = vec![];

		// Position snapshotting.
		for (_, position) in self.world.query::<&mut comps::Position>().iter()
		{
			position.snapshot();
		}

		// Input.
		if state.controls.get_action_state(controls::Action::Move) > 0.5
		{
			for (_, position) in self.world.query::<&mut comps::Position>().iter()
			{
				position.pos.y += 100. * DT;
			}
		}

		// Movement.
		for (_, position) in self.world.query::<&mut comps::Position>().iter()
		{
			position.pos.x += 1500. * DT;
			if position.pos.x > state.buffer_width()
			{
				position.pos.x %= state.buffer_width();
				position.snapshot();
			}
		}

		// Remove dead entities
		to_die.sort();
		to_die.dedup();
		for id in to_die
		{
			//println!("died {id:?}");
			self.world.despawn(id)?;
		}

		Ok(None)
	}

	fn input(
		&mut self, _event: &Event, _state: &mut game_state::GameState,
	) -> Result<Option<game_state::NextScreen>>
	{
		Ok(None)
	}

	fn make_project(&self, state: &game_state::GameState) -> Perspective3<f32>
	{
		utils::projection_transform(state.buffer_width(), state.buffer_height(), PI / 2.)
	}

	fn camera_pos(&self) -> Point3<f32>
	{
		Point3::new(5., 2., -2.)
	}

	fn make_camera(&self) -> Isometry3<f32>
	{
		utils::make_camera(self.camera_pos(), Point3::new(0., 0., 0.))
	}

	fn draw(&mut self, state: &mut game_state::GameState) -> Result<()>
	{
		let project = self.make_project(state);
		let camera = self.make_camera();

		// Forward pass.
		state
			.core
			.use_projection_transform(&utils::mat4_to_transform(project.to_homogeneous()));
		state
			.core
			.use_transform(&utils::mat4_to_transform(camera.to_homogeneous()));
		state
			.deferred_renderer
			.as_mut()
			.unwrap()
			.begin_forward_pass(&state.core)?;
		state
			.core
			.use_shader(Some(&*state.forward_shader.upgrade().unwrap()))
			.unwrap();

		let shift = Isometry3::new(Vector3::zeros(), Vector3::zeros()).to_homogeneous();

		state
			.core
			.use_transform(&utils::mat4_to_transform(camera.to_homogeneous() * shift));
		state
			.core
			.set_shader_transform("model_matrix", &utils::mat4_to_transform(shift))
			.ok();

		let material_mapper = |_material: &mesh::Material, texture_name: &str| -> Result<&Bitmap> {
			state.get_bitmap(texture_name)
		};

		state
			.get_mesh("data/test_level_sprytile.glb")
			.unwrap()
			.draw(&state.core, &state.prim, material_mapper);

		// Light pass.
		state.deferred_renderer.as_mut().unwrap().begin_light_pass(
			&state.core,
			state.light_shader.clone(),
			&utils::mat4_to_transform(project.to_homogeneous()),
			self.camera_pos(),
		)?;

		let shift = Isometry3::new(Vector3::new(2., 2., -0.5), Vector3::zeros());
		let intensity = 100.0_f32;
		let transform = Similarity3::from_isometry(shift, 0.5 * intensity.sqrt());
		let light_pos = transform.transform_point(&Point3::origin());

		let (r, g, b) = (1., 0.1, 1.);

		state
			.core
			.set_shader_uniform("light_color", &[[r, g, b, 1.0]][..])
			.ok(); //.unwrap();
		state
			.core
			.set_shader_uniform("light_pos", &[[light_pos.x, light_pos.y, light_pos.z]][..])
			.ok(); //.unwrap();
		state
			.core
			.set_shader_uniform("light_intensity", &[intensity][..])
			.ok(); //.unwrap();

		state.core.use_transform(&utils::mat4_to_transform(
			camera.to_homogeneous() * transform.to_homogeneous(),
		));

		if let Ok(mesh) = state.get_mesh("data/sphere.glb")
		{
			mesh.draw(&state.core, &state.prim, |_, s| state.get_bitmap(s));
		}

		// Final pass.
		state.deferred_renderer.as_mut().unwrap().final_pass(
			&state.core,
			&state.prim,
			state.final_shader.clone(),
			state.buffer1.as_ref().unwrap(),
		)?;

		state
			.core
			.use_shader(Some(&*state.basic_shader.upgrade().unwrap()))
			.unwrap();
		unsafe {
			gl::Disable(gl::CULL_FACE);
		}
		state.core.set_depth_test(None);
		state
			.core
			.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::InverseAlpha);
		Ok(())
	}
}
