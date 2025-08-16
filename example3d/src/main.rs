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

use crate::error::{Result, ResultHelper};
use allegro::*;
use allegro_dialog::*;
use allegro_sys::*;
use rand::prelude::*;
use serde_derive::{Deserialize, Serialize};
use slhack::game_loop::{GameState as _GameState, Screen as _Screen};
use slhack::{deferred, game_loop, hack_state, utils};
use std::rc::Rc;

pub enum Screen
{
	Game(game::Game),
	Menu(menu::Menu),
}

impl game_loop::Screen<game_state::GameState> for Screen
{
	type NextScreen = game_state::NextScreen;
	const INIT_SCREEN: Self::NextScreen = game_state::NextScreen::Menu;

	fn new(
		next_screen: Self::NextScreen, game_state: &mut game_state::GameState,
	) -> slhack::error::Result<Option<Self>>
	{
		match next_screen
		{
			game_state::NextScreen::Game => Ok(Some(Screen::Game(
				game::Game::new(game_state).into_slhack()?,
			))),
			game_state::NextScreen::Menu => Ok(Some(Screen::Menu(
				menu::Menu::new(game_state).into_slhack()?,
			))),
			game_state::NextScreen::Quit => Ok(None),
			_ => panic!("Unknown next screen {:?}", next_screen),
		}
	}

	fn draw(&mut self, game_state: &mut game_state::GameState) -> slhack::error::Result<()>
	{
		match self
		{
			Screen::Menu(menu) => menu.draw(game_state),
			Screen::Game(game) => game.draw(game_state),
		}
		.into_slhack()
	}

	fn input(
		&mut self, event: &Event, game_state: &mut game_state::GameState,
	) -> slhack::error::Result<Option<Self::NextScreen>>
	{
		match self
		{
			Screen::Menu(menu) => menu.input(event, game_state),
			Screen::Game(game) => game.input(event, game_state),
		}
		.into_slhack()
	}

	fn logic(
		&mut self, game_state: &mut game_state::GameState,
	) -> slhack::error::Result<Option<Self::NextScreen>>
	{
		match self
		{
			Screen::Game(game) => game.logic(game_state).into_slhack(),
			_ => Ok(None),
		}
	}

	fn resize(&mut self, game_state: &mut game_state::GameState) -> slhack::error::Result<()>
	{
		match self
		{
			Screen::Menu(menu) => Ok(menu.resize(game_state)),
			Screen::Game(game) => Ok(game.resize(game_state)),
		}
	}
}

impl game_loop::GameState for game_state::GameState
{
	type ScreenT = Screen;

	fn hs(&mut self) -> &mut hack_state::HackState
	{
		&mut self.hs
	}

	fn gfx_options(&self) -> &hack_state::GfxOptions
	{
		&self.options.gfx
	}

	fn resize_display(&mut self) -> slhack::error::Result<()>
	{
		if self.basic_shader.is_none()
		{
			self.basic_shader = Some(utils::load_shader(self.hs.display_mut(), "data/basic")?);
			self.forward_shader = Some(utils::load_shader(self.hs.display_mut(), "data/forward")?);
			self.light_shader = Some(utils::load_shader(self.hs.display_mut(), "data/light")?);
			self.final_shader = Some(utils::load_shader(self.hs.display_mut(), "data/final")?);
			let deferred_renderer = {
				let hs = &mut self.hs;
				let (width, height) = hs.fixed_buffer_size.unwrap();
				deferred::DeferredRenderer::new(
					hs.display.as_mut().unwrap(),
					&hs.prim,
					width,
					height,
				)?
			};
			self.deferred_renderer = Some(deferred_renderer);
		}
		self.hs.resize_display()
	}

	fn input(&mut self, event: &Event) -> slhack::error::Result<()>
	{
		self.controls.decode_event(event);
		Ok(())
	}

	fn logic(&mut self) -> slhack::error::Result<()>
	{
		self.sfx.update_sounds(&self.hs.core)
	}
}

fn real_main() -> Result<()>
{
	let mut state = game_state::GameState::new()?;
	let options = game_loop::OPTIONS.depth_buffer(true);
	game_loop::game_loop(&mut state, options)?;
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
