use crate::error::{Error, Result};
use allegro::*;
use allegro_audio::*;
use allegro_color::*;
use allegro_font::*;
use allegro_ttf::*;
use nalgebra;
use rand::prelude::*;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use slr_config::{from_element, to_element, ConfigElement, Source};
use std::{fs, path};

pub const DT: f32 = 1. / 60.;
pub const PI: f32 = std::f32::consts::PI;
pub type Vec2D = nalgebra::Vector2<f32>;
pub type Vec3D = nalgebra::Vector3<f32>;

use na::{
	Isometry3, Matrix4, Perspective3, Point2, Point3, Quaternion, RealField, Rotation2, Rotation3,
	Unit, Vector2, Vector3, Vector4,
};
use nalgebra as na;

pub fn projection_transform(dw: f32, dh: f32, fov: f32) -> Perspective3<f32>
{
	Perspective3::new(dw / dh, fov, 0.01, 200.)
}

pub fn mat4_to_transform(mat: Matrix4<f32>) -> Transform
{
	let mut trans = Transform::identity();
	for i in 0..4
	{
		for j in 0..4
		{
			trans.get_matrix_mut()[j][i] = mat[(i, j)];
		}
	}
	trans
}

pub fn make_camera(eye: Point3<f32>, target: Point3<f32>) -> Isometry3<f32>
{
	Isometry3::look_at_rh(&eye, &target, &Vector3::y())
}

pub fn random_color(seed: u64, saturation: f32, value: f32) -> Color
{
	let mut rng = StdRng::seed_from_u64(seed);
	Color::from_hsv(rng.gen_range(0. ..360.), saturation, value)
}

pub trait ColorExt
{
	fn interpolate(&self, other: Color, f: f32) -> Color;
}

impl ColorExt for Color
{
	fn interpolate(&self, other: Color, f: f32) -> Color
	{
		let fi = 1. - f;
		let (r, g, b, a) = self.to_rgba_f();
		let (or, og, ob, oa) = other.to_rgba_f();
		Color::from_rgba_f(
			r * fi + or * f,
			g * fi + og * f,
			b * fi + ob * f,
			a * fi + oa * f,
		)
	}
}

pub fn max<T: PartialOrd>(x: T, y: T) -> T
{
	if x > y
	{
		x
	}
	else
	{
		y
	}
}

pub fn min<T: PartialOrd>(x: T, y: T) -> T
{
	if x < y
	{
		x
	}
	else
	{
		y
	}
}

pub fn clamp<T: PartialOrd>(x: T, min: T, max: T) -> T
{
	if x < min
	{
		min
	}
	else if x > max
	{
		max
	}
	else
	{
		x
	}
}

pub fn round_point(point: Point2<f32>) -> Point2<f32>
{
	Point2::new(point.x.round(), point.y.round())
}

pub trait XYExt
{
	fn set_xy(&mut self, xy: impl std::ops::Index<usize, Output = f32>);
	fn add_xy(&mut self, xy: impl std::ops::Index<usize, Output = f32>);
}

impl XYExt for Point3<f32>
{
	fn set_xy(&mut self, xy: impl std::ops::Index<usize, Output = f32>)
	{
		self.x = xy[0];
		self.y = xy[1];
	}

	fn add_xy(&mut self, xy: impl std::ops::Index<usize, Output = f32>)
	{
		self.x += xy[0];
		self.y += xy[1];
	}
}

impl XYExt for Vector3<f32>
{
	fn set_xy(&mut self, xy: impl std::ops::Index<usize, Output = f32>)
	{
		self.x = xy[0];
		self.y = xy[1];
	}

	fn add_xy(&mut self, xy: impl std::ops::Index<usize, Output = f32>)
	{
		self.x += xy[0];
		self.y += xy[1];
	}
}

pub fn sigmoid(x: f32) -> f32
{
	1. / (1. + (-x).exp())
}

