use crate::error::Result;
use crate::{controls, ui, utils};

use allegro::*;
use allegro_font::*;
use allegro_image::*;
use allegro_primitives::*;
use allegro_ttf::*;
use nalgebra::Point2;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GfxOptions
{
	pub fullscreen: bool,
	pub width: i32,
	pub height: i32,
	pub vsync_method: i32,
	pub grab_mouse: bool,
	pub ui_scale: f32,
	pub frac_scale: bool,
}

pub struct HackState
{
	pub core: Core,
	pub prim: PrimitivesAddon,
	pub image: ImageAddon,
	pub font: FontAddon,
	pub ttf: TtfAddon,
	pub tick: i64,
	pub paused: bool,

	pub ui_font: Option<Font>,
	pub game_ui_controls: controls::ControlsHandler<ui::UIAction>,
	pub menu_controls: controls::ControlsHandler<ui::UIAction>,
	pub track_mouse: bool,
	pub hide_mouse: bool,
	pub mouse_pos: Point2<i32>,

	pub fixed_buffer_size: Option<(i32, i32)>,
	pub draw_scale: f32,
	pub display_width: f32,
	pub display_height: f32,
	pub buffer1: Option<Bitmap>,
	pub buffer2: Option<Bitmap>,

	// TODO: GET RID OF THIS
	pub gfx_options: GfxOptions,

	pub alpha: f32,
	pub time: f64,

	// Has to be last!
	pub display: Option<Display>,
}

impl HackState
{
	pub fn new(
		name: &str, mut load_options: impl FnMut(&Core) -> Result<GfxOptions>,
		fixed_buffer_size: Option<(i32, i32)>,
	) -> Result<Self>
	{
		let core = Core::init()?;
		core.set_app_name(name);
		core.set_org_name("SiegeLord");

		let gfx_options = load_options(&core)?;
		let prim = PrimitivesAddon::init(&core)?;
		let image = ImageAddon::init(&core)?;
		let font = FontAddon::init(&core)?;
		let ttf = TtfAddon::init(&font)?;
		core.install_keyboard()
			.map_err(|_| "Couldn't install keyboard".to_string())?;
		core.install_mouse()
			.map_err(|_| "Couldn't install mouse".to_string())?;
		core.set_joystick_mappings("data/gamecontrollerdb.txt")
			.map_err(|_| "Couldn't set joystick mappings".to_string())?;
		core.install_joystick()
			.map_err(|_| "Couldn't install joysticks".to_string())?;

		Ok(Self {
			fixed_buffer_size: fixed_buffer_size,
			gfx_options: gfx_options,
			core: core,
			prim: prim,
			image: image,
			tick: 0,
			time: 0.0,
			font: font,
			ttf: ttf,
			paused: false,
			ui_font: None,
			draw_scale: 1.,
			display_width: 0.,
			display_height: 0.,
			buffer1: None,
			buffer2: None,
			game_ui_controls: controls::ControlsHandler::new(ui::new_game_ui_controls()),
			menu_controls: controls::ControlsHandler::new(ui::new_menu_controls()),
			track_mouse: true,
			hide_mouse: false,
			mouse_pos: Point2::new(0, 0),
			display: None,
			alpha: 0.,
		})
	}

	pub fn buffer1(&self) -> &Bitmap
	{
		self.buffer1.as_ref().unwrap()
	}

	pub fn buffer2(&self) -> &Bitmap
	{
		self.buffer2.as_ref().unwrap()
	}

	pub fn buffer_width(&self) -> f32
	{
		if let Some(buffer) = self.buffer1.as_ref()
		{
			buffer.get_width() as f32
		}
		else
		{
			self.display.as_ref().unwrap().get_width() as f32
		}
	}

	pub fn buffer_height(&self) -> f32
	{
		if let Some(buffer) = self.buffer1.as_ref()
		{
			buffer.get_height() as f32
		}
		else
		{
			self.display.as_ref().unwrap().get_height() as f32
		}
	}

	pub fn ui_font(&self) -> &Font
	{
		self.ui_font.as_ref().unwrap()
	}

	pub fn resize_display(&mut self, gfx_options: &GfxOptions) -> Result<()>
	{
		let buffer_width;
		let buffer_height;
		let display = self.display.as_ref().unwrap();
		if let Some((width, height)) = self.fixed_buffer_size
		{
			buffer_width = width;
			buffer_height = height;
		}
		else
		{
			buffer_width = display.get_width();
			buffer_height = display.get_height();
		}

		self.display_width = display.get_width() as f32;
		self.display_height = display.get_height() as f32;
		self.draw_scale = utils::min(
			(display.get_width() as f32) / (buffer_width as f32),
			(display.get_height() as f32) / (buffer_height as f32),
		);
		if !gfx_options.frac_scale
		{
			self.draw_scale = self.draw_scale.floor();
		}

		if self.buffer1.is_none() || self.fixed_buffer_size.is_none()
		{
			let old_depth = self.core.get_new_bitmap_depth();
			self.core.set_new_bitmap_depth(16);
			self.buffer1 = Some(Bitmap::new(&self.core, buffer_width, buffer_height).unwrap());
			self.buffer2 = Some(Bitmap::new(&self.core, buffer_width, buffer_height).unwrap());
			self.core.set_new_bitmap_depth(old_depth);
		}

		self.ui_font = Some(utils::load_ttf_font(
			&self.ttf,
			"data/Energon.ttf",
			(-24. * gfx_options.ui_scale) as i32,
		)?);
		Ok(())
	}

	pub fn transform_mouse(&self, x: f32, y: f32) -> (f32, f32)
	{
		let x = (x - self.display_width / 2.) / self.draw_scale + self.buffer_width() / 2.;
		let y = (y - self.display_height / 2.) / self.draw_scale + self.buffer_height() / 2.;
		(x, y)
	}

	pub fn time(&self) -> f64
	{
		self.time
	}

	pub fn set_display(&mut self, display: Display)
	{
		self.display = Some(display);
	}

	pub fn display(&self) -> &Display
	{
		self.display.as_ref().unwrap()
	}

	pub fn display_mut(&mut self) -> &mut Display
	{
		self.display.as_mut().unwrap()
	}
}
