use crate::{controls, hack_state, sfx, utils};

use allegro::*;
use allegro_font::*;
use nalgebra::{Point2, Vector2};
use serde_derive::{Deserialize, Serialize};

use std::collections::BTreeMap;

pub const UNSELECTED: Color = Color::from_rgb_f(0.9, 0.9, 0.4);
pub const LABEL: Color = Color::from_rgb_f(0.7 * 0.9, 0.7 * 0.9, 0.7 * 0.4);
pub const SELECTED: Color = Color::from_rgb_f(1., 1., 1.);

pub const HORIZ_SPACE: f32 = 16.;
pub const VERT_SPACE: f32 = 16.;
pub const BUTTON_WIDTH: f32 = 128.;
pub const BUTTON_HEIGHT: f32 = 16.;
pub const CONTROL_WIDTH: f32 = 80.;

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Copy, Clone, Debug, PartialOrd, Ord)]
pub enum UIAction
{
	Left,
	Right,
	Up,
	Down,
	Accept,
	Cancel,
}

impl controls::Action for UIAction
{
	fn to_str(&self) -> &'static str
	{
		match self
		{
			UIAction::Left => "Left",
			UIAction::Right => "Right",
			UIAction::Up => "Up",
			UIAction::Down => "Down",
			UIAction::Accept => "Accept",
			UIAction::Cancel => "Cancel",
		}
	}
}

pub fn new_menu_controls() -> controls::Controls<UIAction>
{
	let mut action_to_inputs = BTreeMap::new();
	action_to_inputs.insert(
		UIAction::Up,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Up)),
			Some(controls::Input::JoystickNegAxis(
				allegro::JoystickStick::DPad,
				1,
			)),
		],
	);
	action_to_inputs.insert(
		UIAction::Down,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Down)),
			Some(controls::Input::JoystickPosAxis(
				allegro::JoystickStick::DPad,
				1,
			)),
		],
	);
	action_to_inputs.insert(
		UIAction::Left,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Left)),
			Some(controls::Input::JoystickNegAxis(
				allegro::JoystickStick::DPad,
				0,
			)),
		],
	);
	action_to_inputs.insert(
		UIAction::Right,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Right)),
			Some(controls::Input::JoystickPosAxis(
				allegro::JoystickStick::DPad,
				0,
			)),
		],
	);
	action_to_inputs.insert(
		UIAction::Accept,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Enter)),
			Some(controls::Input::JoystickButton(allegro::JoystickButton::A)),
		],
	);
	action_to_inputs.insert(
		UIAction::Cancel,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Escape)),
			Some(controls::Input::JoystickButton(allegro::JoystickButton::B)),
		],
	);
	controls::Controls::new(action_to_inputs)
}

pub fn new_game_ui_controls() -> controls::Controls<UIAction>
{
	let mut action_to_inputs = BTreeMap::new();
	action_to_inputs.insert(
		UIAction::Up,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Up)),
			Some(controls::Input::JoystickNegAxis(
				allegro::JoystickStick::DPad,
				1,
			)),
		],
	);
	action_to_inputs.insert(
		UIAction::Down,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Down)),
			Some(controls::Input::JoystickPosAxis(
				allegro::JoystickStick::DPad,
				1,
			)),
		],
	);
	action_to_inputs.insert(
		UIAction::Left,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Left)),
			Some(controls::Input::JoystickNegAxis(
				allegro::JoystickStick::DPad,
				0,
			)),
		],
	);
	action_to_inputs.insert(
		UIAction::Right,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Right)),
			Some(controls::Input::JoystickPosAxis(
				allegro::JoystickStick::DPad,
				0,
			)),
		],
	);
	action_to_inputs.insert(
		UIAction::Accept,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Enter)),
			Some(controls::Input::JoystickButton(allegro::JoystickButton::A)),
		],
	);
	action_to_inputs.insert(
		UIAction::Cancel,
		[
			Some(controls::Input::Keyboard(allegro::KeyCode::Escape)),
			Some(controls::Input::JoystickButton(
				allegro::JoystickButton::Start,
			)),
		],
	);

	controls::Controls::new(action_to_inputs)
}