pub fn read_to_string(path: &str) -> Result<String>
{
	fs::read_to_string(path)
		.map_err(|e| Error::new(format!("Couldn't read '{}'", path), Some(Box::new(e))))
}

pub fn load_config<T: DeserializeOwned + Clone>(file: &str) -> Result<T>
{
	let contents = read_to_string(file)?;
	let mut source = Source::new(path::Path::new(file), &contents);
	let element = ConfigElement::from_source(&mut source)
		.map_err(|e| Error::new(format!("Config parsing error"), Some(Box::new(e))))?;
	from_element::<T>(&element, Some(&source))
		.map_err(|e| Error::new(format!("Config parsing error"), Some(Box::new(e))))
}

pub fn save_config<T: Serialize>(file: &str, val: T) -> Result<()>
{
	let element = to_element(&val)
		.map_err(|e| Error::new(format!("Config writing error"), Some(Box::new(e))))?;
	std::fs::write(file, format!("{}", element))
		.map_err(|e| Error::new(format!("Couldn't write '{}'", file), Some(Box::new(e))))?;
	Ok(())
}

pub fn user_data_path(core: &Core) -> Result<path::PathBuf>
{
	let mut path_buf = path::PathBuf::new();
	if cfg!(feature = "use_user_settings")
	{
		path_buf.push(
			core.get_standard_path(StandardPath::UserSettings)
				.map_err(|_| "Couldn't get standard path".to_string())?,
		);
	}
	Ok(path_buf)
}

pub fn load_user_data<T: DeserializeOwned + Clone>(core: &Core, filename: &str)
	-> Result<Option<T>>
{
	let mut path_buf = user_data_path(core)?;
	path_buf.push(filename);
	if path_buf.exists()
	{
		Ok(Some(load_config(path_buf.to_str().unwrap())?))
	}
	else
	{
		Ok(None)
	}
}

pub fn save_user_data<T: Serialize>(core: &Core, filename: &str, options: &T) -> Result<()>
{
	let mut path_buf = user_data_path(core)?;
	std::fs::create_dir_all(&path_buf).map_err(|_| "Couldn't create directory".to_string())?;
	path_buf.push(filename);
	save_config(path_buf.to_str().unwrap(), &options)
}

pub fn load_bitmap(core: &Core, file: &str) -> Result<Bitmap>
{
	Ok(Bitmap::load(&core, file).map_err(|_| format!("Couldn't load {}", file))?)
}

pub fn load_bitmap_indexed(core: &Core, file: &str) -> Result<Bitmap>
{
	Ok(Bitmap::load_indexed(&core, file).map_err(|_| format!("Couldn't load {}", file))?)
}

pub fn load_sample(audio: &AudioAddon, path: &str) -> Result<Sample>
{
	Ok(Sample::load(audio, path).map_err(|_| format!("Couldn't load '{}'", path))?)
}

pub fn load_ttf_font(ttf: &TtfAddon, file: &str, size: i32) -> Result<Font>
{
	Ok(ttf
		.load_ttf_font(file, size, Flag::zero())
		.map_err(|_| format!("Couldn't load {}", file))?)
}

fn process_shader_source(source: String) -> String
{
	let replacements = [
		("STATIC_MATERIAL", "0"),
		("DYNAMIC_MATERIAL", "1"),
		("FULLBRIGHT_MATERIAL", "2"),
	];

	let mut ret = source;
	for (from, to) in replacements
	{
		ret = ret.replace(from, to);
	}
	ret
}

