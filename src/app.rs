//! Maccy-simple list + Atuin colors, with a right-hand preview pane.

use crate::fonts;
use crate::model::{
    ClipboardEntry, ContentType, PrototypeAction, UiMode, actions_for,
};
use crate::sample_data::sample_entries;
use crate::search::{SearchContext, search_entries};
use crate::selectable_preview::SelectablePreview;
use crate::transform::{TransformKind, openable_url, transform_entry};
use chrono::Utc;
use gpui::{
    App, Bounds, ClipboardItem, Context, Entity, FocusHandle, Focusable, KeyBinding, KeyDownEvent,
    SharedString, Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind,
    WindowOptions, actions, div, prelude::*, px, rgb, rgba, size,
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
        BackspaceChar,
        ClearQuery,
        PasteIndex1,
        PasteIndex2,
        PasteIndex3,
        PasteIndex4,
        PasteIndex5,
        PasteIndex6,
        PasteIndex7,
        PasteIndex8,
        PasteIndex9,
        Quit,
    ]
);

// Atuin-ish accents on a Maccy-dark chrome.
const BG: u32 = 0x121018;
const PANEL: u32 = 0x1a1824;
const SURFACE: u32 = 0x221f2e;
const BORDER: u32 = 0x2e2a3a;
const TEXT: u32 = 0xd6d4e0;
const MUTED: u32 = 0x8b8798;
const DIM: u32 = 0x5c5868;
const PINK: u32 = 0xff2d7b;
const CYAN: u32 = 0x7dcfff;
const GREEN: u32 = 0x9ece6a;
const SELECT: u32 = 0x2a2035;

pub struct StashApp {
    entries: Vec<ClipboardEntry>,
    query: String,
    edit_buffer: String,
    selected: usize,
    mode: UiMode,
    focus_handle: FocusHandle,
    search_ctx: SearchContext,
    status: SharedString,
    action_selected: usize,
    preview: Entity<SelectablePreview>,
    preview_entry_id: Option<String>,
}

