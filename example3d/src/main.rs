#![feature(float_next_up_down)]
#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(clippy::all)]

mod components;
mod error;
mod game;
mod game_state;
mod menu;
mod ui;

use crate::error::Result;
use allegro::*;
use allegro_dialog::*;
use allegro_sys::*;
use game_state::NextScreen;
use rand::prelude::*;
use serde_derive::{Deserialize, Serialize};
use slhack::utils;
use std::rc::Rc;

trait Screen
{
	type NextScreen;
	type ScreenT;
	const INIT_SCREEN: Self::NextScreen;

	fn new(
		next_screen: Self::NextScreen, game_state: &mut game_state::GameState,
	) -> Result<Option<Self::ScreenT>>;
	fn draw(&mut self, game_state: &mut game_state::GameState) -> Result<()>;
	fn input(
		&mut self, event: &Event, game_state: &mut game_state::GameState,
	) -> Result<Option<Self::NextScreen>>;
	fn logic(&mut self, game_state: &mut game_state::GameState)
	-> Result<Option<Self::NextScreen>>;
	fn resize(&mut self, game_state: &mut game_state::GameState) -> Result<()>;
}

enum Screen2
{
	Game(game::Game),
	Menu(menu::Menu),
}

impl Screen for Screen2
{
	type ScreenT = Screen2;
	type NextScreen = game_state::NextScreen;
	const INIT_SCREEN: Self::NextScreen = game_state::NextScreen::Menu;

	fn new(
		next_screen: Self::NextScreen, game_state: &mut game_state::GameState,
	) -> Result<Option<Self>>
	{
		match next_screen
		{
			game_state::NextScreen::Game => Ok(Some(Screen2::Game(game::Game::new(game_state)?))),
			game_state::NextScreen::Menu => Ok(Some(Screen2::Menu(menu::Menu::new(game_state)?))),
			game_state::NextScreen::Quit => Ok(None),
			_ => panic!("Unknown next screen {:?}", next_screen),
		}
	}

	fn draw(&mut self, game_state: &mut game_state::GameState) -> Result<()>
	{
		match self
		{
			Screen2::Menu(menu) => menu.draw(game_state),
			Screen2::Game(game) => game.draw(game_state),
		}
	}

	fn input(
		&mut self, event: &Event, game_state: &mut game_state::GameState,
	) -> Result<Option<Self::NextScreen>>
	{
		match self
		{
			Screen2::Menu(menu) => menu.input(event, game_state),
			Screen2::Game(game) => game.input(event, game_state),
		}
	}

	fn logic(&mut self, game_state: &mut game_state::GameState)
	-> Result<Option<Self::NextScreen>>
	{
		match self
		{
			Screen2::Game(game) => game.logic(game_state),
			_ => Ok(None),
		}
	}

	fn resize(&mut self, game_state: &mut game_state::GameState) -> Result<()>
	{
		match self
		{
			Screen2::Menu(menu) => Ok(menu.resize(game_state)),
			Screen2::Game(game) => Ok(game.resize(game_state)),
		}
	}
}