pub fn load_shader(disp: &mut Display, path: &str) -> Result<Shader>
{
	let shader = Shader::new(disp, ShaderPlatform::GLSL).unwrap();
	let vertex_path = format!("{path}_vertex.glsl");
	let pixel_path = format!("{path}_pixel.glsl");

	let mut vertex_source = read_to_string(&vertex_path)?;
	let mut pixel_source = read_to_string(&pixel_path)?;

	vertex_source = process_shader_source(vertex_source);
	pixel_source = process_shader_source(pixel_source);

	shader
		.attach_shader_source(ShaderType::Vertex, Some(&vertex_source))
		.map_err(|e| format!("Error attaching {vertex_path}\n{e}"))?;
	shader
		.attach_shader_source(ShaderType::Pixel, Some(&pixel_source))
		.map_err(|e| format!("Error attaching {pixel_path}\n{e}"))?;
	shader
		.build()
		.map_err(|e| format!("Error compiling shader {path}\n{e}"))?;
	Ok(shader)
}

pub fn nearest_line_point(v1: Point2<f32>, v2: Point2<f32>, test_point: Point2<f32>)
	-> Point2<f32>
{
	let v1_t = test_point - v1;

	let v2_v1 = v2 - v1;
	let v2_v1_norm_sq = max(v2_v1.norm_squared(), 1e-20);

	let dot = v1_t.dot(&v2_v1) / v2_v1_norm_sq;

	if dot < 0.
	{
		v1
	}
	else if dot > 1.
	{
		v2
	}
	else
	{
		v1 + dot * (v2 - v1)
	}
}

pub fn nearest_poly_point(vs: &[Point2<f32>], test_point: Point2<f32>) -> Point2<f32>
{
	assert!(vs.len() >= 3);
	let mut best_dist_sq = f32::INFINITY;
	let mut best_point = Point2::new(0., 0.);

	for idx in 0..vs.len()
	{
		let v1 = vs[idx];
		let v2 = vs[(idx + 1) % vs.len()];

		let cand_point = nearest_line_point(v1, v2, test_point);
		let cand_dist_sq = (cand_point - test_point).norm_squared();
		if cand_dist_sq < best_dist_sq
		{
			best_point = cand_point;
			best_dist_sq = cand_dist_sq;
		}
	}
	best_point
}

pub fn is_inside_poly(vs: &[Point2<f32>], test_point: Point2<f32>) -> bool
{
	// Clockwise.
	assert!(vs.len() >= 3);
	let mut inside = true;

	for idx in 0..vs.len()
	{
		let v1 = vs[idx];
		let v2 = vs[(idx + 1) % vs.len()];
		let v1_v2 = v2 - v1;
		let normal = Vector2::new(-v1_v2.y, v1_v2.x);

		let v1_t = test_point - v1;
		inside &= v1_t.dot(&normal) < 0.;
		if !inside
		{
			return false;
		}
	}
	true
}

// Stolen from ncollide.
pub fn intersect_segment_segment(
	start1: Point2<f32>, end1: Point2<f32>, start2: Point2<f32>, end2: Point2<f32>,
) -> bool
{
	// Inspired by RealField-time collision detection by Christer Ericson.
	let eps = 1e-3;
	let d1 = end1 - start1;
	let d2 = end2 - start2;
	let r = start1 - start2;

	let a = d1.norm_squared();
	let e = d2.norm_squared();
	let f = d2.dot(&r);

	let mut s;
	let mut t;
	let parallel;

	if a <= eps && e <= eps
	{
		s = 0.;
		t = 0.;
	}
	else if a <= eps
	{
		s = 0.;
		t = clamp(f / e, 0., 1.);
	}
	else
	{
		let c = d1.dot(&r);
		if e <= eps
		{
			t = 0.;
			s = clamp(-c / a, 0., 1.);
		}
		else
		{
			let b = d1.dot(&d2);
			let ae = a * e;
			let bb = b * b;
			let denom = ae - bb;

			parallel = denom <= eps || (ae / bb - 1.).abs() < eps;

			// Use absolute and ulps error to test collinearity.
			if !parallel
			{
				s = clamp((b * f - c * e) / denom, 0., 1.);
			}
			else
			{
				s = 0.;
			}

			t = (b * s + f) / e;

			if t < 0.
			{
				t = 0.;
				s = clamp(-c / a, 0., 1.);
			}
			else if t > 1.
			{
				t = 1.;
				s = clamp((b - c) / a, 0., 1.);
			}
		}
	}

	let v1 = start1 + d1 * s;
	let v2 = start2 + d2 * t;

	//~ dbg!(v1, v2, (v1 - v2).norm_squared());

	(v1 - v2).norm_squared() < eps
}