pub trait Action: PartialEq + Eq
{
	const SELECT_ME: Self;
	const BACK: Self;
}

#[derive(Clone)]
pub struct Button<ActionT>
{
	loc: Point2<f32>,
	size: Vector2<f32>,
	text: String,
	action: ActionT,
	selected: bool,
}

impl<ActionT: Action + Clone> Button<ActionT>
{
	pub fn new(w: f32, h: f32, text: &str, action: ActionT) -> Self
	{
		Self {
			loc: Point2::new(0., 0.),
			size: Vector2::new(w, h),
			text: text.into(),
			action: action,
			selected: false,
		}
	}

	pub fn width(&self) -> f32
	{
		self.size.x
	}

	pub fn height(&self) -> f32
	{
		self.size.y
	}

	pub fn draw(&self, state: &hack_state::HackState)
	{
		let c_ui = if self.selected { SELECTED } else { UNSELECTED };

		state.core.draw_text(
			state.ui_font(),
			c_ui,
			self.loc.x.round(),
			(self.loc.y - state.ui_font().get_line_height() as f32 / 2.).round(),
			FontAlign::Centre,
			&self.text,
		);
	}

	pub fn input(
		&mut self, event: &Event, sfx: &mut sfx::Sfx, state: &mut hack_state::HackState,
	) -> Option<ActionT>
	{
		let s = state.gfx_options.ui_scale;
		let start = self.loc - s * self.size / 2.;
		let end = self.loc + s * self.size / 2.;

		if state.menu_controls.get_action_state(UIAction::Accept) > 0.5
		{
			if self.selected
			{
				sfx.play_sound("data/ui2.ogg").unwrap();
				return Some(self.action.clone());
			}
		}
		if state.menu_controls.get_action_state(UIAction::Cancel) > 0.5
		{
			if self.action == ActionT::BACK
			{
				sfx.play_sound("data/ui2.ogg").unwrap();
				return Some(self.action.clone());
			}
		}
		match event
		{
			Event::MouseAxes { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					return Some(ActionT::SELECT_ME);
				}
			}
			Event::MouseButtonUp { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					sfx.play_sound("data/ui2.ogg").unwrap();
					return Some(self.action.clone());
				}
			}
			_ => (),
		}
		None
	}
}

#[derive(Clone)]
pub struct Toggle<ActionT>
{
	loc: Point2<f32>,
	size: Vector2<f32>,
	texts: Vec<String>,
	cur_value: usize,
	action_fn: fn(usize) -> ActionT,
	selected: bool,
}

impl<ActionT: Action> Toggle<ActionT>
{
	pub fn new(
		w: f32, h: f32, cur_value: usize, texts: Vec<String>, action_fn: fn(usize) -> ActionT,
	) -> Self
	{
		Self {
			loc: Point2::new(0., 0.),
			size: Vector2::new(w, h),
			texts: texts,
			cur_value: cur_value,
			action_fn: action_fn,
			selected: false,
		}
	}

	pub fn width(&self) -> f32
	{
		self.size.x
	}

	pub fn height(&self) -> f32
	{
		self.size.y
	}

	pub fn draw(&self, state: &hack_state::HackState)
	{
		let c_ui = if self.selected { SELECTED } else { UNSELECTED };

		state.core.draw_text(
			state.ui_font(),
			c_ui,
			self.loc.x,
			self.loc.y - state.ui_font().get_line_height() as f32 / 2.,
			FontAlign::Centre,
			&self.texts[self.cur_value],
		);
	}

