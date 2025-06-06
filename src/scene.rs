use crate::error::Result;
use crate::utils;
use nalgebra::Point3;
use serde_derive::{Deserialize, Serialize};

use allegro::*;
use allegro_primitives::*;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[repr(i32)]
pub enum MaterialKind
{
	Static = 0,
	Dynamic = 1,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MaterialDesc
{
	pub texture: String,
	pub material_kind: MaterialKind,
}

#[derive(Debug, Clone)]
pub struct Material
{
	pub name: String,
	pub desc: MaterialDesc,
}

#[derive(Clone, Debug)]
pub struct Mesh
{
	pub vtxs: Vec<NormVertex>,
	pub idxs: Vec<i32>,
	pub material: Option<Material>,
}

#[derive(Clone, Debug)]
pub enum ObjectKind
{
	MultiMesh
	{
		meshes: Vec<Mesh>
	},
	Light
	{
		intensity: f32, color: Color
	},
}

#[derive(Clone, Debug)]
pub struct Object
{
	pub position: Point3<f32>,
	pub kind: ObjectKind,
}

#[derive(Clone, Debug)]
pub struct Scene
{
	pub objects: Vec<Object>,
}

impl Scene
{
	pub fn load(gltf_file: &str) -> Result<Self>
	{
		let (document, buffers, _) = gltf::import(gltf_file)?;
		let mut objects = vec![];
		for node in document.nodes()
		{
			let (translation, _rot, _scale) = node.transform().decomposed();
			let position = Point3::new(translation[0], translation[1], translation[2]);
			let mut object = None;
			if let Some(light) = node.light()
			{
				let color = light.color();
				object = Some(Object {
					position: position,
					kind: ObjectKind::Light {
						intensity: light.intensity(),
						color: Color::from_rgb_f(color[0], color[1], color[2]),
					},
				});
			}
			else if let Some(mesh) = node.mesh()
			{
				let mut meshes = vec![];
				for prim in mesh.primitives()
				{
					let mut vtxs = vec![];
					let mut idxs = vec![];
					let reader = prim.reader(|buffer| Some(&buffers[buffer.index()]));
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
								vtxs.push(NormVertex {
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
								vtxs.push(NormVertex {
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

					let material = prim
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
					meshes.push(Mesh {
						vtxs: vtxs,
						idxs: idxs,
						material: material,
					});
				}
				object = Some(Object {
					position: position,
					kind: ObjectKind::MultiMesh { meshes: meshes },
				});
			}
			if let Some(object) = object
			{
				objects.push(object);
			}
		}
		Ok(Self { objects: objects })
	}

	pub fn draw<'l, T: Fn(&Material, &str) -> Result<&'l Bitmap>>(
		&self, core: &Core, prim: &PrimitivesAddon, bitmap_fn: T,
	)
	{
		for object in self.objects.iter()
		{
			if let ObjectKind::MultiMesh { meshes } = &object.kind
			{
				for mesh in meshes
				{
					core.set_shader_uniform(
						"material",
						&[mesh
							.material
							.as_ref()
							.map(|m| m.desc.material_kind as i32)
							.unwrap_or(0)][..],
					)
					.ok();
					prim.draw_indexed_prim(
						&mesh.vtxs[..],
						mesh.material
							.as_ref()
							.and_then(|m| Some(bitmap_fn(&m, &m.desc.texture).unwrap())),
						&mesh.idxs[..],
						0,
						mesh.idxs.len() as u32,
						PrimType::TriangleList,
					);
				}
			}
		}
	}
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct NormVertex
{
	x: f32,
	y: f32,
	z: f32,
	u: f32,
	v: f32,
	u2: f32,
	v2: f32,
	nx: f32,
	ny: f32,
	nz: f32,
	color: Color,
}

unsafe impl VertexType for NormVertex
{
	fn get_decl(prim: &PrimitivesAddon) -> VertexDecl
	{
		fn make_builder() -> std::result::Result<VertexDeclBuilder, ()>
		{
			VertexDeclBuilder::new(std::mem::size_of::<NormVertex>())
				.pos(
					VertexAttrStorage::F32_3,
					memoffset::offset_of!(NormVertex, x),
				)?
				.uv(
					VertexAttrStorage::F32_2,
					memoffset::offset_of!(NormVertex, u),
				)?
				.color(memoffset::offset_of!(NormVertex, color))?
				.user_attr(
					VertexAttrStorage::F32_3,
					memoffset::offset_of!(NormVertex, nx),
				)?
				.user_attr(
					VertexAttrStorage::F32_2,
					memoffset::offset_of!(NormVertex, u2),
				)
		}

		VertexDecl::from_builder(prim, &make_builder().unwrap())
	}
}
