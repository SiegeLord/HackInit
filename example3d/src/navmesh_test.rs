use crate::error::{Result, ResultHelper};
use crate::game_state::DT;
use crate::{components as comps, game_state, ui, utils};
use allegro::*;
use allegro_font::*;
use na::{
	Isometry3, Matrix4, Perspective3, Point2, Point3, RealField, Rotation2, Rotation3, Similarity3,
	Transform3, UnitQuaternion, Vector2, Vector3, Vector4,
};
use nalgebra as na;
use rand::prelude::*;
use rapier3d::dynamics::{
	CCDSolver, FixedJointBuilder, ImpulseJointHandle, ImpulseJointSet, IntegrationParameters,
	IslandManager, MassProperties, MultibodyJointSet, RigidBody, RigidBodyBuilder, RigidBodyHandle,
	RigidBodySet, SpringJointBuilder,
};
use rapier3d::geometry::{
	Ball, ColliderBuilder, ColliderHandle, ColliderSet, CollisionEvent, ContactPair,
	DefaultBroadPhase, Group, InteractionGroups, InteractionTestMode, NarrowPhase, Ray,
	SharedShape, TriMeshFlags,
};
use rapier3d::pipeline::{ActiveEvents, EventHandler, PhysicsPipeline, QueryFilter, QueryPipeline};
use slhack::{controls, scene, sprite, ui as slhack_ui};

use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::RwLock;

const BIG_GROUP: Group = Group::GROUP_1;

pub struct NavMeshTest
{
	map: Map,
	subscreens: ui::SubScreens,
}

