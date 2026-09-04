//! Interactive Atuin/fzf-inspired clipboard popup.

use crate::model::{ClipboardEntry, PrototypeAction, UiMode, actions_for};
use crate::sample_data::sample_entries;
use crate::search::{SearchContext, search_entries};
use crate::text_input::TextInput;
use crate::transform::{TransformKind, openable_url, transform_entry};
use chrono::Utc;
use gpui::{
    App, Bounds, ClipboardItem, Context, Entity, FocusHandle, Focusable, KeyBinding, SharedString,
    Window, WindowBounds, WindowDecorations, WindowKind, WindowOptions, actions, div, prelude::*,
    px, rgb, rgba, size,
};

actions!(
    stash,
    [
        MoveUp,
        MoveDown,
        Confirm,
        OpenActions,
        Close,
        EditBeforePaste,
        TogglePin,
        DeleteEntry,
        CopySelected,
        Quit,
    ]
);

pub struct StashApp {
    entries: Vec<ClipboardEntry>,
    query: SharedString,
    selected: usize,
    mode: UiMode,
    search_input: Entity<TextInput>,
    edit_input: Entity<TextInput>,
    focus_handle: FocusHandle,
    search_ctx: SearchContext,
    status: SharedString,
    action_selected: usize,
}

impl StashApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let this = cx.weak_entity();

        let search_input = cx.new(|cx| {
            let mut input = TextInput::new(cx, "search clipboard history…");
            let this = this.clone();
            input.on_change = Some(Box::new(move |text, cx| {
                this.update(cx, |app, cx| {
                    if app.mode == UiMode::Search {
                        app.query = text.to_string().into();
                        app.selected = 0;
                        cx.notify();
                    }
                })
                .ok();
            }));
            input
        });

        let edit_input = cx.new(|cx| TextInput::new(cx, "edit before paste…"));

        Self {
            entries: sample_entries(),
            query: "".into(),
            selected: 0,
            mode: UiMode::Search,
            search_input,
            edit_input,
            focus_handle,
            search_ctx: SearchContext::default(),
            status: "↑↓ navigate   ↵ paste   ⇥ actions   ⌘e edit   ⌘p pin   ⌘d delete   esc close"
                .into(),
            action_selected: 0,
        }
    }

    fn ranked_indices(&self) -> Vec<usize> {
        search_entries(&self.entries, &self.query, Utc::now(), &self.search_ctx)
            .into_iter()
            .map(|hit| hit.index)
            .collect()
    }

    fn selected_entry(&self) -> Option<&ClipboardEntry> {
        let indices = self.ranked_indices();
        indices.get(self.selected).map(|&i| &self.entries[i])
    }

    fn selected_entry_mut(&mut self) -> Option<&mut ClipboardEntry> {
        let indices = self.ranked_indices();
        let idx = *indices.get(self.selected)?;
        self.entries.get_mut(idx)
    }

    fn clamp_selection(&mut self) {
        let len = self.ranked_indices().len();
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }

    fn move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        match self.mode {
            UiMode::Search => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            UiMode::Actions => {
                if self.action_selected > 0 {
                    self.action_selected -= 1;
                }
            }
            UiMode::Edit => {}
        }
        cx.notify();
    }

    fn move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        match self.mode {
            UiMode::Search => {
                let len = self.ranked_indices().len();
                if len > 0 && self.selected + 1 < len {
                    self.selected += 1;
                }
            }
            UiMode::Actions => {
                if let Some(entry) = self.selected_entry() {
                    let count = actions_for(entry).len();
                    if count > 0 && self.action_selected + 1 < count {
                        self.action_selected += 1;
                    }
                }
            }
            UiMode::Edit => {}
        }
        cx.notify();
    }

    fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        match self.mode {
            UiMode::Search => {
                if let Some(content) = self.selected_entry().map(|e| e.content.clone()) {
                    self.copy_and_close(&content, window, cx);
                }
            }
            UiMode::Actions => self.run_selected_action(window, cx),
            UiMode::Edit => {
                let content = self.edit_input.read(cx).content().to_string();
                self.copy_and_close(&content, window, cx);
            }
        }
    }

    fn open_actions(&mut self, _: &OpenActions, window: &mut Window, cx: &mut Context<Self>) {
        if self.mode != UiMode::Search {
            return;
        }
        if self.selected_entry().is_some() {
            self.mode = UiMode::Actions;
            self.action_selected = 0;
            self.status = "↑↓ choose action   ↵ run   esc back".into();
            window.focus(&self.focus_handle, cx);
            cx.notify();
        }
    }

    fn close(&mut self, _: &Close, window: &mut Window, cx: &mut Context<Self>) {
        match self.mode {
            UiMode::Search => {
                window.remove_window();
                cx.quit();
            }
            UiMode::Actions | UiMode::Edit => {
                self.mode = UiMode::Search;
                self.status =
                    "↑↓ navigate   ↵ paste   ⇥ actions   ⌘e edit   ⌘p pin   ⌘d delete   esc close"
                        .into();
                window.focus(&self.search_input.focus_handle(cx), cx);
                cx.notify();
            }
        }
    }

    fn edit_before_paste(
        &mut self,
        _: &EditBeforePaste,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(content) = self.selected_entry().map(|e| e.content.clone()) else {
            return;
        };
        self.mode = UiMode::Edit;
        self.edit_input.update(cx, |input, cx| {
            input.set_content(content, cx);
        });
        self.status = "↵ paste edited value             esc cancel".into();
        window.focus(&self.edit_input.focus_handle(cx), cx);
        cx.notify();
    }

    fn toggle_pin(&mut self, _: &TogglePin, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.selected_entry_mut() {
            entry.pinned = !entry.pinned;
            let label = if entry.pinned { "pinned" } else { "unpinned" };
            self.status = format!("{label} · ⌘p to toggle").into();
            cx.notify();
        }
    }

    fn delete_entry(&mut self, _: &DeleteEntry, _: &mut Window, cx: &mut Context<Self>) {
        let indices = self.ranked_indices();
        if let Some(&idx) = indices.get(self.selected) {
            self.entries.remove(idx);
            self.clamp_selection();
            if self.mode == UiMode::Actions {
                self.mode = UiMode::Search;
            }
            self.status = "deleted".into();
            cx.notify();
        }
    }

    fn copy_selected(&mut self, _: &CopySelected, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(content) = self.selected_entry().map(|e| e.content.clone()) {
            cx.write_to_clipboard(ClipboardItem::new_string(content));
            self.status = "copied (popup stays open)".into();
            cx.notify();
        }
    }

    fn run_selected_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.selected_entry().cloned() else {
            return;
        };
        let actions = actions_for(&entry);
        let Some(action) = actions.get(self.action_selected).copied() else {
            return;
        };

        match action {
            PrototypeAction::Paste | PrototypeAction::Copy => {
                self.copy_and_close(&entry.content, window, cx);
            }
            PrototypeAction::PrettyJson => {
                self.apply_transform(&entry, TransformKind::PrettyJson, window, cx);
            }
            PrototypeAction::MinifyJson => {
                self.apply_transform(&entry, TransformKind::MinifyJson, window, cx);
            }
            PrototypeAction::RemoveTracking => {
                self.apply_transform(&entry, TransformKind::RemoveTracking, window, cx);
            }
            PrototypeAction::DecodeJwt => {
                self.apply_transform(&entry, TransformKind::DecodeJwt, window, cx);
            }
            PrototypeAction::OpenUrl => {
                if let Some(url) = openable_url(&entry) {
                    cx.open_url(url);
                    self.status = "opened URL".into();
                    self.mode = UiMode::Search;
                    cx.notify();
                }
            }
            PrototypeAction::EditBeforePaste => {
                self.edit_before_paste(&EditBeforePaste, window, cx);
            }
            PrototypeAction::Pin => {
                self.toggle_pin(&TogglePin, window, cx);
                self.mode = UiMode::Search;
                window.focus(&self.search_input.focus_handle(cx), cx);
            }
            PrototypeAction::Delete => {
                self.delete_entry(&DeleteEntry, window, cx);
                self.mode = UiMode::Search;
                window.focus(&self.search_input.focus_handle(cx), cx);
            }
        }
    }

    fn apply_transform(
        &mut self,
        entry: &ClipboardEntry,
        kind: TransformKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match transform_entry(entry, kind) {
            Ok(content) => self.copy_and_close(&content, window, cx),
            Err(err) => {
                self.status = err.message().to_string().into();
                cx.notify();
            }
        }
    }

    fn copy_and_close(&mut self, content: &str, window: &mut Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(content.to_string()));
        window.remove_window();
        cx.quit();
    }

    fn render_result_row(
        &self,
        entry: &ClipboardEntry,
        selected: bool,
        now: chrono::DateTime<chrono::Utc>,
    ) -> impl IntoElement {
        let age = entry.age_label(now);
        let glyph = entry.primary_type().glyph();
        let preview = entry.preview_line(64);
        let source = entry.source.app_name.clone();
        let pin = if entry.pinned { " ★" } else { "" };
        let count = if entry.copy_count > 1 {
            format!(" ×{}", entry.copy_count)
        } else {
            String::new()
        };
        let context = entry.context_line();

        let bg = if selected {
            rgba(0x3d59a155)
        } else {
            rgba(0x00000000)
        };
        let fg = if selected {
            rgb(0xc0caf5)
        } else {
            rgb(0xa9b1d6)
        };
        let muted = rgb(0x565f89);

        div()
            .flex()
            .flex_col()
            .w_full()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(bg)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().w(px(36.)).text_color(muted).text_sm().child(age))
                    .child(
                        div()
                            .w(px(36.))
                            .text_color(rgb(0x7aa2f7))
                            .text_sm()
                            .child(glyph.to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(fg)
                            .text_sm()
                            .child(format!("{preview}{count}{pin}")),
                    )
                    .child(div().text_color(muted).text_sm().child(source)),
            )
            .when_some(context, |this, context| {
                this.child(div().pl(px(75.)).text_xs().text_color(muted).child(context))
            })
    }

    fn render_actions_panel(&self) -> impl IntoElement {
        let actions = self.selected_entry().map(actions_for).unwrap_or_default();

        div()
            .id("actions")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .p_3()
            .gap_1()
            .overflow_y_scroll()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x7aa2f7))
                    .mb_2()
                    .child("Actions"),
            )
            .children(actions.into_iter().enumerate().map(|(idx, action)| {
                let selected = idx == self.action_selected;
                div()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(if selected {
                        rgba(0x3d59a155)
                    } else {
                        rgba(0x00000000)
                    })
                    .text_color(if selected {
                        rgb(0xc0caf5)
                    } else {
                        rgb(0xa9b1d6)
                    })
                    .child(action.label())
            }))
    }

    fn render_edit_panel(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .p_4()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x7aa2f7))
                    .child("Edit before paste"),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x3b4261))
                    .bg(rgb(0x1a1b26))
                    .child(self.edit_input.clone()),
            )
    }
}