fn real_main() -> Result<()>
{
	let mut state = game_state::GameState::new()?;

	let mut flags = OPENGL | OPENGL_3_0 | PROGRAMMABLE_PIPELINE;

	if state.options.gfx.fullscreen
	{
		flags = flags | FULLSCREEN_WINDOW;
	}
	state.hs.core.set_new_display_flags(flags);

	if state.options.gfx.vsync_method == 1
	{
		state.hs.core.set_new_display_option(
			DisplayOption::Vsync,
			1,
			DisplayOptionImportance::Suggest,
		);
	}
	state.hs.core.set_new_display_option(
		DisplayOption::DepthSize,
		16,
		DisplayOptionImportance::Suggest,
	);
	state.hs.set_display(
		Display::new(
			&state.hs.core,
			state.options.gfx.width,
			state.options.gfx.height,
		)
		.map_err(|_| "Couldn't create display".to_string())?,
	);

	gl_loader::init_gl();
	gl::load_with(|symbol| gl_loader::get_proc_address(symbol) as *const _);

	let scale_shader = utils::load_shader(state.hs.display_mut(), "data/scale")?;
	state.basic_shader = Some(utils::load_shader(state.hs.display_mut(), "data/basic")?);
	state.forward_shader = Some(utils::load_shader(state.hs.display_mut(), "data/forward")?);
	state.light_shader = Some(utils::load_shader(state.hs.display_mut(), "data/light")?);
	state.final_shader = Some(utils::load_shader(state.hs.display_mut(), "data/final")?);
	state.resize_display()?;

	let timer = Timer::new(&state.hs.core, utils::DT as f64)
		.map_err(|_| "Couldn't create timer".to_string())?;

	let queue =
		EventQueue::new(&state.hs.core).map_err(|_| "Couldn't create event queue".to_string())?;
	queue.register_event_source(state.hs.display().get_event_source());
	queue.register_event_source(
		state
			.hs
			.core
			.get_keyboard_event_source()
			.expect("Couldn't get keyboard"),
	);
	queue.register_event_source(
		state
			.hs
			.core
			.get_mouse_event_source()
			.expect("Couldn't get mouse"),
	);
	queue.register_event_source(timer.get_event_source());
	queue.register_event_source(
		state
			.hs
			.core
			.get_joystick_event_source()
			.expect("Couldn't get joystick"),
	);

	let mut quit = false;

	let mut logics_without_draw = 0;
	let mut old_mouse_hide = state.hs.hide_mouse;
	let mut old_fullscreen = state.options.gfx.fullscreen;
	let mut old_ui_scale = state.options.gfx.ui_scale;
	let mut old_frac_scale = state.options.gfx.frac_scale;
	let mut switched_in = true;

	let prev_frame_start = state.hs.core.get_time();
	let mut logic_end = prev_frame_start;
	let mut frame_count = 0;
	if state.options.gfx.grab_mouse
	{
		state.hs.core.grab_mouse(state.hs.display()).ok();
	}

	type ScreenT = Screen2;
	let mut cur_screen = ScreenT::new(ScreenT::INIT_SCREEN, &mut state)?.unwrap();
	state.hs.hide_mouse = true;

	timer.start();
	while !quit
	{
		if queue.is_empty()
		{
			if state.hs.display_width != state.hs.display().get_width() as f32
				|| state.hs.display_height != state.hs.display().get_height() as f32
				|| old_ui_scale != state.options.gfx.ui_scale
				|| old_frac_scale != state.options.gfx.frac_scale
			{
				old_ui_scale = state.options.gfx.ui_scale;
				old_frac_scale = state.options.gfx.frac_scale;
				state.resize_display()?;
				cur_screen.resize(&mut state)?;
			}

			let frame_start = state.hs.core.get_time();
			state.hs.core.set_target_bitmap(Some(state.hs.buffer1()));
			state.hs.alpha = (frame_start - logic_end) as f32 / utils::DT;

			cur_screen.draw(&mut state)?;

			if state.options.gfx.vsync_method == 2
			{
				state.hs.core.wait_for_vsync().ok();
			}

			state.hs.core.set_target_bitmap(Some(state.hs.buffer2()));

			state
				.hs
				.core
				.use_shader(state.basic_shader.as_ref())
				.unwrap();

			state
				.hs
				.core
				.draw_bitmap(state.hs.buffer1(), 0., 0., Flag::zero());

			state
				.hs
				.core
				.set_target_bitmap(Some(state.hs.display().get_backbuffer()));

			let bw = state.hs.buffer_width() as f32;
			let bh = state.hs.buffer_height() as f32;
			let dw = state.hs.display().get_width() as f32;
			let dh = state.hs.display().get_height() as f32;

			state.hs.core.use_shader(Some(&scale_shader)).unwrap();
			state
				.hs
				.core
				.set_shader_uniform("bitmap_width", &[bw][..])
				.ok();
			state
				.hs
				.core
				.set_shader_uniform("bitmap_height", &[bh][..])
				.ok();
			state
				.hs
				.core
				.set_shader_uniform("scale", &[state.hs.draw_scale][..])
				.ok();

			state.hs.core.clear_to_color(Color::from_rgb_f(0., 0., 0.));

			state.hs.core.draw_scaled_bitmap(
				state.hs.buffer2(),
				0.,
				0.,
				bw,
				bh,
				(dw / 2. - bw / 2. * state.hs.draw_scale).floor(),
				(dh / 2. - bh / 2. * state.hs.draw_scale).floor(),
				bw * state.hs.draw_scale,
				bh * state.hs.draw_scale,
				Flag::zero(),
			);

			state.hs.core.flip_display();

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
		state.controls.decode_event(&event);
		state.hs.game_ui_controls.decode_event(&event);
		state.hs.menu_controls.decode_event(&event);
		let mut next_screen = cur_screen.input(&event, &mut state)?;
		state.hs.game_ui_controls.clear_action_states();
		state.hs.menu_controls.clear_action_states();

		match event
		{
			Event::DisplayClose { .. } => quit = true,
			Event::DisplayResize { .. } =>
			{
				state
					.hs
					.display()
					.acknowledge_resize()
					.map_err(|_| "Couldn't acknowledge resize".to_string())?;
			}
			Event::DisplaySwitchIn { .. } =>
			{
				if state.options.gfx.grab_mouse
				{
					state.hs.core.grab_mouse(state.hs.display()).ok();
				}
				state.hs.track_mouse = true;
			}
			Event::DisplaySwitchOut { .. } =>
			{
				if state.options.gfx.grab_mouse
				{
					state.hs.core.ungrab_mouse().ok();
				}
				state.hs.track_mouse = false;
				switched_in = false;
				state
					.hs
					.display()
					.show_cursor(true)
					.map_err(|_| "Could not hide cursor.".to_string())?;
			}
			Event::MouseButtonDown { .. } =>
			{
				if state.options.gfx.grab_mouse
				{
					state.hs.core.grab_mouse(state.hs.display()).ok();
				}
				state.hs.track_mouse = true;
				if !switched_in
				{
					state
						.hs
						.display()
						.show_cursor(!state.hs.hide_mouse)
						.map_err(|_| "Could not hide cursor.".to_string())?;
				}
				switched_in = true;
			}
			Event::JoystickConfiguration { .. } =>
			{
				state
					.hs
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

				if next_screen.is_none()
				{
					next_screen = cur_screen.logic(&mut state)?;
				}

				if state.hs.hide_mouse && switched_in
				{
					state
						.hs
						.core
						.set_mouse_xy(
							state.hs.display(),
							state.hs.display().get_width() / 2,
							state.hs.display().get_height() / 2,
						)
						.map_err(|_| "Couldn't set mouse position".to_string())?;
				}

				if old_mouse_hide != state.hs.hide_mouse && switched_in
				{
					old_mouse_hide = state.hs.hide_mouse;
					state
						.hs
						.display()
						.show_cursor(!state.hs.hide_mouse)
						.map_err(|_| "Could not hide cursor.".to_string())?;
				}

				if old_fullscreen != state.options.gfx.fullscreen
				{
					state
						.hs
						.display()
						.set_flag(FULLSCREEN_WINDOW, state.options.gfx.fullscreen);
					old_fullscreen = state.options.gfx.fullscreen;
				}

				logics_without_draw += 1;
				state.sfx.update_sounds(&state.hs.core)?;

				if !state.hs.paused
				{
					state.hs.tick += 1;
				}
				logic_end = state.hs.core.get_time();
			}
			_ => (),
		}

		if let Some(next_screen) = next_screen
		{
			if let Some(new_screen) = ScreenT::new(next_screen, &mut state)?
			{
				cur_screen = new_screen;
			}
			else
			{
				quit = true;
			}
		}
	}
	state.sfx.fade_out(&state.hs.core);

	Ok(())
}

allegro_main! {
	use std::panic::catch_unwind;

	match catch_unwind(|| real_main().unwrap())
	{
		Err(e) =>
		{
			let err: String = e
				.downcast_ref::<&'static str>()
				.map(|&e| e.to_owned())
				.or_else(|| e.downcast_ref::<String>().map(|e| e.clone()))
				.unwrap_or("Unknown error!".to_owned());

			let mut lines = vec![];
			for line in err.lines().take(10)
			{
				lines.push(line.to_string());
			}
			show_native_message_box(
				None,
				"Error!",
				"An error has occurred!",
				&lines.join("\n"),
				Some("You make me sad."),
				MESSAGEBOX_ERROR,
			);
		}
		Ok(_) => (),
	}
}