	pub fn input(
		&mut self, event: &Event, sfx: &mut sfx::Sfx, state: &mut hack_state::HackState,
	) -> Option<ActionT>
	{
		let s = state.gfx_options.ui_scale;
		let start = self.loc - s * self.size / 2.;
		let end = self.loc + s * self.size / 2.;
		if state.menu_controls.get_action_state(UIAction::Accept) > 0.5
		{
			if self.selected
			{
				return Some(self.trigger(sfx, state));
			}
		}
		match event
		{
			Event::MouseAxes { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					return Some(ActionT::SELECT_ME);
				}
			}
			Event::MouseButtonUp { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					return Some(self.trigger(sfx, state));
				}
			}
			_ => (),
		}
		None
	}

	fn trigger(&mut self, sfx: &mut sfx::Sfx, _state: &mut hack_state::HackState) -> ActionT
	{
		sfx.play_sound("data/ui2.ogg").unwrap();
		self.cur_value = (self.cur_value + 1) % self.texts.len();
		(self.action_fn)(self.cur_value)
	}
}

#[derive(Clone)]
pub struct Slider<ActionT>
{
	loc: Point2<f32>,
	size: Vector2<f32>,
	cur_pos: f32,
	min_pos: f32,
	max_pos: f32,
	grabbed: bool,
	selected: bool,
	round_to: f32,
	action_fn: fn(f32) -> ActionT,
}

impl<ActionT: Action> Slider<ActionT>
{
	pub fn new(
		w: f32, h: f32, cur_pos: f32, min_pos: f32, max_pos: f32, round_to: f32,
		action_fn: fn(f32) -> ActionT,
	) -> Self
	{
		Self {
			loc: Point2::new(0., 0.),
			size: Vector2::new(w, h),
			cur_pos: cur_pos,
			min_pos: min_pos,
			max_pos: max_pos,
			grabbed: false,
			selected: false,
			round_to: round_to,
			action_fn: action_fn,
		}
	}

	pub fn width(&self) -> f32
	{
		self.size.x
	}

	pub fn height(&self) -> f32
	{
		self.size.y
	}

	fn round_cur_pos(&mut self)
	{
		self.cur_pos = (self.cur_pos / self.round_to).round() * self.round_to;
	}

	pub fn draw(&self, state: &hack_state::HackState)
	{
		let s = state.gfx_options.ui_scale;
		let c_ui = if self.selected { SELECTED } else { UNSELECTED };

		let w = s * self.width();
		let cursor_x =
			self.loc.x - w / 2. + w * (self.cur_pos - self.min_pos) / (self.max_pos - self.min_pos);
		let start_x = self.loc.x - w / 2.;
		let end_x = self.loc.x + w / 2.;

		let ww = s * HORIZ_SPACE;
		if cursor_x - start_x > ww
		{
			state
				.prim
				.draw_line(start_x, self.loc.y, cursor_x - ww, self.loc.y, c_ui, s * 4.);
		}
		if end_x - cursor_x > ww
		{
			state
				.prim
				.draw_line(cursor_x + ww, self.loc.y, end_x, self.loc.y, c_ui, s * 4.);
		}
		//state.prim.draw_filled_circle(self.loc.x - w / 2. + w * self.cur_pos / self.max_pos, self.loc.y, 8., c_ui);

		let text = format!("{:.2}", self.cur_pos);
		let text = if text.contains('.')
		{
			text.trim_end_matches("0").trim_end_matches(".")
		}
		else
		{
			&text
		};

		state.core.draw_text(
			state.ui_font(),
			c_ui,
			cursor_x.floor(),
			self.loc.y - state.ui_font().get_line_height() as f32 / 2.,
			FontAlign::Centre,
			text,
		);
	}