impl Focusable for StashApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StashApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.clamp_selection();
        let now = Utc::now();
        let indices = self.ranked_indices();
        let selected = self.selected;

        div()
            .key_context("StashApp")
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::open_actions))
            .on_action(cx.listener(Self::close))
            .on_action(cx.listener(Self::edit_before_paste))
            .on_action(cx.listener(Self::toggle_pin))
            .on_action(cx.listener(Self::delete_entry))
            .on_action(cx.listener(Self::copy_selected))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1a1b26))
            .text_color(rgb(0xc0caf5))
            .font_family(".SystemUIFont")
            .child(
                // Search bar
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(0x3b4261))
                    .child(div().text_color(rgb(0x7aa2f7)).text_lg().child(">"))
                    .child(
                        div()
                            .flex_1()
                            .text_lg()
                            .when(self.mode == UiMode::Search, |this| {
                                this.child(self.search_input.clone())
                            })
                            .when(self.mode != UiMode::Search, |this| {
                                this.text_color(rgb(0x565f89))
                                    .child(if self.query.is_empty() {
                                        "search clipboard history…".to_string()
                                    } else {
                                        self.query.to_string()
                                    })
                            }),
                    ),
            )
            .child(
                // Body
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .when(self.mode == UiMode::Search, |this| {
                        this.child(
                            div()
                                .id("results")
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_h_0()
                                .p_2()
                                .gap_1()
                                .overflow_y_scroll()
                                .when(indices.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .flex_1()
                                            .items_center()
                                            .justify_center()
                                            .text_color(rgb(0x565f89))
                                            .child("No matches"),
                                    )
                                })
                                .children(indices.into_iter().enumerate().map(|(row, idx)| {
                                    let entry = &self.entries[idx];
                                    self.render_result_row(entry, row == selected, now)
                                })),
                        )
                    })
                    .when(self.mode == UiMode::Actions, |this| {
                        this.child(self.render_actions_panel())
                    })
                    .when(self.mode == UiMode::Edit, |this| {
                        this.child(self.render_edit_panel())
                    }),
            )
            .child(
                // Footer
                div()
                    .px_4()
                    .py_2()
                    .border_t_1()
                    .border_color(rgb(0x3b4261))
                    .text_xs()
                    .text_color(rgb(0x565f89))
                    .child(self.status.clone()),
            )
    }
}