impl StashApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let preview = cx.new(SelectablePreview::new);
        Self {
            entries: sample_entries(),
            query: String::new(),
            edit_buffer: String::new(),
            selected: 0,
            mode: UiMode::Search,
            focus_handle: cx.focus_handle(),
            search_ctx: SearchContext::default(),
            status: SharedString::from(""),
            action_selected: 0,
            preview,
            preview_entry_id: None,
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

    fn active_buffer_mut(&mut self) -> &mut String {
        match self.mode {
            UiMode::Edit => &mut self.edit_buffer,
            _ => &mut self.query,
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.mode == UiMode::Actions {
            return;
        }
        let mods = &event.keystroke.modifiers;
        if mods.platform || mods.control || mods.secondary() {
            return;
        }
        let Some(ch) = event.keystroke.key_char.as_deref() else {
            return;
        };
        if ch.is_empty() || ch.chars().any(|c| c.is_control()) {
            return;
        }
        match event.keystroke.key.as_str() {
            "up" | "down" | "left" | "right" | "enter" | "escape" | "tab" | "backspace"
            | "delete" => return,
            _ => {}
        }
        self.active_buffer_mut().push_str(ch);
        if self.mode == UiMode::Search {
            self.selected = 0;
        }
        cx.notify();
    }

    fn backspace_char(&mut self, _: &BackspaceChar, _: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.mode, UiMode::Search | UiMode::Edit) {
            self.active_buffer_mut().pop();
            if self.mode == UiMode::Search {
                self.selected = 0;
            }
            cx.notify();
        }
    }

    fn clear_query(&mut self, _: &ClearQuery, _: &mut Window, cx: &mut Context<Self>) {
        match self.mode {
            UiMode::Search => {
                self.query.clear();
                self.selected = 0;
                cx.notify();
            }
            UiMode::Edit => {
                self.edit_buffer.clear();
                cx.notify();
            }
            UiMode::Actions => {}
        }
    }

    fn move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        match self.mode {
            UiMode::Search => self.selected = self.selected.saturating_sub(1),
            UiMode::Actions => self.action_selected = self.action_selected.saturating_sub(1),
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
                if let Some(content) = self.paste_payload() {
                    self.copy_and_close(&content, window, cx);
                }
            }
            UiMode::Actions => self.run_selected_action(window, cx),
            UiMode::Edit => {
                let content = self.edit_buffer.clone();
                self.copy_and_close(&content, window, cx);
            }
        }
    }

    fn paste_payload(&self) -> Option<String> {
        self.selected_entry().map(|entry| entry.content.clone())
    }

    fn paste_index(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.mode != UiMode::Search {
            return;
        }
        let indices = self.ranked_indices();
        if let Some(&entry_idx) = indices.get(index) {
            let content = self.entries[entry_idx].content.clone();
            self.copy_and_close(&content, window, cx);
        }
    }

    fn open_actions(&mut self, _: &OpenActions, _: &mut Window, cx: &mut Context<Self>) {
        if self.mode != UiMode::Search || self.selected_entry().is_none() {
            return;
        }
        self.mode = UiMode::Actions;
        self.action_selected = 0;
        cx.notify();
    }

    fn close(&mut self, _: &Close, window: &mut Window, cx: &mut Context<Self>) {
        match self.mode {
            UiMode::Search => {
                window.remove_window();
                cx.quit();
            }
            UiMode::Actions | UiMode::Edit => {
                self.mode = UiMode::Search;
                cx.notify();
            }
        }
    }

    fn edit_before_paste(&mut self, _: &EditBeforePaste, _: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.selected_entry() else {
            return;
        };
        if entry.is_image() {
            self.status = "images are not editable in this prototype".into();
            cx.notify();
            return;
        }
        self.edit_buffer = entry.content.clone();
        self.mode = UiMode::Edit;
        cx.notify();
    }

    fn toggle_pin(&mut self, _: &TogglePin, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.selected_entry_mut() {
            entry.pinned = !entry.pinned;
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
            cx.notify();
        }
    }

    fn copy_selected(&mut self, _: &CopySelected, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected) = self.preview.read(cx).selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(selected));
            self.status = "copied selection".into();
            cx.notify();
            return;
        }
        if let Some(content) = self.paste_payload() {
            cx.write_to_clipboard(ClipboardItem::new_string(content));
            self.status = "copied".into();
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
            }
            PrototypeAction::Delete => {
                self.delete_entry(&DeleteEntry, window, cx);
                self.mode = UiMode::Search;
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

    fn type_color(ty: ContentType) -> u32 {
        match ty {
            ContentType::ShellCommand => GREEN,
            ContentType::Json => 0xe0af68,
            ContentType::Url | ContentType::GitHubUrl => CYAN,
            ContentType::StackTrace => PINK,
            ContentType::Jwt => 0xbb9af7,
            ContentType::Image => 0x7aa2f7,
            ContentType::GitSha | ContentType::Uuid => 0x73daca,
            ContentType::FilePath => 0xff9e64,
            ContentType::PlainText => MUTED,
        }
    }

    fn render_header(&self) -> impl IntoElement {
        let tab = match self.mode {
            UiMode::Search => "Search",
            UiMode::Actions => "Actions",
            UiMode::Edit => "Edit",
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .h(px(40.))
            .bg(rgb(PANEL))
            .border_b_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(PINK))
                            .child("stash 0.1.0"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(TEXT))
                            .child(tab),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(DIM))
                            .child("| Inspect"),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(MUTED))
                    .child("<esc> exit   <tab> actions   <enter> paste"),
            )
    }

    fn render_list_row(
        &self,
        entry: &ClipboardEntry,
        row: usize,
        selected: bool,
        now: chrono::DateTime<chrono::Utc>,
    ) -> impl IntoElement {
        let age = entry.age_ago_label(now);
        let ty = entry.primary_type();
        let shortcut = if row < 9 {
            format!("⌘{}", row + 1)
        } else {
            String::new()
        };
        let pin = if entry.pinned { "*" } else { " " };
        let marker = if selected { ">" } else { " " };

        div()
            .flex()
            .items_center()
            .gap_2()
            .w_full()
            .px_2()
            .py_1p5()
            .rounded_sm()
            .bg(if selected { rgb(SELECT) } else { rgb(BG) })
            .child(
                div()
                    .w(px(12.))
                    .text_size(px(13.))
                    .text_color(rgb(PINK))
                    .child(marker),
            )
            .child(
                // Thumbnail / type chip
                if entry.is_image() {
                    let accent = entry.image.as_ref().map(|i| i.accent).unwrap_or(0x7aa2f7);
                    div()
                        .w(px(34.))
                        .h(px(26.))
                        .rounded_sm()
                        .bg(rgb(accent))
                        .border_1()
                        .border_color(rgb(BORDER))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_size(px(9.))
                                .text_color(rgb(TEXT))
                                .child("IMG"),
                        )
                } else {
                    div()
                        .w(px(34.))
                        .text_size(px(12.))
                        .text_color(rgb(Self::type_color(ty)))
                        .child(ty.glyph().to_string())
                },
            )
            .child(
                div()
                    .w(px(58.))
                    .text_size(px(12.))
                    .text_color(rgb(CYAN))
                    .child(age),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(13.))
                    .text_color(if selected { rgb(PINK) } else { rgb(TEXT) })
                    .child(format!("{pin}{}", entry.preview_line(42))),
            )
            .child(
                div()
                    .w(px(36.))
                    .text_size(px(11.))
                    .text_color(rgb(DIM))
                    .child(shortcut),
            )
    }

    fn render_preview(&self, now: chrono::DateTime<chrono::Utc>) -> impl IntoElement {
        let entry = self.selected_entry().cloned();

        div()
            .flex()
            .flex_col()
            .w(px(360.))
            .h_full()
            .bg(rgb(PANEL))
            .border_l_1()
            .border_color(rgb(BORDER))
            .child(
                // Preview toolbar
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .h(px(36.))
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(MUTED))
                            .child("preview"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .text_size(px(11.))
                            .text_color(rgb(DIM))
                            .child("drag to select")
                            .child("⌘c copy")
                            .child("⌘p pin")
                            .child("⌘d del"),
                    ),
            )
            .child(
                // Content area
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .p_3()
                    .gap_3()
                    .when_some(entry.clone(), |this, entry| {
                        this.child(self.render_preview_body(&entry))
                            .child(self.render_metadata(&entry, now))
                    })
                    .when(entry.is_none(), |this| {
                        this.child(
                            div()
                                .flex_1()
                                .items_center()
                                .justify_center()
                                .text_color(rgb(DIM))
                                .child("no selection"),
                        )
                    }),
            )
    }

    fn sync_preview(&mut self, cx: &mut Context<Self>) {
        let Some(entry) = self.selected_entry().cloned() else {
            if self.preview_entry_id.take().is_some() {
                self.preview.update(cx, |preview, cx| {
                    preview.set_content("", cx);
                });
            }
            return;
        };
        if self.preview_entry_id.as_deref() == Some(entry.id.as_str()) {
            return;
        }
        self.preview_entry_id = Some(entry.id.clone());
        let text = if entry.is_image() {
            format!(
                "{}\n{}×{}",
                entry
                    .image
                    .as_ref()
                    .map(|i| i.label.as_str())
                    .unwrap_or("image"),
                entry.image.as_ref().map(|i| i.width).unwrap_or(0),
                entry.image.as_ref().map(|i| i.height).unwrap_or(0),
            )
        } else if entry.primary_type() == ContentType::Json {
            crate::transform::pretty_json(&entry.content).unwrap_or_else(|_| entry.content.clone())
        } else {
            entry.content.clone()
        };
        self.preview.update(cx, |preview, cx| {
            preview.set_content(text, cx);
        });
    }

    fn render_preview_body(&self, entry: &ClipboardEntry) -> impl IntoElement {
        if let Some(image) = &entry.image {
            div()
                .id("preview-image")
                .flex()
                .flex_col()
                .flex_1()
                .min_h(px(180.))
                .rounded_md()
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(SURFACE))
                .overflow_hidden()
                .child(
                    div()
                        .flex_1()
                        .w_full()
                        .bg(rgb(image.accent))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(14.))
                                .text_color(rgb(TEXT))
                                .child(image.label.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(MUTED))
                                .child(format!("{}×{}", image.width, image.height)),
                        )
                        .child(
                            div()
                                .mt_2()
                                .px_3()
                                .py_2()
                                .rounded_sm()
                                .bg(rgb(0x1a1520))
                                .text_size(px(10.))
                                .text_color(rgb(TEXT))
                                .child(
                                    "  14s   8h ago   git init\n  32ms  2h ago   cargo run\n> 50s  50s ago   stash",
                                ),
                        ),
                )
        } else {
            div()
                .id("preview-text")
                .flex_1()
                .min_h(px(180.))
                .w_full()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(SURFACE))
                .overflow_y_scroll()
                .text_size(px(12.))
                .text_color(rgb(TEXT))
                .child(self.preview.clone())
        }
    }

    fn render_metadata(
        &self,
        entry: &ClipboardEntry,
        _now: chrono::DateTime<chrono::Utc>,
    ) -> impl IntoElement {
        let ty = entry
            .detected_types
            .iter()
            .map(|t| t.label())
            .collect::<Vec<_>>()
            .join(" · ");
        let mut meta_rows = vec![
            ("Application", entry.source.app_name.clone()),
            ("Type", ty),
        ];
        if let Some(image) = &entry.image {
            meta_rows.push((
                "Dimensions",
                format!("{}×{}", image.width, image.height),
            ));
        }
        if let Some(repo) = &entry.source.git_repo {
            let branch = entry
                .source
                .git_branch
                .as_deref()
                .unwrap_or("-");
            meta_rows.push(("Repository", format!("{repo} · {branch}")));
        }
        meta_rows.push((
            "First copy",
            entry.format_timestamp(entry.created_at),
        ));
        meta_rows.push((
            "Last copy",
            entry.format_timestamp(entry.last_copied_at),
        ));
        meta_rows.push(("Copies", entry.copy_count.to_string()));
        if entry.pinned {
            meta_rows.push(("Pinned", "yes".into()));
        }

        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .pt_1()
            .children(meta_rows.into_iter().map(|(label, value)| {
                div()
                    .flex()
                    .gap_3()
                    .text_size(px(11.))
                    .child(
                        div()
                            .w(px(92.))
                            .text_color(rgb(MUTED))
                            .child(label),
                    )
                    .child(div().flex_1().text_color(rgb(TEXT)).child(value))
            }))
    }

    fn render_actions_overlay(&self) -> impl IntoElement {
        let actions = self.selected_entry().map(actions_for).unwrap_or_default();
        div()
            .id("actions")
            .absolute()
            .inset_0()
            .bg(rgba(0x0a0810ee))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id("actions-panel")
                    .w(px(360.))
                    .max_h(px(360.))
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(PANEL))
                    .p_3()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .mb_2()
                            .text_size(px(12.))
                            .text_color(rgb(PINK))
                            .child("actions"),
                    )
                    .children(actions.into_iter().enumerate().map(|(idx, action)| {
                        let selected = idx == self.action_selected;
                        div()
                            .px_3()
                            .py_2()
                            .rounded_sm()
                            .bg(if selected { rgb(SELECT) } else { rgb(PANEL) })
                            .text_size(px(13.))
                            .text_color(if selected { rgb(PINK) } else { rgb(TEXT) })
                            .child(format!(
                                "{} {}",
                                if selected { ">" } else { " " },
                                action.label()
                            ))
                    })),
            )
    }

    fn render_edit_overlay(&self) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x0a0810ee))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(520.))
                    .h(px(280.))
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(PANEL))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(PINK))
                            .child("edit before paste"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(SURFACE))
                            .text_size(px(13.))
                            .text_color(rgb(TEXT))
                            .child(format!("{}▌", self.edit_buffer)),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(MUTED))
                            .child("<enter> paste edited   <esc> cancel   <ctrl-u> clear"),
                    ),
            )
    }

    fn render_search_footer(&self) -> impl IntoElement {
        let mode_badge = match self.mode {
            UiMode::Search => "[ GLOBAL ]",
            UiMode::Actions => "[ ACTIONS ]",
            UiMode::Edit => "[ EDIT ]",
        };
        let display = if self.mode == UiMode::Edit {
            String::new()
        } else {
            format!("{}▌", self.query)
        };

        div()
            .flex()
            .flex_col()
            .bg(rgb(PANEL))
            .border_t_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .h(px(40.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(PINK))
                            .child(mode_badge),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(14.))
                            .text_color(if self.query.is_empty() {
                                rgb(DIM)
                            } else {
                                rgb(TEXT)
                            })
                            .child(if self.query.is_empty() && self.mode == UiMode::Search {
                                "type to search…▌".to_string()
                            } else {
                                display
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(DIM))
                            .child(format!("{} matches", self.ranked_indices().len())),
                    ),
            )
            .when(!self.status.is_empty(), |this| {
                this.child(
                    div()
                        .px_4()
                        .pb_2()
                        .text_size(px(11.))
                        .text_color(rgb(MUTED))
                        .child(self.status.clone()),
                )
            })
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
        self.sync_preview(cx);
        let now = Utc::now();
        let indices = self.ranked_indices();
        let selected = self.selected;

        div()
            .key_context("StashApp")
            .track_focus(&self.focus_handle(cx))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::open_actions))
            .on_action(cx.listener(Self::close))
            .on_action(cx.listener(Self::edit_before_paste))
            .on_action(cx.listener(Self::toggle_pin))
            .on_action(cx.listener(Self::delete_entry))
            .on_action(cx.listener(Self::copy_selected))
            .on_action(cx.listener(Self::backspace_char))
            .on_action(cx.listener(Self::clear_query))
            .on_action(cx.listener(|this, _: &PasteIndex1, w, cx| this.paste_index(0, w, cx)))
            .on_action(cx.listener(|this, _: &PasteIndex2, w, cx| this.paste_index(1, w, cx)))
            .on_action(cx.listener(|this, _: &PasteIndex3, w, cx| this.paste_index(2, w, cx)))
            .on_action(cx.listener(|this, _: &PasteIndex4, w, cx| this.paste_index(3, w, cx)))
            .on_action(cx.listener(|this, _: &PasteIndex5, w, cx| this.paste_index(4, w, cx)))
            .on_action(cx.listener(|this, _: &PasteIndex6, w, cx| this.paste_index(5, w, cx)))
            .on_action(cx.listener(|this, _: &PasteIndex7, w, cx| this.paste_index(6, w, cx)))
            .on_action(cx.listener(|this, _: &PasteIndex8, w, cx| this.paste_index(7, w, cx)))
            .on_action(cx.listener(|this, _: &PasteIndex9, w, cx| this.paste_index(8, w, cx)))
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .font_family(fonts::UI_MONO)
            .text_size(px(13.))
            .line_height(px(18.))
            .border_1()
            .border_color(rgb(BORDER))
            .rounded_xl()
            .overflow_hidden()
            .child(self.render_header())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        // Left list — Maccy density + Atuin columns
                        div()
                            .id("results")
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .px_2()
                            .py_2()
                            .gap_0p5()
                            .overflow_y_scroll()
                            .when(indices.is_empty(), |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .flex_1()
                                        .items_center()
                                        .justify_center()
                                        .text_color(rgb(DIM))
                                        .child("no matches"),
                                )
                            })
                            .children(indices.into_iter().enumerate().map(|(row, idx)| {
                                let entry = &self.entries[idx];
                                self.render_list_row(entry, row, row == selected, now)
                            })),
                    )
                    .child(self.render_preview(now)),
            )
            .child(self.render_search_footer())
            .when(self.mode == UiMode::Actions, |this| {
                this.child(self.render_actions_overlay())
            })
            .when(self.mode == UiMode::Edit, |this| {
                this.child(self.render_edit_overlay())
            })
    }
}