	pub fn input(
		&mut self, event: &Event, sfx: &mut sfx::Sfx, state: &mut hack_state::HackState,
	) -> Option<ActionT>
	{
		let s = state.gfx_options.ui_scale;
		let start = self.loc - s * self.size / 2.;
		let end = self.loc + s * self.size / 2.;
		let increment = self.round_to;
		if state.menu_controls.get_action_state(UIAction::Left) > 0.5
		{
			if self.selected && self.cur_pos > self.min_pos
			{
				sfx.play_sound("data/ui2.ogg").unwrap();
				self.cur_pos = utils::max(self.min_pos, self.cur_pos - increment);
				self.round_cur_pos();
				return Some((self.action_fn)(self.cur_pos));
			}
		}
		if state.menu_controls.get_action_state(UIAction::Right) > 0.5
		{
			if self.selected && self.cur_pos < self.max_pos
			{
				sfx.play_sound("data/ui2.ogg").unwrap();
				self.cur_pos = utils::min(self.max_pos, self.cur_pos + increment);
				self.round_cur_pos();
				return Some((self.action_fn)(self.cur_pos));
			}
		}
		match event
		{
			Event::MouseAxes { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					if self.grabbed
					{
						self.cur_pos = self.min_pos
							+ (x - start.x) / (s * self.width()) * (self.max_pos - self.min_pos);
						self.round_cur_pos();
						return Some((self.action_fn)(self.cur_pos));
					}
					else
					{
						return Some(ActionT::SELECT_ME);
					}
				}
			}
			Event::MouseButtonUp { .. } =>
			{
				self.grabbed = false;
			}
			Event::MouseButtonDown { x, y, .. } =>
			{
				let (x, y) = state.transform_mouse(*x as f32, *y as f32);
				if x > start.x && x < end.x && y > start.y && y < end.y
				{
					sfx.play_sound("data/ui2.ogg").unwrap();
					self.grabbed = true;
					self.cur_pos = self.min_pos
						+ (x - start.x) / (s * self.width()) * (self.max_pos - self.min_pos);
					self.round_cur_pos();
					return Some((self.action_fn)(self.cur_pos));
				}
			}
			_ => (),
		}
		None
	}
}

#[derive(Clone)]
pub struct Label
{
	loc: Point2<f32>,
	size: Vector2<f32>,
	text: String,
	align: FontAlign,
}

impl Label
{
	pub fn new(w: f32, h: f32, text: &str) -> Self
	{
		Self::new_align(w, h, text, FontAlign::Centre)
	}

	pub fn new_align(w: f32, h: f32, text: &str, align: FontAlign) -> Self
	{
		Self {
			loc: Point2::new(0., 0.),
			size: Vector2::new(w, h),
			text: text.into(),
			align: align,
		}
	}

	pub fn width(&self) -> f32
	{
		self.size.x
	}

	pub fn height(&self) -> f32
	{
		self.size.y
	}

	fn draw(&self, state: &hack_state::HackState)
	{
		let x = match self.align
		{
			FontAlign::Centre => self.loc.x,
			FontAlign::Left => self.loc.x - self.size.x / 2.,
			FontAlign::Right => self.loc.x + self.size.x / 2.,
		};
		state.core.draw_text(
			state.ui_font(),
			LABEL,
			x,
			self.loc.y - state.ui_font().get_line_height() as f32 / 2.,
			self.align,
			&self.text,
		);
	}
}

#[derive(Clone)]
pub enum Widget<ActionT>
{
	Button(Button<ActionT>),
	Label(Label),
	Slider(Slider<ActionT>),
	Toggle(Toggle<ActionT>),
}

impl<ActionT: Action + Clone> Widget<ActionT>
{
	pub fn height(&self) -> f32
	{
		match self
		{
			Widget::Button(w) => w.height(),
			Widget::Label(w) => w.height(),
			Widget::Slider(w) => w.height(),
			Widget::Toggle(w) => w.height(),
		}
	}

	pub fn width(&self) -> f32
	{
		match self
		{
			Widget::Button(w) => w.width(),
			Widget::Label(w) => w.width(),
			Widget::Slider(w) => w.width(),
			Widget::Toggle(w) => w.width(),
		}
	}

	pub fn loc(&self) -> Point2<f32>
	{
		match self
		{
			Widget::Button(w) => w.loc,
			Widget::Label(w) => w.loc,
			Widget::Slider(w) => w.loc,
			Widget::Toggle(w) => w.loc,
		}
	}

	pub fn selectable(&self) -> bool
	{
		match self
		{
			Widget::Button(_) => true,
			Widget::Label(_) => false,
			Widget::Slider(_) => true,
			Widget::Toggle(_) => true,
		}
	}