pub fn run() {
    gpui_platform::application().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("up", MoveUp, Some("StashApp")),
            KeyBinding::new("ctrl-k", MoveUp, Some("StashApp")),
            KeyBinding::new("down", MoveDown, Some("StashApp")),
            KeyBinding::new("ctrl-j", MoveDown, Some("StashApp")),
            KeyBinding::new("enter", Confirm, Some("StashApp")),
            KeyBinding::new("tab", OpenActions, Some("StashApp")),
            KeyBinding::new("escape", Close, Some("StashApp")),
            KeyBinding::new("cmd-e", EditBeforePaste, Some("StashApp")),
            KeyBinding::new("cmd-p", TogglePin, Some("StashApp")),
            KeyBinding::new("cmd-d", DeleteEntry, Some("StashApp")),
            KeyBinding::new("cmd-c", CopySelected, Some("StashApp")),
            KeyBinding::new("cmd-q", Quit, None),
            // Text input bindings
            KeyBinding::new("backspace", crate::text_input::Backspace, Some("TextInput")),
            KeyBinding::new("delete", crate::text_input::Delete, Some("TextInput")),
            KeyBinding::new("left", crate::text_input::Left, Some("TextInput")),
            KeyBinding::new("right", crate::text_input::Right, Some("TextInput")),
            KeyBinding::new(
                "shift-left",
                crate::text_input::SelectLeft,
                Some("TextInput"),
            ),
            KeyBinding::new(
                "shift-right",
                crate::text_input::SelectRight,
                Some("TextInput"),
            ),
            KeyBinding::new("cmd-a", crate::text_input::SelectAll, Some("TextInput")),
            KeyBinding::new("cmd-v", crate::text_input::Paste, Some("TextInput")),
            KeyBinding::new("cmd-x", crate::text_input::Cut, Some("TextInput")),
            KeyBinding::new("home", crate::text_input::Home, Some("TextInput")),
            KeyBinding::new("end", crate::text_input::End, Some("TextInput")),
        ]);

        cx.on_action(|_: &Quit, cx| cx.quit());

        let bounds = Bounds::centered(None, size(px(840.), px(520.)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: None,
                    window_decorations: Some(WindowDecorations::Client),
                    kind: WindowKind::PopUp,
                    is_resizable: false,
                    is_minimizable: false,
                    ..Default::default()
                },
                |_, cx| cx.new(StashApp::new),
            )
            .unwrap();

        window
            .update(cx, |app, window, cx| {
                window.focus(&app.search_input.focus_handle(cx), cx);
                cx.activate(true);
            })
            .unwrap();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_keeps_selection_in_bounds() {
        let mut entries = sample_entries();
        assert!(!entries.is_empty());
        let mut selected = 0usize;
        entries.remove(0);
        let len = entries.len();
        if selected >= len {
            selected = len.saturating_sub(1);
        }
        assert!(selected < entries.len() || entries.is_empty());
    }

    #[test]
    fn pin_toggle_flips_flag() {
        let mut entries = sample_entries();
        let before = entries[0].pinned;
        entries[0].pinned = !before;
        assert_ne!(entries[0].pinned, before);
    }
}