impl NavMeshTest
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
		if self.subscreens.is_empty()
		{
			let mut in_game_menu = false;
			let handled = false; // In case there's other in-game UI to handle this.
			if state
				.hs
				.game_ui_controls
				.get_action_state(slhack_ui::UIAction::Cancel)
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
			state
				.hs
				.core
				.clear_to_color(Color::from_rgb_f(0.0, 0.0, 0.0));
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

pub fn spawn_obj(pos: Point3<f32>, world: &mut hecs::World) -> Result<hecs::Entity>
{
	let entity = world.spawn((comps::Position::new(pos),));
	Ok(entity)
}

pub fn spawn_light(
	pos: Point3<f32>, light: comps::Light, world: &mut hecs::World,
) -> Result<hecs::Entity>
{
	let entity = world.spawn((comps::Position::new(pos), light));
	Ok(entity)
}

fn meshes_to_trimesh(
	meshes: &[scene::Mesh<game_state::MaterialKind>], pos: Point3<f32>, rot: UnitQuaternion<f32>,
	scale: Vector3<f32>,
) -> (Vec<Point3<f32>>, Vec<[u32; 3]>)
{
	let mut vertices = vec![];
	let mut indices = vec![];
	let mut index_offset = 0;

	let shift = Isometry3 {
		translation: pos.coords.into(),
		rotation: rot.into(),
	}
	.to_homogeneous();
	let scale = Matrix4::new_nonuniform_scaling(&scale);
	let transform = Transform3::from_matrix_unchecked(shift * scale);

	for mesh in meshes
	{
		for vtx in &mesh.vtxs
		{
			vertices.push(transform * Point3::new(vtx.x, vtx.y, vtx.z));
		}
		for idxs in mesh.idxs.chunks(3)
		{
			indices.push([
				idxs[0] as u32 + index_offset,
				idxs[1] as u32 + index_offset,
				idxs[2] as u32 + index_offset,
			]);
		}
		index_offset += mesh.vtxs.len() as u32;
	}
	(vertices, indices)
}

fn get_collision_trimeshes(
	scene: &scene::Scene<game_state::MaterialKind>,
) -> Vec<(Vec<Point3<f32>>, Vec<[u32; 3]>)>
{
	let mut trimeshes = vec![];
	for object in &scene.objects
	{
		match &object.kind
		{
			scene::ObjectKind::CollisionMesh { meshes } =>
			{
				trimeshes.push(meshes_to_trimesh(
					&meshes[..],
					object.pos,
					object.rot,
					object.scale,
				));
			}
			_ => (),
		}
	}
	if !trimeshes.is_empty()
	{
		return trimeshes;
	}
	for object in &scene.objects
	{
		match &object.kind
		{
			scene::ObjectKind::MultiMesh { meshes } =>
			{
				trimeshes.push(meshes_to_trimesh(
					&meshes[..],
					object.pos,
					object.rot,
					object.scale,
				));
			}
			_ => (),
		}
	}
	trimeshes
}

pub struct PhysicsEventHandler
{
	collision_events: RwLock<Vec<(CollisionEvent, Option<ContactPair>)>>,
	contact_force_events: RwLock<Vec<(f32, ContactPair)>>,
}

impl PhysicsEventHandler
{
	pub fn new() -> Self
	{
		Self {
			collision_events: RwLock::new(vec![]),
			contact_force_events: RwLock::new(vec![]),
		}
	}
}

impl EventHandler for PhysicsEventHandler
{
	fn handle_collision_event(
		&self, _bodies: &RigidBodySet, _colliders: &ColliderSet, event: CollisionEvent,
		contact_pair: Option<&ContactPair>,
	)
	{
		let mut events = self.collision_events.write().unwrap();
		events.push((event, contact_pair.cloned()));
	}

	fn handle_contact_force_event(
		&self, _dt: f32, _bodies: &RigidBodySet, _colliders: &ColliderSet,
		contact_pair: &ContactPair, total_force_magnitude: f32,
	)
	{
		self.contact_force_events
			.write()
			.unwrap()
			.push((total_force_magnitude, contact_pair.clone()));
	}
}

pub struct Physics
{
	rigid_body_set: RigidBodySet,
	collider_set: ColliderSet,
	integration_parameters: IntegrationParameters,
	physics_pipeline: PhysicsPipeline,
	island_manager: IslandManager,
	broad_phase: DefaultBroadPhase,
	narrow_phase: NarrowPhase,
	impulse_joint_set: ImpulseJointSet,
	multibody_joint_set: MultibodyJointSet,
	ccd_solver: CCDSolver,
}

impl Physics
{
	fn new() -> Self
	{
		Self {
			rigid_body_set: RigidBodySet::new(),
			collider_set: ColliderSet::new(),
			integration_parameters: IntegrationParameters {
				dt: game_state::DT,
				num_solver_iterations: 10,
				//num_internal_pgs_iterations: 10,
				//contact_damping_ratio: 0.01,
				//contact_natural_frequency: 60.,
				//normalized_max_corrective_velocity: 100.,
				//joint_damping_ratio: 0.01,
				//joint_natural_frequency: 1e8,
				..IntegrationParameters::default()
			},
			physics_pipeline: PhysicsPipeline::new(),
			island_manager: IslandManager::new(),
			broad_phase: DefaultBroadPhase::new(),
			narrow_phase: NarrowPhase::new(),
			impulse_joint_set: ImpulseJointSet::new(),
			multibody_joint_set: MultibodyJointSet::new(),
			ccd_solver: CCDSolver::new(),
		}
	}

	fn step(&mut self, event_handler: &PhysicsEventHandler)
	{
		let gravity = Vector3::zeros();
		self.physics_pipeline.step(
			&gravity,
			&self.integration_parameters,
			&mut self.island_manager,
			&mut self.broad_phase,
			&mut self.narrow_phase,
			&mut self.rigid_body_set,
			&mut self.collider_set,
			&mut self.impulse_joint_set,
			&mut self.multibody_joint_set,
			&mut self.ccd_solver,
			&(),
			event_handler,
		);
	}

	fn make_query_pipeline(&self, source: Option<RigidBodyHandle>) -> QueryPipeline<'_>
	{
		let mut query_filter = QueryFilter::default();
		if let Some(source) = source
		{
			query_filter = query_filter.exclude_rigid_body(source);
		}
		self.broad_phase.as_query_pipeline(
			self.narrow_phase.query_dispatcher(),
			&self.rigid_body_set,
			&self.collider_set,
			query_filter,
		)
	}

	fn ray_cast(
		&self, source: Option<RigidBodyHandle>, pos: Point3<f32>, dir: Vector3<f32>, range: f32,
	) -> Option<(ColliderHandle, f32)>
	{
		let ray = Ray::new(pos, dir);
		self.make_query_pipeline(source).cast_ray(&ray, range, true)
	}

	fn ball_query(
		&self, source: Option<RigidBodyHandle>, pos: Point3<f32>, radius: f32,
	) -> Vec<ColliderHandle>
	{
		let ball = Ball::new(radius);
		self.make_query_pipeline(source)
			.intersect_shape(Isometry3::translation(pos.x, pos.y, pos.z), &ball)
			.map(|(c, _)| c)
			.collect()
	}
}

struct Map
{
	world: hecs::World,
	physics: Physics,
	level_handle: rapier3d::dynamics::RigidBodyHandle,

	navmesh: scene::NavMesh,
	camera_target: Point3<f32>,
	camera_elev: f32,
	camera_azim: f32,
	camera_radius: f32,

	source: hecs::Entity,
	target: hecs::Entity,
}

impl Map
{
	fn new(state: &mut game_state::GameState) -> Result<Self>
	{
		let mut world = hecs::World::new();
		let mut physics = Physics::new();

		spawn_obj(Point3::new(0., 0., 0.), &mut world)?;
		game_state::cache_scene(state, "data/navmesh_test.glb")?;
		state.cache_bitmap("data/level_lightmap.png")?;
		game_state::cache_scene(state, "data/sphere.glb")?;
		game_state::cache_scene(state, "data/cube.glb")?;
		game_state::cache_scene(state, "data/test.obj")?;

		let level_scene = state.get_scene("data/navmesh_test.glb").unwrap();
		let mut navmesh = None;
		for object in &level_scene.objects
		{
			match &object.kind
			{
				scene::ObjectKind::Light { color, intensity } =>
				{
					spawn_light(
						object.pos,
						comps::Light {
							color: *color,
							intensity: intensity / 50.,
							static_: true,
						},
						&mut world,
					)?;
				}
				scene::ObjectKind::NavMesh { navmesh: n } =>
				{
					navmesh = Some(n.clone());
				}
				_ => (),
			}
		}

		let rigid_body = RigidBodyBuilder::fixed().build();
		let level_handle = physics.rigid_body_set.insert(rigid_body);

		for (vertices, indices) in get_collision_trimeshes(level_scene)
		{
			let collider = ColliderBuilder::trimesh(vertices, indices)?.build();
			physics.collider_set.insert_with_parent(
				collider,
				level_handle,
				&mut physics.rigid_body_set,
			);
		}

		let mut handler = PhysicsEventHandler::new();
		physics.step(&mut handler);

		let navmesh = navmesh.unwrap();

		let navmesh_scene = state
			.with_scene("data/navmesh_test.glb", |scene, state| {
				Ok(scene.create_mesh_from_navmesh(
					state.hs.display.as_mut().unwrap(),
					&state.hs.prim,
					Some(&scene::Material {
						name: "navmesh_material".to_string(),
						desc: utils::load_config("data/navmesh_material.cfg")?,
					}),
				)?)
			})?
			.unwrap();
		state.insert_scene("navmesh_scene", navmesh_scene);

		world.spawn((
			comps::Position::new(Point3::origin()),
			comps::AdditiveScene::new("navmesh_scene")
				.set_color(Color::from_rgb_f(0., 0.5, 0.))
				.clone(),
		));

		for node in &navmesh.nodes
		{
			world.spawn((
				comps::Position::new(node.pos).with_scale(Vector3::from_element(0.5)),
				comps::Scene::new("data/sphere.glb").set_color(Color::from_rgb_f(0., 0., 1.)),
			));

			for neighbour in &node.neighbours
			{
				let start = node.pos;
				let end = navmesh.nodes[*neighbour as usize].pos;

				let center = start + (end - start) * 0.25;
				let length = (end - start).norm() / 2.;
				let rot = UnitQuaternion::face_towards(&(end - start), &Vector3::y_axis());

				world.spawn((
					comps::Position::new(center)
						.with_scale(Vector3::new(0.25, 0.25, length))
						.with_rot(rot),
					comps::Scene::new("data/cube.glb").set_color(Color::from_rgb_f(0., 1., 1.)),
				));
			}
		}

		world.spawn((
			comps::Position::new(Point3::new(2.5, 1.5, -1.)),
			comps::Scene::new("data/sphere.glb"),
		));

		world.spawn((
			comps::Position::new(Point3::new(5.5, 1.5, -2.)),
			comps::Scene::new("data/test.obj"),
		));

		let source = world.spawn((
			comps::Position::new(Point3::new(3.5, 1.5, -1.)),
			comps::Scene::new("data/sphere.glb").set_color(Color::from_rgb_f(1., 0., 0.)),
		));

		let target = world.spawn((
			comps::Position::new(Point3::new(4.5, 1.5, -1.)),
			comps::Scene::new("data/sphere.glb").set_color(Color::from_rgb_f(0., 0., 1.)),
		));

		Ok(Self {
			world: world,
			physics: physics,
			level_handle: level_handle,
			camera_target: Point3::new(3., 2., -3.),
			camera_elev: 0.,
			camera_azim: 0.,
			camera_radius: 1.,
			navmesh: navmesh,
			source: source,
			target: target,
		})
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
		let want_left = state
			.controls
			.get_action_state(game_state::Action::RotateViewLeft);
		let want_right = state
			.controls
			.get_action_state(game_state::Action::RotateViewRight);
		let want_up = state
			.controls
			.get_action_state(game_state::Action::RotateViewUp);
		let want_down = state
			.controls
			.get_action_state(game_state::Action::RotateViewDown);

		let want_move_left = state
			.controls
			.get_action_state(game_state::Action::MoveViewLeft);
		let want_move_right = state
			.controls
			.get_action_state(game_state::Action::MoveViewRight);
		let want_move_forward = state
			.controls
			.get_action_state(game_state::Action::MoveViewForward);
		let want_move_back = state
			.controls
			.get_action_state(game_state::Action::MoveViewBackward);
		let want_move_up = state
			.controls
			.get_action_state(game_state::Action::MoveViewUp);
		let want_move_down = state
			.controls
			.get_action_state(game_state::Action::MoveViewDown);

		let want_zoom_in = state.controls.get_action_state(game_state::Action::ZoomIn) > 0.5;
		let want_zoom_out = state.controls.get_action_state(game_state::Action::ZoomOut) > 0.5;

		let want_set_source = state
			.controls
			.get_action_state(game_state::Action::SelectSource)
			> 0.5;
		let want_set_target = state
			.controls
			.get_action_state(game_state::Action::SelectTarget)
			> 0.5;

		if state
			.controls
			.get_action_state(game_state::Action::RotateView)
			> 0.5
		{
			let s = 1.;
			self.camera_azim -= 2. * s * DT * want_left;
			self.camera_azim += 2. * s * DT * want_right;

			self.camera_elev -= s * DT * want_up;
			self.camera_elev += s * DT * want_down;
			self.camera_elev = utils::clamp(self.camera_elev, -PI / 2. + 1e-3, PI / 2. - 1e-3);
		}
		self.camera_radius *=
			1.1_f32.powf(want_zoom_out as i32 as f32 - want_zoom_in as i32 as f32);
		let rot = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -self.camera_azim);
		let left_right =
			rot * (Vector3::z_axis().into_inner() * (want_move_left - want_move_right));
		let fwd_bwd = rot * (Vector3::x_axis().into_inner() * (want_move_back - want_move_forward));
		let up_down = Vector3::y_axis().into_inner() * (want_move_up - want_move_down);

		self.camera_target += 5. * (left_right + fwd_bwd + up_down) * DT;

		if want_set_source || want_set_target
		{
			let mouse_pos = state.hs.mouse_pos.coords.cast::<f32>();
			let buffer_size = Vector2::new(state.hs.buffer_width(), state.hs.buffer_height());
			let mouse_pos_view = (mouse_pos - buffer_size / 2.)
				.component_div(&(buffer_size / 2.))
				.component_mul(&Vector2::new(1., -1.));
			let project = self.make_project(state);
			let camera = self.make_camera();

			let mouse_pos_camera =
				project.unproject_point(&Point3::new(mouse_pos_view.x, mouse_pos_view.y, -1.));

			let camera_pos = self.camera_pos();
			let mouse_pos_world = camera.inverse_transform_point(&mouse_pos_camera);
			let dir = (mouse_pos_world - camera_pos).normalize();

			if let Some((_, length)) = self.physics.ray_cast(None, camera_pos, dir, 1000.)
			{
				let pos = camera_pos + dir * length;
				let entity = if want_set_source
				{
					self.source
				}
				else
				{
					self.target
				};
				let mut position = self.world.get::<&mut comps::Position>(entity).unwrap();
				position.set_pos(pos);
			}
		}

		//if state.controls.get_action_state(game_state::Action::Move) > 0.5
		//{
		//	for (_, position) in self.world.query::<&mut comps::Position>().iter()
		//	{
		//		position.pos.y += 100. * DT;
		//	}
		//}

		// Movement.
		//for (_, position) in self.world.query::<&mut comps::Position>().iter()
		//{
		//	position.pos.x += 1500. * DT;
		//	if position.pos.x > state.buffer_width()
		//	{
		//		position.pos.x %= state.buffer_width();
		//		position.snapshot();
		//	}
		//}

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
		let buffer_width = state.hs.buffer_width();
		let buffer_height = state.hs.buffer_height();
		let fov = PI / 3.;
		Perspective3::new(buffer_width / buffer_height, fov, 0.1, 100.)
	}

	fn camera_pos(&self) -> Point3<f32>
	{
		let radius = self.camera_radius;
		let proj_radius = radius * self.camera_elev.cos();

		Point3::new(
			proj_radius * self.camera_azim.cos(),
			radius * self.camera_elev.sin(),
			proj_radius * self.camera_azim.sin(),
		) + self.camera_target.coords
	}

	fn make_camera(&self) -> Isometry3<f32>
	{
		utils::make_camera(self.camera_pos(), self.camera_target)
	}

	fn draw(&mut self, state: &mut game_state::GameState) -> Result<()>
	{
		let project = self.make_project(state);
		let camera = self.make_camera();
		let alpha = state.hs.alpha;

		// Forward pass.
		state
			.hs
			.core
			.use_projection_transform(&utils::mat4_to_transform(project.to_homogeneous()));
		state
			.hs
			.core
			.use_transform(&utils::mat4_to_transform(camera.to_homogeneous()));
		state
			.deferred_renderer
			.as_mut()
			.unwrap()
			.begin_forward_pass(&state.hs.core)?;
		state
			.hs
			.core
			.use_shader(Some(state.forward_shader.as_ref().unwrap()))
			.unwrap();

		let shift = Isometry3::new(Vector3::zeros(), Vector3::zeros()).to_homogeneous();

		state
			.hs
			.core
			.use_transform(&utils::mat4_to_transform(camera.to_homogeneous() * shift));
		state
			.hs
			.core
			.set_shader_transform("model_matrix", &utils::mat4_to_transform(shift))
			.ok();

		fn material_mapper<'l>(
			material: &scene::Material<game_state::MaterialKind>, texture_name: &str,
			state: &'l game_state::GameState,
		) -> Option<(scene::Material<game_state::MaterialKind>, &'l Bitmap)>
		{
			if material.desc.two_sided
			{
				unsafe {
					gl::Disable(gl::CULL_FACE);
				}
			}
			else
			{
				unsafe {
					gl::Enable(gl::CULL_FACE);
				}
			}
			state
				.get_bitmap(texture_name)
				.map(|b| (material.clone(), b))
				.ok()
		}

		state
			.hs
			.core
			.set_shader_sampler("lightmap", state.get_bitmap("data/level_lightmap.png")?, 1)
			.ok();
		state
			.hs
			.core
			.set_shader_uniform("base_color", &[[1.; 4]][..])
			.ok();

		state.get_scene("data/navmesh_test.glb")?.draw(
			&state.hs.core,
			&state.hs.prim,
			|_, _| None,
			material_mapper,
			|_, _, _| {},
			state,
		);

		// Solid pass.
		for (_, (position, scene)) in self
			.world
			.query::<(&comps::Position, &comps::Scene)>()
			.iter()
		{
			let shift = Isometry3 {
				translation: position.draw_pos(alpha).coords.into(),
				rotation: position.draw_rot(alpha),
			}
			.to_homogeneous();
			let scale = Matrix4::new_nonuniform_scaling(&position.draw_scale(alpha));

			let pos_fn =
				|obj_pos: Point3<f32>, obj_rot: UnitQuaternion<f32>, obj_scale: Vector3<f32>| {
					let obj_shift = Isometry3 {
						translation: obj_pos.coords.into(),
						rotation: obj_rot.into(),
					}
					.to_homogeneous();
					let obj_scale = Matrix4::new_nonuniform_scaling(&obj_scale);

					state.hs.core.use_transform(&utils::mat4_to_transform(
						camera.to_homogeneous() * shift * scale * obj_shift * obj_scale,
					));
					state
						.hs
						.core
						.set_shader_transform(
							"model_matrix",
							&utils::mat4_to_transform(shift * scale * obj_shift * obj_scale),
						)
						.ok();
				};

			state
				.hs
				.core
				.set_shader_uniform("base_color", &[scene.color.to_rgba_array_f()][..])
				.ok();

			state.get_scene(&scene.scene).unwrap().draw(
				&state.hs.core,
				&state.hs.prim,
				|_, _| None,
				material_mapper,
				pos_fn,
				state,
			);
		}

		// Light pass.
		state.deferred_renderer.as_mut().unwrap().begin_light_pass(
			&state.hs.core,
			state.light_shader.as_ref().unwrap(),
			&utils::mat4_to_transform(project.to_homogeneous()),
			self.camera_pos(),
		)?;

		for (_, (position, light)) in self
			.world
			.query::<(&comps::Position, &comps::Light)>()
			.iter()
		{
			let shift = Isometry3::new(position.draw_pos(alpha).coords, Vector3::zeros());
			let transform = Similarity3::from_isometry(shift, 2.5 * light.intensity.sqrt());
			let light_pos = transform.transform_point(&Point3::origin());

			let (r, g, b) = light.color.to_rgb_f();

			state
				.hs
				.core
				.set_shader_uniform("light_color", &[[r, g, b, 1.0]][..])
				.ok(); //.unwrap();
			state
				.hs
				.core
				.set_shader_uniform("light_pos", &[[light_pos.x, light_pos.y, light_pos.z]][..])
				.ok(); //.unwrap();
			state
				.hs
				.core
				.set_shader_uniform("light_intensity", &[light.intensity][..])
				.ok(); //.unwrap();
			state
				.hs
				.core
				.set_shader_uniform("is_static", &[light.static_ as i32][..])
				.ok(); //.unwrap();

			state.hs.core.use_transform(&utils::mat4_to_transform(
				camera.to_homogeneous() * transform.to_homogeneous(),
			));

			if let Ok(scene) = state.get_scene("data/sphere.glb")
			{
				scene.draw(
					&state.hs.core,
					&state.hs.prim,
					|_, _| None,
					|m, s, state| state.get_bitmap(s).map(|b| (m.clone(), b)).ok(),
					|_, _, _| {},
					state,
				);
			}
		}

		// Final pass.
		state.deferred_renderer.as_mut().unwrap().final_pass(
			&state.hs.core,
			&state.hs.prim,
			state.final_shader.as_ref().unwrap(),
			state.hs.buffer1.as_ref().unwrap(),
		)?;

		// Forward Additive pass.
		state
			.hs
			.core
			.use_shader(Some(state.eager_shader.as_ref().unwrap()))
			.unwrap();
		state
			.hs
			.core
			.use_projection_transform(&utils::mat4_to_transform(project.to_homogeneous()));
		state.hs.core.set_depth_test(Some(DepthFunction::Less));
		state
			.hs
			.core
			.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::One);

		for (_, (position, scene)) in self
			.world
			.query::<(&comps::Position, &comps::AdditiveScene)>()
			.iter()
		{
			// TODO: I bet we can fix pos_fn in the same way we fixed material_mapper.
			let shift = Isometry3 {
				translation: position.draw_pos(alpha).coords.into(),
				rotation: position.draw_rot(alpha),
			}
			.to_homogeneous();
			let scale = Matrix4::new_nonuniform_scaling(&position.draw_scale(alpha));

			let pos_fn =
				|obj_pos: Point3<f32>, obj_rot: UnitQuaternion<f32>, obj_scale: Vector3<f32>| {
					let obj_shift = Isometry3 {
						translation: obj_pos.coords.into(),
						rotation: obj_rot.into(),
					}
					.to_homogeneous();
					let obj_scale = Matrix4::new_nonuniform_scaling(&obj_scale);

					state.hs.core.use_transform(&utils::mat4_to_transform(
						camera.to_homogeneous() * shift * scale * obj_shift * obj_scale,
					));
					state
						.hs
						.core
						.set_shader_transform(
							"model_matrix",
							&utils::mat4_to_transform(shift * scale * obj_shift * obj_scale),
						)
						.ok();
				};

			state
				.hs
				.core
				.set_shader_uniform("base_color", &[scene.color.to_rgba_array_f()][..])
				.ok();

			state.get_scene(&scene.scene).unwrap().draw(
				&state.hs.core,
				&state.hs.prim,
				|_, _| None,
				material_mapper,
				pos_fn,
				state,
			);
		}

		state
			.hs
			.core
			.use_shader(Some(state.basic_shader.as_ref().unwrap()))
			.unwrap();
		unsafe {
			gl::Disable(gl::CULL_FACE);
		}
		state.hs.core.set_depth_test(None);
		state
			.hs
			.core
			.set_blender(BlendOperation::Add, BlendMode::One, BlendMode::InverseAlpha);
		let ortho_mat = Matrix4::new_orthographic(
			0.,
			state.hs.buffer_width() as f32,
			state.hs.buffer_height() as f32,
			0.,
			state.hs.buffer_height(),
			-state.hs.buffer_height(),
		);
		state
			.hs
			.core
			.use_projection_transform(&utils::mat4_to_transform(ortho_mat));
		state.hs.core.use_transform(&Transform::identity());
		Ok(())
	}
}