	pub fn set_loc(&mut self, loc: Point2<f32>)
	{
		match self
		{
			Widget::Button(w) => w.loc = loc,
			Widget::Label(w) => w.loc = loc,
			Widget::Slider(w) => w.loc = loc,
			Widget::Toggle(w) => w.loc = loc,
		}
	}

	pub fn selected(&self) -> bool
	{
		match self
		{
			Widget::Button(w) => w.selected,
			Widget::Label(_) => false,
			Widget::Slider(w) => w.selected,
			Widget::Toggle(w) => w.selected,
		}
	}

	pub fn set_selected(&mut self, selected: bool)
	{
		match self
		{
			Widget::Button(w) => w.selected = selected,
			Widget::Label(_) => (),
			Widget::Slider(w) => w.selected = selected,
			Widget::Toggle(w) => w.selected = selected,
		}
	}

	pub fn draw(&self, state: &hack_state::HackState)
	{
		match self
		{
			Widget::Button(w) => w.draw(state),
			Widget::Label(w) => w.draw(state),
			Widget::Slider(w) => w.draw(state),
			Widget::Toggle(w) => w.draw(state),
		}
	}

	pub fn input(
		&mut self, event: &Event, sfx: &mut sfx::Sfx, state: &mut hack_state::HackState,
	) -> Option<ActionT>
	{
		match self
		{
			Widget::Button(w) => w.input(event, sfx, state),
			Widget::Label(_) => None,
			Widget::Slider(w) => w.input(event, sfx, state),
			Widget::Toggle(w) => w.input(event, sfx, state),
		}
	}
}

pub struct WidgetList<ActionT>
{
	widgets: Vec<Vec<Widget<ActionT>>>,
	cur_selection: (usize, usize),
	pos: Point2<f32>,
}

impl<ActionT: Action + Clone> WidgetList<ActionT>
{
	pub fn new(widgets: &[&[Widget<ActionT>]]) -> Self
	{
		let mut new_widgets = Vec::with_capacity(widgets.len());
		let mut cur_selection = None;
		for (i, row) in widgets.iter().enumerate()
		{
			let mut new_row = Vec::with_capacity(row.len());
			for (j, w) in row.iter().enumerate()
			{
				if w.selectable() && cur_selection.is_none()
				{
					cur_selection = Some((i, j));
				}
				new_row.push(w.clone());
			}
			new_widgets.push(new_row);
		}

		if let Some((i, j)) = cur_selection
		{
			new_widgets[i][j].set_selected(true);
		}

		Self {
			pos: Point2::new(0., 0.),
			widgets: new_widgets,
			cur_selection: cur_selection.expect("No selectable widgets?"),
		}
	}

	pub fn draw(&self, state: &hack_state::HackState)
	{
		for row in &self.widgets
		{
			for w in row
			{
				w.draw(state);
			}
		}
	}