pub fn run() {
    gpui_platform::application().run(|cx: &mut App| {
        fonts::load(cx);
        crate::selectable_preview::bind_keys(cx);
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
            KeyBinding::new("backspace", BackspaceChar, Some("StashApp")),
            KeyBinding::new("ctrl-u", ClearQuery, Some("StashApp")),
            KeyBinding::new("cmd-1", PasteIndex1, Some("StashApp")),
            KeyBinding::new("cmd-2", PasteIndex2, Some("StashApp")),
            KeyBinding::new("cmd-3", PasteIndex3, Some("StashApp")),
            KeyBinding::new("cmd-4", PasteIndex4, Some("StashApp")),
            KeyBinding::new("cmd-5", PasteIndex5, Some("StashApp")),
            KeyBinding::new("cmd-6", PasteIndex6, Some("StashApp")),
            KeyBinding::new("cmd-7", PasteIndex7, Some("StashApp")),
            KeyBinding::new("cmd-8", PasteIndex8, Some("StashApp")),
            KeyBinding::new("cmd-9", PasteIndex9, Some("StashApp")),
            KeyBinding::new("cmd-q", Quit, None),
        ]);

        cx.on_action(|_: &Quit, cx| cx.quit());

        let bounds = Bounds::centered(None, size(px(980.), px(560.)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: None,
                    window_decorations: Some(WindowDecorations::Client),
                    kind: WindowKind::Floating,
                    is_resizable: true,
                    is_minimizable: false,
                    // Opaque enables cleaner glyph rasterization on macOS GPUI.
                    window_background: WindowBackgroundAppearance::Opaque,
                    ..Default::default()
                },
                |_, cx| cx.new(StashApp::new),
            )
            .unwrap();

        window
            .update(cx, |app, window, cx| {
                window.focus(&app.focus_handle, cx);
                cx.activate(true);
            })
            .unwrap();
    });
}
