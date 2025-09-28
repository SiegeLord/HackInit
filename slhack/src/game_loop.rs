use crate::error::Result;
use crate::{hack_state, utils};

use allegro::*;

pub trait LoopState: Sized
{
	fn hs(&mut self) -> &mut hack_state::HackState;
	fn gfx_options(&self) -> &hack_state::GfxOptions;
	fn resize_display(&mut self) -> Result<()>;

	fn init(&mut self) -> Result<()>
	{
		Ok(())
	}
	fn draw(&mut self) -> Result<()>
	{
		Ok(())
	}
	fn input(&mut self, _event: &Event) -> Result<()>
	{
		Ok(())
	}
	fn logic(&mut self) -> Result<()>
	{
		Ok(())
	}
	fn step(&mut self) -> Result<bool>
	{
		Ok(false)
	}
}

#[derive(Copy, Clone, Debug)]
pub struct Options
{
	pub depth_buffer: bool,
	pub dt: f64,
}

impl Options
{
	pub fn new() -> Self
	{
		Self {
			depth_buffer: false,
			dt: 1.0 / 60.0,
		}
	}
}

pub fn game_loop<LoopStateT: LoopState>(state: &mut LoopStateT, options: Options) -> Result<()>
{
	let mut flags = OPENGL | OPENGL_3_0 | PROGRAMMABLE_PIPELINE;

	if state.gfx_options().fullscreen
	{
		flags = flags | FULLSCREEN_WINDOW;
	}
	state.hs().core.set_new_display_flags(flags);

	if state.gfx_options().vsync_method == 1
	{
		state.hs().core.set_new_display_option(
			DisplayOption::Vsync,
			1,
			DisplayOptionImportance::Suggest,
		);
	}
	if options.depth_buffer
	{
		state.hs().core.set_new_display_option(
			DisplayOption::DepthSize,
			16,
			DisplayOptionImportance::Suggest,
		);
	}
	let width = state.gfx_options().width;
	let height = state.gfx_options().height;
	let display = Display::new(&state.hs().core, width, height)
		.map_err(|_| "Couldn't create display".to_string())?;
	state.hs().set_display(display);

	gl_loader::init_gl();
	gl::load_with(|symbol| gl_loader::get_proc_address(symbol) as *const _);

	let scale_shader = utils::load_shader(state.hs().display_mut(), "data/scale")?;
	state.init()?;

	let timer = Timer::new(&state.hs().core, options.dt)
		.map_err(|_| "Couldn't create timer".to_string())?;

	let queue =
		EventQueue::new(&state.hs().core).map_err(|_| "Couldn't create event queue".to_string())?;
	queue.register_event_source(state.hs().display().get_event_source());
	queue.register_event_source(
		state
			.hs()
			.core
			.get_keyboard_event_source()
			.expect("Couldn't get keyboard"),
	);
	queue.register_event_source(
		state
			.hs()
			.core
			.get_mouse_event_source()
			.expect("Couldn't get mouse"),
	);
	queue.register_event_source(timer.get_event_source());
	queue.register_event_source(
		state
			.hs()
			.core
			.get_joystick_event_source()
			.expect("Couldn't get joystick"),
	);

	let mut quit = false;

	let mut logics_without_draw = 0;
	let mut old_mouse_hide = state.hs().hide_mouse;
	let mut old_fullscreen = state.gfx_options().fullscreen;
	let mut old_ui_scale = state.gfx_options().ui_scale;
	let mut old_frac_scale = state.gfx_options().frac_scale;
	let mut switched_in = true;

	let prev_frame_start = state.hs().core.get_time();
	let mut logic_end = prev_frame_start;
	let mut frame_count = 0;
	if state.gfx_options().grab_mouse
	{
		let hs = state.hs();
		hs.core.grab_mouse(hs.display()).ok();
	}
	if state.hs().hide_mouse
	{
		let hs = state.hs();
		hs.display()
			.show_cursor(!hs.hide_mouse)
			.map_err(|_| "Could not hide cursor.".to_string())?;
	}

	timer.start();
	while !quit
	{
		if queue.is_empty()
		{
			if state.hs().display_width != state.hs().display().get_width() as f32
				|| state.hs().display_height != state.hs().display().get_height() as f32
				|| old_ui_scale != state.gfx_options().ui_scale
				|| old_frac_scale != state.gfx_options().frac_scale
			{
				old_ui_scale = state.gfx_options().ui_scale;
				old_frac_scale = state.gfx_options().frac_scale;
				state.resize_display()?;
			}

			let frame_start = state.hs().core.get_time();
			if state.hs().fixed_buffer_size.is_some()
			{
				let hs = state.hs();
				hs.core.set_target_bitmap(Some(hs.buffer1()));
			}
			else
			{
				let hs = state.hs();
				hs.core
					.set_target_bitmap(Some(hs.display().get_backbuffer()));
			}

			state.hs().alpha = ((frame_start - logic_end) / options.dt) as f32;
			state.draw()?;

			if state.hs().fixed_buffer_size.is_some()
			{
				let hs = state.hs();
				hs.core.set_target_bitmap(Some(hs.buffer2()));

				hs.core.draw_bitmap(hs.buffer1(), 0., 0., Flag::zero());

				hs.core
					.set_target_bitmap(Some(hs.display().get_backbuffer()));

				let bw = hs.buffer_width() as f32;
				let bh = hs.buffer_height() as f32;
				let dw = hs.display().get_width() as f32;
				let dh = hs.display().get_height() as f32;

				hs.core.use_shader(Some(&scale_shader)).unwrap();
				hs.core.set_shader_uniform("bitmap_width", &[bw][..]).ok();
				hs.core.set_shader_uniform("bitmap_height", &[bh][..]).ok();
				hs.core
					.set_shader_uniform("scale", &[hs.draw_scale][..])
					.ok();

				hs.core.clear_to_color(Color::from_rgb_f(0., 0., 0.));

				hs.core.draw_scaled_bitmap(
					hs.buffer2(),
					0.,
					0.,
					bw,
					bh,
					(dw / 2. - bw / 2. * hs.draw_scale).floor(),
					(dh / 2. - bh / 2. * hs.draw_scale).floor(),
					bw * hs.draw_scale,
					bh * hs.draw_scale,
					Flag::zero(),
				);
			}

			if state.gfx_options().vsync_method == 2
			{
				state.hs().core.wait_for_vsync().ok();
			}
			state.hs().core.flip_display();

			if frame_count == 120
			{
				//println!("FPS: {:.2}", 120. / (frame_start - prev_frame_start));
				//prev_frame_start = frame_start;
				frame_count = 0;
			}
			frame_count += 1;
			logics_without_draw = 0;
		}

		let event = queue.get_next_event();
		state.hs().game_ui_controls.decode_event(&event);
		state.hs().menu_controls.decode_event(&event);
		state.input(&event)?;
		state.hs().game_ui_controls.clear_action_states();
		state.hs().menu_controls.clear_action_states();

		match event
		{
			Event::DisplayClose { .. } => quit = true,
			Event::DisplayResize { .. } =>
			{
				state
					.hs()
					.display()
					.acknowledge_resize()
					.map_err(|_| "Couldn't acknowledge resize".to_string())?;
			}
			Event::DisplaySwitchIn { .. } =>
			{
				if state.gfx_options().grab_mouse
				{
					let hs = state.hs();
					hs.core.grab_mouse(hs.display()).ok();
				}
				state.hs().track_mouse = true;
			}
			Event::DisplaySwitchOut { .. } =>
			{
				if state.gfx_options().grab_mouse
				{
					state.hs().core.ungrab_mouse().ok();
				}
				state.hs().track_mouse = false;
				switched_in = false;
				state
					.hs()
					.display()
					.show_cursor(true)
					.map_err(|_| "Could not hide cursor.".to_string())?;
			}
			Event::MouseButtonDown { .. } =>
			{
				if state.gfx_options().grab_mouse
				{
					let hs = state.hs();
					hs.core.grab_mouse(hs.display()).ok();
				}
				let hs = state.hs();
				hs.track_mouse = true;
				if !switched_in
				{
					hs.display()
						.show_cursor(!hs.hide_mouse)
						.map_err(|_| "Could not hide cursor.".to_string())?;
				}
				switched_in = true;
			}
			Event::JoystickConfiguration { .. } =>
			{
				state
					.hs()
					.core
					.reconfigure_joysticks()
					.map_err(|_| "Couldn't reconfigure joysticks".to_string())?;
			}
			Event::TimerTick { .. } =>
			{
				if logics_without_draw > 10
				{
					continue;
				}

				state.logic()?;

				if state.hs().hide_mouse && switched_in
				{
					let hs = state.hs();
					hs.core
						.set_mouse_xy(
							hs.display(),
							hs.display().get_width() / 2,
							hs.display().get_height() / 2,
						)
						.map_err(|_| "Couldn't set mouse position".to_string())?;
				}

				if old_mouse_hide != state.hs().hide_mouse && switched_in
				{
					let hs = state.hs();
					old_mouse_hide = hs.hide_mouse;
					hs.display()
						.show_cursor(!hs.hide_mouse)
						.map_err(|_| "Could not hide cursor.".to_string())?;
				}

				let fullscreen = state.gfx_options().fullscreen;
				if old_fullscreen != fullscreen
				{
					let hs = state.hs();
					hs.display().set_flag(FULLSCREEN_WINDOW, fullscreen);
					old_fullscreen = fullscreen;
				}

				logics_without_draw += 1;

				if !state.hs().paused
				{
					let hs = state.hs();
					hs.tick += 1;
					hs.time = hs.tick as f64 * options.dt;
				}
				logic_end = state.hs().core.get_time();
			}
			_ => (),
		}

		quit |= !state.step()?;
	}
	Ok(())
}