	pub fn input(
		&mut self, event: &Event, sfx: &mut sfx::Sfx, state: &mut hack_state::HackState,
	) -> Option<ActionT>
	{
		let mut action = None;
		let old_selection = self.cur_selection;
		'got_action: for (i, row) in self.widgets.iter_mut().enumerate()
		{
			for (j, w) in row.iter_mut().enumerate()
			{
				let cur_action = w.input(event, sfx, state);
				if cur_action.is_some()
				{
					action = cur_action;
					if self.cur_selection != (i, j) && action == Some(ActionT::SELECT_ME)
					{
						sfx.play_sound("data/ui1.ogg").unwrap();
					}
					self.cur_selection = (i, j);
					break 'got_action;
				}
			}
		}
		if action.is_none() || action == Some(ActionT::SELECT_ME)
		{
			if state.menu_controls.get_action_state(UIAction::Up) > 0.5
			{
				sfx.play_sound("data/ui1.ogg").unwrap();
				'found1: loop
				{
					self.cur_selection.0 =
						(self.cur_selection.0 + self.widgets.len() - 1) % self.widgets.len();
					let row_len = self.widgets[self.cur_selection.0].len();
					if self.cur_selection.1 >= row_len
					{
						self.cur_selection.1 = row_len - 1;
					}
					for _ in 0..row_len
					{
						if self.widgets[self.cur_selection.0][self.cur_selection.1].selectable()
						{
							break 'found1;
						}
						self.cur_selection.1 = (self.cur_selection.1 + row_len - 1) % row_len;
					}
				}
			}
			if state.menu_controls.get_action_state(UIAction::Down) > 0.5
			{
				sfx.play_sound("data/ui1.ogg").unwrap();
				'found2: loop
				{
					self.cur_selection.0 =
						(self.cur_selection.0 + self.widgets.len() + 1) % self.widgets.len();
					let row_len = self.widgets[self.cur_selection.0].len();
					if self.cur_selection.1 >= row_len
					{
						self.cur_selection.1 = row_len - 1;
					}
					for _ in 0..row_len
					{
						if self.widgets[self.cur_selection.0][self.cur_selection.1].selectable()
						{
							break 'found2;
						}
						self.cur_selection.1 = (self.cur_selection.1 + row_len - 1) % row_len;
					}
				}
			}
			if state.menu_controls.get_action_state(UIAction::Left) > 0.5
			{
				sfx.play_sound("data/ui1.ogg").unwrap();
				let row_len = self.widgets[self.cur_selection.0].len();
				loop
				{
					self.cur_selection.1 = (self.cur_selection.1 + row_len - 1) % row_len;
					if self.widgets[self.cur_selection.0][self.cur_selection.1].selectable()
					{
						break;
					}
				}
			}
			if state.menu_controls.get_action_state(UIAction::Right) > 0.5
			{
				sfx.play_sound("data/ui1.ogg").unwrap();
				let row_len = self.widgets[self.cur_selection.0].len();
				loop
				{
					self.cur_selection.1 = (self.cur_selection.1 + row_len + 1) % row_len;
					if self.widgets[self.cur_selection.0][self.cur_selection.1].selectable()
					{
						break;
					}
				}
			}
		}
		self.widgets[old_selection.0][old_selection.1].set_selected(false);
		self.widgets[self.cur_selection.0][self.cur_selection.1].set_selected(true);
		action
	}

	pub fn resize(&mut self, state: &hack_state::HackState)
	{
		let s = state.gfx_options.ui_scale;
		let w_space = s * HORIZ_SPACE;
		let h_space = s * VERT_SPACE;
		let cx = self.pos.x;
		let cy = self.pos.y;

		let mut y = 0.;
		let mut cur_selection = None;
		let num_rows = self.widgets.len();
		for (i, row) in self.widgets.iter_mut().enumerate()
		{
			let mut max_height = -f32::INFINITY;
			let mut x = 0.;

			// Place the relative x's, collect max height.
			let num_cols = row.len();
			for (j, w) in row.iter_mut().enumerate()
			{
				if w.selectable() && cur_selection.is_none()
				{
					cur_selection = Some((i, j));
				}
				if j > 0
				{
					x += (w_space + s * w.width()) / 2.;
				}
				let mut loc = w.loc();
				loc.x = x;
				w.set_loc(loc);
				max_height = utils::max(max_height, s * w.height());
				if j + 1 < num_cols
				{
					x += (w_space + s * w.width()) / 2.;
				}
			}

			if i > 0
			{
				y += (h_space + max_height) / 2.;
			}

			// Place the relative y's, shift the x's.
			for w in row.iter_mut()
			{
				let mut loc = w.loc();
				loc.y = y;
				loc.x += cx - x / 2.;
				w.set_loc(loc);
			}

			if i + 1 < num_rows
			{
				y += (h_space + max_height) / 2.;
			}
		}

		// Shift the y's
		for row in self.widgets.iter_mut()
		{
			for w in row.iter_mut()
			{
				let mut loc = w.loc();
				loc.y += cy - y / 2.;
				w.set_loc(loc);
			}
		}
	}

	pub fn set_pos(&mut self, pos: Point2<f32>)
	{
		self.pos = pos;
	}
}
