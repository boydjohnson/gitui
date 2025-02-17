use crate::{
	components::{
		visibility_blocking, CommandBlocking, CommandInfo,
		CommitList, Component, DrawableComponent, EventState,
	},
	keys::SharedKeyConfig,
	queue::{Action, InternalEvent, Queue},
	strings,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
	sync::{self, CommitId},
	CWD,
};
use crossterm::event::Event;

pub struct StashList {
	list: CommitList,
	visible: bool,
	queue: Queue,
	key_config: SharedKeyConfig,
}

impl StashList {
	///
	pub fn new(
		queue: &Queue,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			visible: false,
			list: CommitList::new(
				&strings::stashlist_title(&key_config),
				theme,
				key_config.clone(),
			),
			queue: queue.clone(),
			key_config,
		}
	}

	///
	pub fn update(&mut self) -> Result<()> {
		if self.is_visible() {
			let stashes = sync::get_stashes(CWD)?;
			let commits =
				sync::get_commits_info(CWD, stashes.as_slice(), 100)?;

			self.list.set_count_total(commits.len());
			self.list.items().set_items(0, commits);
		}

		Ok(())
	}

	fn apply_stash(&mut self) {
		if let Some(e) = self.list.selected_entry() {
			match sync::stash_apply(CWD, e.id, false) {
				Ok(_) => {
					self.queue.push(InternalEvent::TabSwitch);
				}
				Err(e) => {
					self.queue.push(InternalEvent::ShowErrorMsg(
						format!("stash apply error:\n{}", e,),
					));
				}
			}
		}
	}

	fn drop_stash(&mut self) {
		if self.list.marked_count() > 0 {
			self.queue.push(InternalEvent::ConfirmAction(
				Action::StashDrop(self.list.marked().to_vec()),
			));
		} else if let Some(e) = self.list.selected_entry() {
			self.queue.push(InternalEvent::ConfirmAction(
				Action::StashDrop(vec![e.id]),
			));
		}
	}

	fn pop_stash(&mut self) {
		if let Some(e) = self.list.selected_entry() {
			self.queue.push(InternalEvent::ConfirmAction(
				Action::StashPop(e.id),
			));
		}
	}

	fn inspect(&mut self) {
		if let Some(e) = self.list.selected_entry() {
			self.queue.push(InternalEvent::InspectCommit(e.id, None));
		}
	}

	/// Called when a pending stash action has been confirmed
	pub fn action_confirmed(action: &Action) -> Result<()> {
		match action {
			Action::StashDrop(ids) => Self::drop(ids)?,
			Action::StashPop(id) => Self::pop(*id)?,
			_ => (),
		};

		Ok(())
	}

	fn drop(ids: &[CommitId]) -> Result<()> {
		for id in ids {
			sync::stash_drop(CWD, *id)?;
		}

		Ok(())
	}

	fn pop(id: CommitId) -> Result<()> {
		sync::stash_pop(CWD, id)?;
		Ok(())
	}
}

impl DrawableComponent for StashList {
	fn draw<B: tui::backend::Backend>(
		&self,
		f: &mut tui::Frame<B>,
		rect: tui::layout::Rect,
	) -> Result<()> {
		self.list.draw(f, rect)?;

		Ok(())
	}
}

impl Component for StashList {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.visible || force_all {
			self.list.commands(out, force_all);

			let selection_valid =
				self.list.selected_entry().is_some();
			out.push(CommandInfo::new(
				strings::commands::stashlist_pop(&self.key_config),
				selection_valid,
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::stashlist_apply(&self.key_config),
				selection_valid,
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::stashlist_drop(
					&self.key_config,
					self.list.marked_count(),
				),
				selection_valid,
				true,
			));
			out.push(CommandInfo::new(
				strings::commands::stashlist_inspect(
					&self.key_config,
				),
				selection_valid,
				true,
			));
		}

		visibility_blocking(self)
	}

	fn event(
		&mut self,
		ev: crossterm::event::Event,
	) -> Result<EventState> {
		if self.is_visible() {
			if self.list.event(ev)?.is_consumed() {
				return Ok(EventState::Consumed);
			}

			if let Event::Key(k) = ev {
				if k == self.key_config.keys.enter {
					self.pop_stash();
				} else if k == self.key_config.keys.stash_apply {
					self.apply_stash();
				} else if k == self.key_config.keys.stash_drop {
					self.drop_stash();
				} else if k == self.key_config.keys.stash_open {
					self.inspect();
				}
			}
		}

		Ok(EventState::NotConsumed)
	}

	fn is_visible(&self) -> bool {
		self.visible
	}

	fn hide(&mut self) {
		self.visible = false;
	}

	fn show(&mut self) -> Result<()> {
		self.visible = true;
		self.update()?;
		Ok(())
	}
}
