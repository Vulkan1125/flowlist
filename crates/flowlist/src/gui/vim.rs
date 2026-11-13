//! Vim-style key sequences.
//!
//! The GUI feeds every unmodified keystroke through [`VimState`]. The state
//! machine owns counts, operators and the multi-key prefixes (`g`, `d`, `y`,
//! `Ctrl-w`) so every pane shares one grammar.

use gpui::Modifiers;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VimAction {
    Move(isize),
    HalfPage(isize),
    ReorderDown,
    ReorderUp,
    Top,
    Bottom,
    Goto(usize),
    Indent,
    Outdent,
    Complete,
    Flag,
    NewBelow,
    NewAbove,
    Delete,
    Yank,
    Paste,
    Undo,
    Redo,
    Rename,
    Edit,
    Location,
    Search,
    NextMatch,
    PrevMatch,
    Open,
    Inspector,
    Theme,
    Help,
    FocusSidebar,
    FocusContent,
    FocusChecklist,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VimInput {
    Action(VimAction),
    /// A count or operator prefix was consumed; wait for the next key.
    Pending,
    /// Not a vim binding, so the existing shortcut handling may run.
    Ignored,
}

#[derive(Clone, Debug, Default)]
pub struct VimState {
    count: Option<usize>,
    pending_g: bool,
    pending_d: bool,
    pending_y: bool,
    pending_ctrl_w: bool,
}

impl VimState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn is_pending(&self) -> bool {
        self.count.is_some()
            || self.pending_g
            || self.pending_d
            || self.pending_y
            || self.pending_ctrl_w
    }

    fn take_count(&mut self) -> usize {
        self.count.take().unwrap_or(1)
    }

    /// Feed one keystroke. Platform shortcuts are never consumed so ⌘Z, ⌘N,
    /// ⌘1…8 and the rest keep working exactly as before.
    pub fn feed(&mut self, key: &str, mods: Modifiers) -> VimInput {
        if self.pending_ctrl_w {
            self.pending_ctrl_w = false;
            self.count = None;
            return match key {
                "h" => VimInput::Action(VimAction::FocusSidebar),
                "l" => VimInput::Action(VimAction::FocusContent),
                "j" => VimInput::Action(VimAction::FocusChecklist),
                _ => VimInput::Pending,
            };
        }

        if mods.control {
            self.count = None;
            return match key {
                "d" => VimInput::Action(VimAction::HalfPage(1)),
                "u" => VimInput::Action(VimAction::HalfPage(-1)),
                "r" => VimInput::Action(VimAction::Redo),
                "w" => {
                    self.pending_ctrl_w = true;
                    VimInput::Pending
                }
                _ => VimInput::Ignored,
            };
        }

        if mods.platform || mods.alt {
            self.reset();
            return VimInput::Ignored;
        }

        if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() {
            let digit = (key.as_bytes()[0] - b'0') as usize;
            if digit == 0 && self.count.is_none() {
                return VimInput::Ignored;
            }
            self.count = Some(
                self.count
                    .unwrap_or(0)
                    .saturating_mul(10)
                    .saturating_add(digit),
            );
            return VimInput::Pending;
        }

        if self.pending_g {
            self.pending_g = false;
            if key == "g" {
                return VimInput::Action(match self.count.take() {
                    Some(line) => VimAction::Goto(line),
                    None => VimAction::Top,
                });
            }
            self.count = None;
            return VimInput::Ignored;
        }

        if self.pending_d {
            self.pending_d = false;
            if key == "d" {
                self.count = None;
                return VimInput::Action(VimAction::Delete);
            }
            self.count = None;
            return VimInput::Ignored;
        }

        if self.pending_y {
            self.pending_y = false;
            if key == "y" {
                self.count = None;
                return VimInput::Action(VimAction::Yank);
            }
            self.count = None;
            return VimInput::Ignored;
        }

        match key {
            "j" if mods.shift => {
                self.count = None;
                VimInput::Action(VimAction::ReorderDown)
            }
            "k" if mods.shift => {
                self.count = None;
                VimInput::Action(VimAction::ReorderUp)
            }
            "j" => {
                let count = self.take_count();
                VimInput::Action(VimAction::Move(count as isize))
            }
            "k" => {
                let count = self.take_count();
                VimInput::Action(VimAction::Move(-(count as isize)))
            }
            "h" => {
                self.count = None;
                VimInput::Action(VimAction::Outdent)
            }
            "l" if mods.shift => {
                self.count = None;
                VimInput::Action(VimAction::Location)
            }
            "t" if mods.shift => {
                self.count = None;
                VimInput::Action(VimAction::Theme)
            }
            "l" => {
                self.count = None;
                VimInput::Action(VimAction::Indent)
            }
            "g" if mods.shift => {
                let line = self.count.take();
                VimInput::Action(match line {
                    Some(line) => VimAction::Goto(line),
                    None => VimAction::Bottom,
                })
            }
            "g" => {
                self.pending_g = true;
                VimInput::Pending
            }
            "d" => {
                self.pending_d = true;
                VimInput::Pending
            }
            "y" => {
                self.pending_y = true;
                VimInput::Pending
            }
            "x" => {
                self.count = None;
                VimInput::Action(VimAction::Complete)
            }
            "!" => {
                self.count = None;
                VimInput::Action(VimAction::Flag)
            }
            "o" if mods.shift => {
                self.count = None;
                VimInput::Action(VimAction::NewAbove)
            }
            "o" => {
                self.count = None;
                VimInput::Action(VimAction::NewBelow)
            }
            "u" => {
                self.count = None;
                VimInput::Action(VimAction::Undo)
            }
            "p" => {
                self.count = None;
                VimInput::Action(VimAction::Paste)
            }
            "r" => {
                self.count = None;
                VimInput::Action(VimAction::Rename)
            }
            "e" => {
                self.count = None;
                VimInput::Action(VimAction::Edit)
            }
            "/" => {
                self.count = None;
                VimInput::Action(VimAction::Search)
            }
            "n" if mods.shift => {
                self.count = None;
                VimInput::Action(VimAction::PrevMatch)
            }
            "n" => {
                self.count = None;
                VimInput::Action(VimAction::NextMatch)
            }
            "enter" => {
                self.count = None;
                VimInput::Action(VimAction::Open)
            }
            "i" => {
                self.count = None;
                VimInput::Action(VimAction::Inspector)
            }
            "?" => {
                self.count = None;
                VimInput::Action(VimAction::Help)
            }
            "escape" => {
                self.reset();
                VimInput::Action(VimAction::Cancel)
            }
            _ => {
                self.count = None;
                VimInput::Ignored
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain() -> Modifiers {
        Modifiers::default()
    }

    fn keys(sequence: &[&str]) -> VimInput {
        let mut state = VimState::default();
        let mut result = VimInput::Ignored;
        for key in sequence {
            result = state.feed(key, plain());
        }
        result
    }

    #[test]
    fn counts_multiply_motions() {
        assert_eq!(keys(&["3", "j"]), VimInput::Action(VimAction::Move(3)));
        assert_eq!(
            keys(&["1", "2", "k"]),
            VimInput::Action(VimAction::Move(-12))
        );
    }

    #[test]
    fn gg_and_shift_g_navigate() {
        assert_eq!(keys(&["g", "g"]), VimInput::Action(VimAction::Top));
        let mut state = VimState::default();
        let mut mods = plain();
        mods.shift = true;
        assert_eq!(state.feed("g", mods), VimInput::Action(VimAction::Bottom));
    }

    #[test]
    fn counted_shift_g_goes_to_line() {
        let mut state = VimState::default();
        assert_eq!(state.feed("5", plain()), VimInput::Pending);
        let mut mods = plain();
        mods.shift = true;
        assert_eq!(state.feed("g", mods), VimInput::Action(VimAction::Goto(5)));
    }

    #[test]
    fn operators_need_two_keys() {
        assert_eq!(keys(&["d"]), VimInput::Pending);
        assert_eq!(keys(&["d", "d"]), VimInput::Action(VimAction::Delete));
        assert_eq!(keys(&["y"]), VimInput::Pending);
        assert_eq!(keys(&["y", "y"]), VimInput::Action(VimAction::Yank));
    }

    #[test]
    fn ctrl_prefixes_work() {
        let mut state = VimState::default();
        let mut mods = plain();
        mods.control = true;
        assert_eq!(
            state.feed("d", mods),
            VimInput::Action(VimAction::HalfPage(1))
        );
        assert_eq!(state.feed("w", mods), VimInput::Pending);
        assert_eq!(
            state.feed("h", plain()),
            VimInput::Action(VimAction::FocusSidebar)
        );
    }

    #[test]
    fn shift_l_edits_locations() {
        let mut state = VimState::default();
        let mut mods = plain();
        mods.shift = true;
        assert_eq!(state.feed("l", mods), VimInput::Action(VimAction::Location));
        assert_eq!(keys(&["l"]), VimInput::Action(VimAction::Indent));
    }

    #[test]
    fn shift_t_toggles_the_theme() {
        let mut state = VimState::default();
        let mut mods = plain();
        mods.shift = true;
        assert_eq!(state.feed("t", mods), VimInput::Action(VimAction::Theme));
        assert_eq!(keys(&["t"]), VimInput::Ignored);
    }

    #[test]
    fn platform_shortcuts_pass_through() {
        let mut state = VimState::default();
        let mut mods = plain();
        mods.platform = true;
        assert_eq!(state.feed("z", mods), VimInput::Ignored);
    }
}
