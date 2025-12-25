use crate::astar;
use crate::error::Result;
use crate::utils;
use gltf::animation::util::ReadInputs;
use gltf::animation::util::ReadOutputs;
use nalgebra::{Point3, UnitQuaternion, Vector3};
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};

use allegro::*;
use allegro_primitives::*;

use std::collections::HashMap;
use std::fmt::Debug;

pub trait MaterialKind: Debug + Clone + Into<i32> {}
impl<T: Debug + Clone + Into<i32>> MaterialKind for T {}

fn default_frame_ms() -> f32
{
	1.
}

fn default_num_frames() -> i32
{
	1
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MaterialDesc<MaterialKindT: MaterialKind>
{
	pub texture: String,
	#[serde(default)]
	pub lightmap: String,
	pub material_kind: MaterialKindT,
	#[serde(default)]
	pub two_sided: bool,
	#[serde(default)]
	pub additive: bool,
	#[serde(default = "default_frame_ms")]
	pub frame_ms: f32,
	#[serde(default = "default_num_frames")]
	pub num_frames: i32,
	#[serde(default)]
	pub frame_dx: i32,
	#[serde(default)]
	pub frame_dy: i32,
}

#[derive(Debug, Clone)]
pub struct Material<MaterialKindT: MaterialKind>
{
	pub name: String,
	pub desc: MaterialDesc<MaterialKindT>,
}

pub struct Mesh<MaterialKindT: MaterialKind>
{
	pub vtxs: Vec<MeshVertex>,
	pub idxs: Vec<i32>,
	pub vertex_buffer: VertexBuffer<MeshVertex>,
	pub index_buffer: IndexBuffer<u32>,
	pub material: Option<Material<MaterialKindT>>,
}

#[derive(Clone, Debug)]
pub struct NavNode
{
	pub pos: Point3<f32>,
	pub neighbours: Vec<i32>,
}

#[derive(Clone, Debug)]
pub struct Position
{
	pub pos: Point3<f32>,
	pub rot: UnitQuaternion<f32>,
	pub scale: Vector3<f32>,
}

#[derive(Clone, Debug)]
pub struct Animation
{
	pub start: f64,
	pub end: f64,
	pub pos_track: Vec<(f64, Point3<f32>)>,
	pub rot_track: Vec<(f64, UnitQuaternion<f32>)>,
	pub scale_track: Vec<(f64, Vector3<f32>)>,
}

impl Animation
{
	fn new(
		pos_track: Vec<(f64, Point3<f32>)>, rot_track: Vec<(f64, UnitQuaternion<f32>)>,
		scale_track: Vec<(f64, Vector3<f32>)>,
	) -> Self
	{
		Self {
			start: 0.,
			end: 0.,
			pos_track: pos_track,
			rot_track: rot_track,
			scale_track: scale_track,
		}
	}

	fn compute_start_end(&mut self)
	{
		let mut min = std::f64::MAX;
		let mut max = std::f64::MIN;
		let mut valid = false;
		if !self.pos_track.is_empty()
		{
			min = min.min(self.pos_track[0].0);
			max = max.max(self.pos_track[self.pos_track.len() - 1].0);
			valid = true;
		}
		if !self.rot_track.is_empty()
		{
			min = min.min(self.rot_track[0].0);
			max = max.max(self.rot_track[self.rot_track.len() - 1].0);
			valid = true;
		}
		if !self.scale_track.is_empty()
		{
			min = min.min(self.scale_track[0].0);
			max = max.max(self.scale_track[self.scale_track.len() - 1].0);
			valid = true;
		}
		if valid
		{
			self.start = min;
			self.end = max;
		}
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnimationState
{
	animation_name: String,
	new_animation_name: String,
	animation_progress: f64,
	pub pos_frame_idx: i32,
	pub rot_frame_idx: i32,
	pub scale_frame_idx: i32,
	pub once: bool,
	num_loops: i32,
	need_reset: bool,
	done: bool,
}

impl AnimationState
{
	pub fn new(animation_name: &str, once: bool) -> Self
	{
		Self {
			animation_name: animation_name.to_string(),
			new_animation_name: animation_name.to_string(),
			animation_progress: 0.,
			once: once,
			pos_frame_idx: 0,
			rot_frame_idx: 0,
			scale_frame_idx: 0,
			num_loops: 0,
			need_reset: false,
			done: false,
		}
	}

	pub fn reset(&mut self)
	{
		self.need_reset = true;
	}

	pub fn set_new_animation(&mut self, animation_name: impl Into<String>)
	{
		self.new_animation_name = animation_name.into();
	}

	pub fn get_num_loops(&self) -> i32
	{
		self.num_loops
	}

	pub fn is_done(&self) -> bool
	{
		self.done
	}
}

pub enum ObjectKind<MaterialKindT: MaterialKind>
{
	MultiMesh
	{
		meshes: Vec<Mesh<MaterialKindT>>,
	},
	CollisionMesh
	{
		meshes: Vec<Mesh<MaterialKindT>>,
	},
	NavMesh
	{
		nodes: Vec<NavNode>,
	},
	Light
	{
		intensity: f32,
		color: Color,
	},
	Empty,
}

pub struct Object<MaterialKindT: MaterialKind>
{
	pub name: String,
	pub pos: Point3<f32>,
	pub rot: UnitQuaternion<f32>,
	pub scale: Vector3<f32>,
	pub kind: ObjectKind<MaterialKindT>,
	pub animations: HashMap<String, Animation>,
	pub properties: serde_json::Value,
}

impl<MaterialKindT: MaterialKind> Object<MaterialKindT>
{
	pub fn get_animation_position(
		&self, state: &AnimationState,
	) -> (Point3<f32>, UnitQuaternion<f32>, Vector3<f32>)
	{
		let animation = &self.animations.get(&state.animation_name).expect(&format!(
			"Could not find animation '{}'",
			state.animation_name
		));

		// TODO: support steps
		let pos = if animation.pos_track.is_empty()
		{
			self.pos
		}
		else
		{
			let cur_frame = animation.pos_track[state.pos_frame_idx as usize];
			if state.pos_frame_idx as usize + 1 >= animation.pos_track.len()
			{
				cur_frame.1
			}
			else
			{
				let next_frame = animation.pos_track[state.pos_frame_idx as usize + 1];
				let f = ((state.animation_progress - cur_frame.0) / (next_frame.0 - cur_frame.0))
					as f32;
				cur_frame.1 + f * (next_frame.1 - cur_frame.1)
			}
		};
		let rot = if animation.rot_track.is_empty()
		{
			self.rot
		}
		else
		{
			let cur_frame = animation.rot_track[state.rot_frame_idx as usize];
			if state.rot_frame_idx as usize + 1 >= animation.rot_track.len()
			{
				cur_frame.1
			}
			else
			{
				let next_frame = animation.rot_track[state.rot_frame_idx as usize + 1];
				let f = ((state.animation_progress - cur_frame.0) / (next_frame.0 - cur_frame.0))
					as f32;
				cur_frame.1.slerp(&next_frame.1, f)
			}
		};
		let scale = if animation.scale_track.is_empty()
		{
			self.scale
		}
		else
		{
			let cur_frame = animation.scale_track[state.scale_frame_idx as usize];
			if state.scale_frame_idx as usize + 1 >= animation.scale_track.len()
			{
				cur_frame.1
			}
			else
			{
				let next_frame = animation.scale_track[state.scale_frame_idx as usize + 1];
				let f = ((state.animation_progress - cur_frame.0) / (next_frame.0 - cur_frame.0))
					as f32;
				cur_frame.1 + f * (next_frame.1 - cur_frame.1)
			}
		};

		(pos, rot, scale)
	}

	pub fn draw<
		'l,
		BitmapFn: Fn(&Material<MaterialKindT>, &str) -> Option<&'l Bitmap>,
		PosFn: Fn(Point3<f32>, UnitQuaternion<f32>, Vector3<f32>) -> (),
	>(
		&self, core: &Core, prim: &PrimitivesAddon, animation_state: Option<&AnimationState>,
		bitmap_fn: BitmapFn, pos_fn: PosFn,
	)
	{
		if let Some(state) = animation_state
		{
			let (pos, rot, scale) = self.get_animation_position(state);
			pos_fn(pos, rot, scale);
		}
		else
		{
			pos_fn(self.pos, self.rot, self.scale);
		}
		if let ObjectKind::MultiMesh { meshes } = &self.kind
		{
			for mesh in meshes
			{
				core.set_shader_uniform(
					"material",
					&[mesh
						.material
						.as_ref()
						.map(|m| Into::<i32>::into(m.desc.material_kind.clone()))
						.unwrap_or(0)][..],
				)
				.ok();
				let bitmap = mesh
					.material
					.as_ref()
					.and_then(|m| bitmap_fn(&m, &m.desc.texture));
				if let Some(bitmap) = bitmap
				{
					let material = mesh.material.as_ref().unwrap();
					core.set_shader_uniform(
						"tex_size",
						&[[bitmap.get_width() as f32, bitmap.get_height() as f32]][..],
					)
					.ok();
					core.set_shader_uniform("frame_ms", &[material.desc.frame_ms][..])
						.ok();
					core.set_shader_uniform("num_frames", &[material.desc.num_frames as i32][..])
						.ok();
					core.set_shader_uniform(
						"frame_dxy",
						&[[material.desc.frame_dx as f32, material.desc.frame_dy as f32]][..],
					)
					.ok();
				}
				prim.draw_indexed_buffer(
					&mesh.vertex_buffer,
					bitmap,
					&mesh.index_buffer,
					0,
					mesh.idxs.len() as u32,
					PrimType::TriangleList,
				);
			}
		}
	}

	pub fn advance_state(&self, state: &mut AnimationState, amount: f64)
	{
		state.num_loops = 0;

		let reset_activations =
			(state.animation_name != state.new_animation_name) || state.need_reset;
		let animation = &self.animations.get(&state.animation_name).expect(&format!(
			"Could not find animation '{}'",
			state.animation_name
		));
		if reset_activations
		{
			state.animation_name = state.new_animation_name.clone();
			state.pos_frame_idx = 0;
			state.rot_frame_idx = 0;
			state.scale_frame_idx = 0;
			state.animation_progress = 0.;
			state.need_reset = false;
			state.done = false;
		}
		state.animation_progress += amount;

		loop
		{
			let old_pos_frame_idx = state.pos_frame_idx;
			let old_rot_frame_idx = state.rot_frame_idx;
			let old_scale_frame_idx = state.scale_frame_idx;

			if !animation.pos_track.is_empty()
			{
				if state.animation_progress > animation.pos_track[state.pos_frame_idx as usize].0
				{
					state.pos_frame_idx =
						(state.pos_frame_idx + 1).min(animation.pos_track.len() as i32 - 1);
				}
			}
			if !animation.rot_track.is_empty()
			{
				if state.animation_progress > animation.rot_track[state.rot_frame_idx as usize].0
				{
					state.rot_frame_idx =
						(state.rot_frame_idx + 1).min(animation.rot_track.len() as i32 - 1);
				}
			}
			if !animation.scale_track.is_empty()
			{
				if state.animation_progress > animation.pos_track[state.pos_frame_idx as usize].0
				{
					state.pos_frame_idx =
						(state.pos_frame_idx + 1).min(animation.pos_track.len() as i32 - 1);
				}
			}
			if state.animation_progress > animation.end
			{
				if !state.done
				{
					state.num_loops += 1;
				}
				if state.once
				{
					state.done = true;
				}
				else
				{
					state.pos_frame_idx = 0;
					state.rot_frame_idx = 0;
					state.scale_frame_idx = 0;
					state.animation_progress -= animation.end;
				}
			}
			if old_pos_frame_idx == state.pos_frame_idx
				&& old_rot_frame_idx == state.rot_frame_idx
				&& old_scale_frame_idx == state.scale_frame_idx
			{
				break;
			}
		}
	}

	pub fn create_clone(
		&self, display: &mut Display, prim: &PrimitivesAddon, read_write: bool,
	) -> Result<Self>
	{
		let kind = match &self.kind
		{
			ObjectKind::Empty => ObjectKind::Empty,
			ObjectKind::Light { intensity, color } => ObjectKind::Light {
				intensity: *intensity,
				color: *color,
			},
			ObjectKind::NavMesh { nodes } => ObjectKind::NavMesh {
				nodes: nodes.clone(),
			},
			ObjectKind::CollisionMesh { .. } =>
			{
				unimplemented!();
			}
			ObjectKind::MultiMesh { meshes } =>
			{
				let mut new_meshes = vec![];
				for mesh in meshes
				{
					let (vertex_buffer, index_buffer) =
						create_buffers(display, prim, &mesh.vtxs, &mesh.idxs, read_write)?;

					new_meshes.push(Mesh {
						vtxs: mesh.vtxs.clone(),
						idxs: mesh.idxs.clone(),
						material: mesh.material.clone(),
						vertex_buffer: vertex_buffer,
						index_buffer: index_buffer,
					});
				}
				ObjectKind::MultiMesh { meshes: new_meshes }
			}
		};
		Ok(Self {
			name: self.name.clone(),
			pos: self.pos.clone(),
			rot: self.rot.clone(),
			scale: self.scale.clone(),
			kind: kind,
			animations: self.animations.clone(),
			properties: self.properties.clone(),
		})
	}
}

pub struct Scene<MaterialKindT: MaterialKind>
{
	pub objects: Vec<Object<MaterialKindT>>,
}

impl<MaterialKindT: MaterialKind + DeserializeOwned> Scene<MaterialKindT>
{
	pub fn load(display: &mut Display, prim: &PrimitivesAddon, file: &str) -> Result<Self>
	{
		if file.ends_with("obj")
		{
			Self::load_obj(display, prim, file)
		}
		else
		{
			Self::load_gltf(display, prim, file)
		}
	}

	pub fn load_obj(display: &mut Display, prim: &PrimitivesAddon, obj_file: &str) -> Result<Self>
	{
		let obj_str = std::fs::read_to_string(obj_file)?;
		let obj_set = wavefront_obj::obj::parse(obj_str)
			.map_err(|e| format!("Error while reading {}: {}", obj_file, e.to_string()))?;

		let mut objects = vec![];
		for obj in obj_set.objects
		{
			if obj.name == "Navmesh"
			{
				unimplemented!();
			}

			let mut meshes = vec![];
			for geom in &obj.geometry
			{
				let mut vtx_map = HashMap::new();
				let mut vtxs = vec![];
				let mut idxs = vec![];

				for shape in &geom.shapes
				{
					if let wavefront_obj::obj::Primitive::Triangle(idx1, idx2, idx3) =
						shape.primitive
					{
						for idx in [idx1, idx2, idx3]
						{
							let vtx_idx = vtx_map.entry(idx).or_insert_with(|| {
								let pos = obj.vertices[idx.0];
								let uv = idx.1.map(|idx| obj.tex_vertices[idx]).unwrap_or(
									wavefront_obj::obj::TVertex {
										u: 0.,
										v: 0.,
										w: 0.,
									},
								);
								let norm = idx.2.map(|idx| obj.normals[idx]).unwrap_or(
									wavefront_obj::obj::Normal {
										x: 0.,
										y: 0.,
										z: 0.,
									},
								);
								let vtx_idx = vtxs.len();
								vtxs.push(MeshVertex {
									x: pos.x as f32,
									y: pos.y as f32,
									z: pos.z as f32,
									u: uv.u as f32,
									v: uv.v as f32,
									u2: 0.,
									v2: 0.,
									nx: norm.x as f32,
									ny: norm.y as f32,
									nz: norm.z as f32,
									color: Color::from_rgb_f(1., 1., 1.),
								});
								vtx_idx
							});
							idxs.push(*vtx_idx as i32);
						}
					}
					else
					{
						unimplemented!();
					}
				}
				let material = geom
					.material_name
					.as_ref()
					.map(|name| {
						(
							name.to_string(),
							utils::load_config(&format!("data/{}.cfg", name)),
						)
					})
					.map_or(Ok(None), |(name, desc)| {
						desc.map(|desc| {
							Some(Material {
								name: name,
								desc: desc,
							})
						})
					})?;

				let (vertex_buffer, index_buffer) =
					create_buffers(display, prim, &vtxs, &idxs, false)?;

				meshes.push(Mesh {
					vtxs: vtxs,
					idxs: idxs,
					vertex_buffer: vertex_buffer,
					index_buffer: index_buffer,
					material: material,
				});
			}

			let kind = if obj.name.starts_with("Collision")
			{
				ObjectKind::CollisionMesh { meshes: meshes }
			}
			else
			{
				ObjectKind::MultiMesh { meshes: meshes }
			};

			let object = Object {
				name: obj.name.clone(),
				pos: Point3::origin(),
				rot: UnitQuaternion::identity(),
				scale: Vector3::new(1.0, 1.0, 1.0),
				kind: kind,
				animations: HashMap::new(),
				properties: serde_json::Value::Null,
			};
			objects.push(object);
		}
		Ok(Self { objects: objects })
	}

	pub fn load_gltf(display: &mut Display, prim: &PrimitivesAddon, gltf_file: &str)
	-> Result<Self>
	{
		let (document, buffers, _) = gltf::import(gltf_file)?;
		let mut objects = vec![];

		let mut obj_to_name_to_animation: HashMap<String, HashMap<String, Animation>> =
			HashMap::new();
		for animation in document.animations()
		{
			let name = animation.name().unwrap_or("").to_string();
			for channel in animation.channels()
			{
				let target = channel
					.target()
					.node()
					.name()
					.ok_or_else(|| {
						format!(
							"Animation '{}' in '{}' is missing a target?",
							name, gltf_file
						)
					})?
					.to_string();
				let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));

				let times = if let Some(ReadInputs::Standard(inputs)) = reader.read_inputs()
				{
					inputs.map(|t| t as f64).collect::<Vec<_>>()
				}
				else
				{
					return Err(format!(
						"Animation '{}' in '{}' has sparse values, unsupported.",
						name, gltf_file
					))?;
				};

				// TODO: This is trully horrid, surely we can do better.
				match reader.read_outputs()
				{
					Some(ReadOutputs::Translations(translations)) =>
					{
						let track: Vec<_> = times
							.iter()
							.zip(translations)
							.map(|(&t, v)| (t, Point3::from(v)))
							.collect();

						obj_to_name_to_animation
							.entry(target)
							.and_modify(|name_to_animation| {
								name_to_animation
									.entry(name.clone())
									.and_modify(|animation| {
										animation.pos_track = track.clone();
									})
									.or_insert(Animation::new(track.clone(), vec![], vec![]));
							})
							.or_insert_with(|| {
								let mut name_to_animation = HashMap::new();
								name_to_animation.insert(
									name.clone(),
									Animation::new(track.clone(), vec![], vec![]),
								);
								name_to_animation
							});
					}
					Some(ReadOutputs::Rotations(gltf::animation::util::Rotations::F32(
						rotations,
					))) =>
					{
						let track: Vec<_> = times
							.iter()
							.zip(rotations)
							.map(|(&t, v)| (t, UnitQuaternion::from_quaternion(v.into())))
							.collect();
						obj_to_name_to_animation
							.entry(target)
							.and_modify(|name_to_animation| {
								name_to_animation
									.entry(name.clone())
									.and_modify(|animation| {
										animation.rot_track = track.clone();
									})
									.or_insert(Animation::new(vec![], track.clone(), vec![]));
							})
							.or_insert_with(|| {
								let mut name_to_animation = HashMap::new();
								name_to_animation.insert(
									name.clone(),
									Animation::new(vec![], track.clone(), vec![]),
								);
								name_to_animation
							});
					}
					Some(ReadOutputs::Scales(scales)) =>
					{
						let track: Vec<_> = times
							.iter()
							.zip(scales)
							.map(|(&t, v)| (t, v.into()))
							.collect();
						obj_to_name_to_animation
							.entry(target)
							.and_modify(|name_to_animation| {
								name_to_animation
									.entry(name.clone())
									.and_modify(|animation| {
										animation.scale_track = track.clone();
									})
									.or_insert(Animation::new(vec![], vec![], track.clone()));
							})
							.or_insert_with(|| {
								let mut name_to_animation = HashMap::new();
								name_to_animation.insert(
									name.clone(),
									Animation::new(vec![], vec![], track.clone()),
								);
								name_to_animation
							});
					}
					_ => (),
				}
			}
		}

		for name_to_animation in &mut obj_to_name_to_animation.values_mut()
		{
			for animation in name_to_animation.values_mut()
			{
				animation.compute_start_end();
			}
		}

		for node in document.nodes()
		{
			let (translation, rot, scale) = node.transform().decomposed();
			let pos = translation.into();
			let rot = UnitQuaternion::from_quaternion(rot.into());
			let scale = scale.into();
			let object;
			let name = node.name().unwrap_or("").to_string();
			let animations = obj_to_name_to_animation
				.get(&name)
				.unwrap_or(&HashMap::new())
				.clone();
			let properties = if let Some(extras) = node.extras()
			{
				serde_json::from_str(extras.get())?
			}
			else
			{
				serde_json::Value::Null
			};

			if name == "Navmesh"
			{
				object = Object {
					name: node.name().unwrap_or("").to_string(),
					pos: pos,
					rot: rot,
					scale: scale,
					kind: ObjectKind::NavMesh {
						nodes: get_navmesh(&node, &buffers)?,
					},
					animations: animations,
					properties: properties,
				}
			}
			else if let Some(light) = node.light()
			{
				let color = light.color();
				object = Object {
					name: node.name().unwrap_or("").to_string(),
					pos: pos,
					rot: rot,
					scale: scale,
					kind: ObjectKind::Light {
						intensity: light.intensity(),
						color: Color::from_rgb_f(color[0], color[1], color[2]),
					},
					animations: animations,
					properties: properties,
				};
			}
			else if let Some(mesh) = node.mesh()
			{
				let mut meshes = vec![];
				for primitive in mesh.primitives()
				{
					let mut vtxs = vec![];
					let mut idxs = vec![];
					let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
					if let (
						Some(pos_iter),
						Some(gltf::mesh::util::ReadTexCoords::F32(uv_iter)),
						Some(normal_iter),
					) = (
						reader.read_positions(),
						reader.read_tex_coords(0),
						reader.read_normals(),
					)
					{
						if let Some(gltf::mesh::util::ReadTexCoords::F32(uv2_iter)) =
							reader.read_tex_coords(1)
						{
							for (((pos, uv), uv2), normal) in
								pos_iter.zip(uv_iter).zip(uv2_iter).zip(normal_iter)
							{
								vtxs.push(MeshVertex {
									x: pos[0],
									y: pos[1],
									z: pos[2],
									u: uv[0],
									v: 1. - uv[1],
									u2: uv2[0],
									v2: 1. - uv2[1],
									nx: normal[0],
									ny: normal[1],
									nz: normal[2],
									color: Color::from_rgb_f(1., 1., 1.),
								});
							}
						}
						else
						{
							for ((pos, uv), normal) in pos_iter.zip(uv_iter).zip(normal_iter)
							{
								vtxs.push(MeshVertex {
									x: pos[0],
									y: pos[1],
									z: pos[2],
									u: uv[0],
									v: 1. - uv[1],
									u2: uv[0],
									v2: 1. - uv[1],
									nx: normal[0],
									ny: normal[1],
									nz: normal[2],
									color: Color::from_rgb_f(1., 1., 1.),
								});
							}
						}
					}

					if let Some(iter) = reader.read_indices()
					{
						for idx in iter.into_u32()
						{
							idxs.push(idx as i32)
						}
					}

					let material = primitive
						.material()
						.name()
						.map(|name| {
							(
								name.to_string(),
								utils::load_config(&format!("data/{}.cfg", name)),
							)
						})
						.map_or(Ok(None), |(name, desc)| {
							desc.map(|desc| {
								Some(Material {
									name: name,
									desc: desc,
								})
							})
						})?;

					let (vertex_buffer, index_buffer) =
						create_buffers(display, prim, &vtxs, &idxs, false)?;

					meshes.push(Mesh {
						vtxs: vtxs,
						idxs: idxs,
						vertex_buffer: vertex_buffer,
						index_buffer: index_buffer,
						material: material,
					});
				}

				let name = node.name().unwrap_or("").to_string();
				let kind = if name.starts_with("Collision")
				{
					ObjectKind::CollisionMesh { meshes: meshes }
				}
				else
				{
					ObjectKind::MultiMesh { meshes: meshes }
				};

				object = Object {
					name: name,
					pos: pos,
					rot: rot,
					scale: scale,
					kind: kind,
					animations: animations,
					properties: properties,
				};
			}
			else
			{
				object = Object {
					name: name,
					pos: pos,
					rot: rot,
					scale: scale,
					kind: ObjectKind::Empty,
					animations: animations,
					properties: properties,
				};
			}
			objects.push(object);
		}
		Ok(Self { objects: objects })
	}

	pub fn draw<
		'l,
		AnimationFn: Fn(usize, &Object<MaterialKindT>) -> Option<&'l AnimationState>,
		BitmapFn: Fn(&Material<MaterialKindT>, &str) -> Option<&'l Bitmap>,
		PosFn: Fn(Point3<f32>, UnitQuaternion<f32>, Vector3<f32>) -> (),
	>(
		&self, core: &Core, prim: &PrimitivesAddon, animation_state_fn: AnimationFn,
		bitmap_fn: BitmapFn, pos_fn: PosFn,
	)
	{
		for (idx, object) in self.objects.iter().enumerate()
		{
			object.draw(
				core,
				prim,
				animation_state_fn(idx, object),
				&bitmap_fn,
				&pos_fn,
			);
		}
	}

	pub fn clip_meshes(
		&self, display: &mut Display, prim: &PrimitivesAddon,
		keep_triangle_fn: impl Fn(Vector3<f32>) -> bool,
	) -> Result<Scene<MaterialKindT>>
	{
		let mut objects = vec![];

		for object in &self.objects
		{
			let kind = match &object.kind
			{
				ObjectKind::Empty => ObjectKind::Empty,
				ObjectKind::Light { intensity, color } => ObjectKind::Light {
					intensity: *intensity,
					color: *color,
				},
				ObjectKind::NavMesh { nodes } => ObjectKind::NavMesh {
					nodes: nodes.clone(),
				},
				ObjectKind::CollisionMesh { .. } =>
				{
					unimplemented!();
				}
				ObjectKind::MultiMesh { meshes } =>
				{
					let mut new_meshes = vec![];
					for mesh in meshes
					{
						let mut new_indices = vec![];
						for triangle in mesh.idxs.chunks(3)
						{
							let v1 = &mesh.vtxs[triangle[0] as usize];
							let v2 = &mesh.vtxs[triangle[1] as usize];
							let v3 = &mesh.vtxs[triangle[2] as usize];

							let centre = (Vector3::new(v1.x, v1.y, v1.z)
								+ Vector3::new(v2.x, v2.y, v2.z)
								+ Vector3::new(v3.x, v3.y, v3.z))
								/ 3.;
							if keep_triangle_fn(centre)
							{
								new_indices.extend(triangle.iter().copied());
							}
						}

						let (vertex_buffer, index_buffer) =
							create_buffers(display, prim, &mesh.vtxs, &new_indices, false)?;

						new_meshes.push(Mesh {
							vtxs: mesh.vtxs.clone(),
							idxs: new_indices,
							material: mesh.material.clone(),
							vertex_buffer: vertex_buffer,
							index_buffer: index_buffer,
						});
					}
					ObjectKind::MultiMesh { meshes: new_meshes }
				}
			};
			objects.push(Object {
				name: object.name.clone(),
				pos: object.pos.clone(),
				rot: object.rot.clone(),
				scale: object.scale.clone(),
				kind: kind,
				animations: object.animations.clone(),
				properties: object.properties.clone(),
			})
		}

		Ok(Scene { objects: objects })
	}
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct MeshVertex
{
	pub x: f32,
	pub y: f32,
	pub z: f32,
	pub u: f32,
	pub v: f32,
	pub u2: f32,
	pub v2: f32,
	pub nx: f32,
	pub ny: f32,
	pub nz: f32,
	pub color: Color,
}

unsafe impl VertexType for MeshVertex
{
	fn get_decl(prim: &PrimitivesAddon) -> VertexDecl
	{
		fn make_builder() -> std::result::Result<VertexDeclBuilder, ()>
		{
			VertexDeclBuilder::new(std::mem::size_of::<MeshVertex>())
				.pos(
					VertexAttrStorage::F32_3,
					memoffset::offset_of!(MeshVertex, x),
				)?
				.uv(
					VertexAttrStorage::F32_2,
					memoffset::offset_of!(MeshVertex, u),
				)?
				.color(memoffset::offset_of!(MeshVertex, color))?
				.user_attr(
					VertexAttrStorage::F32_3,
					memoffset::offset_of!(MeshVertex, nx),
				)?
				.user_attr(
					VertexAttrStorage::F32_2,
					memoffset::offset_of!(MeshVertex, u2),
				)
		}

		VertexDecl::from_builder(prim, &make_builder().unwrap())
	}
}

fn get_navmesh(node: &gltf::Node, buffers: &[gltf::buffer::Data]) -> Result<Vec<NavNode>>
{
	let mesh = node.mesh();
	let mesh = mesh.as_ref().ok_or("No mesh in navmesh".to_string())?;
	let prim = mesh
		.primitives()
		.next()
		.ok_or("No prim in navmesh".to_string())?;

	let mut vtxs = vec![];
	let mut idxs = vec![];
	let reader = prim.reader(|buffer| Some(&buffers[buffer.index()]));
	if let Some(pos_iter) = reader.read_positions()
	{
		for pos in pos_iter
		{
			vtxs.push(Point3::new(pos[0], pos[1], pos[2]));
		}
	}

	let tol = 1e-3;
	let get_vtx_id = |pos: Point3<f32>| {
		(
			(pos.x / tol) as i32,
			(pos.y / tol) as i32,
			(pos.z / tol) as i32,
		)
	};
	let mut vtx_id_to_new_idx = HashMap::new();
	let mut new_idxs = vec![];
	let mut old_idxs = vec![];
	let mut cur_idx = 0;
	for (old_idx, vtx) in vtxs.iter().enumerate()
	{
		let vtx_id = get_vtx_id(*vtx);
		let new_idx = *vtx_id_to_new_idx.entry(vtx_id).or_insert_with(|| {
			let new_idx = cur_idx;
			cur_idx += 1;
			old_idxs.push(old_idx);
			new_idx
		});
		new_idxs.push(new_idx);
	}

	if let Some(iter) = reader.read_indices()
	{
		for idx in iter.into_u32()
		{
			idxs.push(idx as i32)
		}
	}

	let mut new_vtx_idx_to_neighbours = HashMap::new();
	for triangle in idxs.chunks(3)
	{
		for &vtx_idx in triangle
		{
			new_vtx_idx_to_neighbours
				.entry(new_idxs[vtx_idx as usize])
				.or_insert(vec![])
				.extend([
					new_idxs[triangle[0] as usize],
					new_idxs[triangle[1] as usize],
					new_idxs[triangle[2] as usize],
				]);
		}
	}

	let mut ret = vec![];
	for (new_idx, &old_idx) in old_idxs.iter().enumerate()
	{
		let new_idx = new_idx as i32;
		let mut neighbours = vec![];
		for &neighbour_vtx_idx in &new_vtx_idx_to_neighbours[&new_idx]
		{
			if neighbour_vtx_idx != new_idx
			{
				neighbours.push(neighbour_vtx_idx);
			}
		}
		neighbours.sort();
		neighbours.dedup();
		let node = NavNode {
			pos: vtxs[old_idx],
			neighbours: neighbours,
		};
		ret.push(node);
	}
	Ok(ret)
}

impl astar::Node for NavNode
{
	fn get_pos(&self) -> Point3<f32>
	{
		self.pos
	}

	fn get_neighbours(&self) -> &[i32]
	{
		&self.neighbours
	}
}

fn create_buffers(
	display: &mut Display, prim: &PrimitivesAddon, vtxs: &[MeshVertex], idxs: &[i32],
	read_write: bool,
) -> Result<(VertexBuffer<MeshVertex>, IndexBuffer<u32>)>
{
	let flags = if read_write
	{
		BUFFER_READWRITE
	}
	else
	{
		BUFFER_STATIC
	};
	let vertex_buffer = VertexBuffer::new(display, prim, Some(&vtxs), vtxs.len() as u32, flags)
		.map_err(|_| "Could not create vertex buffer".to_string())?;
	let index_buffer = IndexBuffer::new(
		display,
		prim,
		Some(&idxs.iter().map(|&i| i as u32).collect::<Vec<_>>()),
		idxs.len() as u32,
		BUFFER_STATIC,
	)
	.map_err(|_| "Could not create index buffer".to_string())?;
	Ok((vertex_buffer, index_buffer))
}