pub fn draw_text(
	core: &Core, font: &Font, color: Color, x: f32, y: f32, align: FontAlign, text: &str,
) -> f32
{
	core.draw_text(font, color, x, y, align, text);
	font.get_text_width(text) as f32
}

pub fn nice_float(f: f32, max_frac_digits: usize) -> String
{
	let res = format!("{:.1$}", f, max_frac_digits);
	let res = if res.contains('.')
	{
		res.trim_end_matches('0').trim_end_matches('.').to_string()
	}
	else
	{
		res
	};
	if res.is_empty()
	{
		return "0".into();
	}
	else
	{
		return res;
	}
}

#[test]
fn nice_float_test()
{
	assert_eq!("15", nice_float(15., 2));
	assert_eq!("0", nice_float(0., 2));
	assert_eq!("0.1", nice_float(0.1, 2));
	assert_eq!("0.11", nice_float(0.11, 2));
	assert_eq!("-0.11", nice_float(-0.11, 2));
	assert_eq!("0.5", nice_float(0.499999, 2));
	assert_eq!("0.04", nice_float(0.04, 2));
	assert_eq!("0.02", nice_float(0.019, 2));
	assert_eq!("0.01", nice_float(0.009, 2));
	assert_eq!("1.8", nice_float(100. * 0.018000001, 2));
	assert_eq!("121", nice_float(120.99999, 2));
	assert_eq!("100", nice_float(100., 2));
	assert_eq!("100.1", nice_float(100.1, 2));
}

#[test]
fn test_nearest_line_point()
{
	let v1 = Point2::new(1., 2.);
	let v2 = Point2::new(3., 4.);

	let t = Point2::new(-1., -1.);
	let n = nearest_line_point(v1, v2, t);
	assert!(n == v1);

	let t = Point2::new(5., 5.);
	let n = nearest_line_point(v1, v2, t);
	assert!((n - v2).norm() < 1e-3);

	let t = Point2::new(2., 3.);
	let n = nearest_line_point(v1, v2, t);
	dbg!(n, t);
	assert!((n - t).norm() < 1e-3);

	let t = Point2::new(1., 4.);
	let n = nearest_line_point(v1, v2, t);
	assert!((n - Point2::new(2., 3.)).norm() < 1e-3);
}

#[test]
fn test_is_inside_poly()
{
	let vs = [
		Point2::new(0., 0.),
		Point2::new(0., 3.),
		Point2::new(3., 3.),
		Point2::new(3., 0.),
	];

	assert!(is_inside_poly(&vs, Point2::new(1., 1.)));
	assert!(!is_inside_poly(&vs, Point2::new(-1., -1.)));

	let vs = [
		Point2::new(-1., 1.),
		Point2::new(1., 3.),
		Point2::new(4., -3.),
		Point2::new(-1., -1.),
	];
	assert!(is_inside_poly(&vs, Point2::new(0., 0.)));
}

#[test]
fn test_segment_segment()
{
	let start1 = Point2::new(0., 64.);
	let end1 = Point2::new(0., 128.);

	let start2 = Point2::new(0., 256.);
	let end2 = Point2::new(0., 120.);

	assert!(intersect_segment_segment(start1, end1, start2, end2));
}

#[test]
fn test_segment_segment2()
{
	let start1 = Point2::new(0., 256.);
	let end1 = Point2::new(15., 65.);

	let start2 = Point2::new(-16., 208.);
	let end2 = Point2::new(16., 208.);

	assert!(intersect_segment_segment(start1, end1, start2, end2));
}
