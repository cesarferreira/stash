//! Read-only selectable multi-line preview text.

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId, Entity, FocusHandle,
    Focusable, GlobalElementId, Hitbox, IntoElement, KeyBinding, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, SharedString,
    StyledText, TextLayout, Window, actions, div, fill, point, prelude::*, px, rgba,
};
use unicode_segmentation::UnicodeSegmentation;

actions!(preview, [CopySelection, SelectAll]);

pub struct SelectablePreview {
    focus_handle: FocusHandle,
    content: SharedString,
    selection: Range<usize>,
    dragging: bool,
    anchor: usize,
    layout: TextLayout,
}

impl SelectablePreview {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            selection: 0..0,
            dragging: false,
            anchor: 0,
            layout: TextLayout::default(),
        }
    }

    pub fn set_content(&mut self, content: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = content.into();
        self.selection = 0..0;
        self.dragging = false;
        self.anchor = 0;
        cx.notify();
    }

    pub fn has_selection(&self) -> bool {
        self.selection.start < self.selection.end
    }

    pub fn selected_text(&self) -> Option<String> {
        if self.has_selection()
            && self.selection.end <= self.content.len()
            && self.content.is_char_boundary(self.selection.start)
            && self.content.is_char_boundary(self.selection.end)
        {
            Some(self.content[self.selection.clone()].to_string())
        } else {
            None
        }
    }

    fn copy_selection(&mut self, _: &CopySelection, _: &mut Window, cx: &mut Context<Self>) {
        let text = self
            .selected_text()
            .unwrap_or_else(|| self.content.to_string());
        if !text.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selection = 0..self.content.len();
        cx.notify();
    }

    fn snap(&self, offset: usize) -> usize {
        if offset >= self.content.len() {
            return self.content.len();
        }
        if self.content.is_char_boundary(offset) {
            offset
        } else {
            self.content
                .grapheme_indices(true)
                .map(|(i, _)| i)
                .take_while(|&i| i <= offset)
                .last()
                .unwrap_or(0)
        }
    }

    fn index_for_position(&self, position: Point<Pixels>) -> usize {
        let raw = self
            .layout
            .index_for_position(position)
            .unwrap_or_else(|e| e);
        self.snap(raw)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        window.focus(&self.focus_handle, cx);
        let ix = self.index_for_position(event.position);
        if event.modifiers.shift {
            self.selection = if ix < self.anchor {
                ix..self.anchor
            } else {
                self.anchor..ix
            };
        } else {
            match event.click_count {
                2 => {
                    self.selection = word_range(&self.content, ix);
                    self.anchor = self.selection.start;
                }
                n if n >= 3 => {
                    self.selection = 0..self.content.len();
                    self.anchor = 0;
                }
                _ => {
                    self.selection = ix..ix;
                    self.anchor = ix;
                    self.dragging = true;
                }
            }
        }
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        let ix = self.index_for_position(event.position);
        self.selection = if ix < self.anchor {
            ix..self.anchor
        } else {
            self.anchor..ix
        };
        cx.notify();
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.dragging = false;
        cx.notify();
    }
}

impl Focusable for SelectablePreview {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SelectablePreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = self.selection.clone();

        div()
            .id("selectable-preview")
            .key_context("SelectablePreview")
            .track_focus(&self.focus_handle(cx))
            .size_full()
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::copy_selection))
            .on_action(cx.listener(Self::select_all))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .child(PreviewTextElement {
                styled: StyledText::new(self.content.clone()),
                selection,
                entity: cx.entity(),
            })
    }
}

struct PreviewTextElement {
    styled: StyledText,
    selection: Range<usize>,
    entity: Entity<SelectablePreview>,
}

struct PrepaintState {
    #[allow(dead_code)]
    hitbox: Hitbox,
}

impl IntoElement for PreviewTextElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PreviewTextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some("preview-text-el".into())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let (layout_id, _) =
            Element::request_layout(&mut self.styled, None, inspector_id, window, cx);
        let layout = self.styled.layout().clone();
        self.entity.update(cx, |preview, _| {
            preview.layout = layout;
        });
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        Element::prepaint(
            &mut self.styled,
            None,
            inspector_id,
            bounds,
            request_layout,
            window,
            cx,
        );
        let layout = self.styled.layout().clone();
        self.entity.update(cx, |preview, _| {
            preview.layout = layout;
        });
        let hitbox = window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal);
        PrepaintState { hitbox }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        for quad in selection_quads(self.styled.layout(), &self.selection) {
            window.paint_quad(quad);
        }
        Element::paint(
            &mut self.styled,
            None,
            inspector_id,
            bounds,
            request_layout,
            &mut (),
            window,
            cx,
        );
    }
}

fn selection_quads(layout: &TextLayout, range: &Range<usize>) -> Vec<PaintQuad> {
    if range.start >= range.end {
        return Vec::new();
    }
    let (Some(start), Some(end)) = (
        layout.position_for_index(range.start),
        layout.position_for_index(range.end),
    ) else {
        return Vec::new();
    };
    let line_height = layout.line_height();
    if line_height <= px(0.) {
        return Vec::new();
    }
    let bounds = layout.bounds();
    let color = rgba(0xff2d7b55);
    let rect = |x0: Pixels, y: Pixels, x1: Pixels| {
        fill(
            Bounds::from_corners(point(x0, y), point(x1.max(x0), y + line_height)),
            color,
        )
    };
    let rows = ((end.y - start.y) / line_height).round() as i32;
    if rows <= 0 {
        return vec![rect(start.x, start.y, end.x)];
    }
    let mut quads = Vec::new();
    quads.push(rect(start.x, start.y, bounds.right()));
    let mut y = start.y + line_height;
    for _ in 1..rows {
        quads.push(rect(bounds.left(), y, bounds.right()));
        y += line_height;
    }
    quads.push(rect(bounds.left(), end.y, end.x));
    quads
}

fn word_range(text: &str, offset: usize) -> Range<usize> {
    if text.is_empty() {
        return 0..0;
    }
    let offset = offset.min(text.len());
    let bytes = text.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.';
    let mut start = offset;
    while start > 0 && is_word(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = offset;
    while end < bytes.len() && is_word(bytes[end]) {
        end += 1;
    }
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }
    if start == end && end < text.len() {
        end = text[end..]
            .chars()
            .next()
            .map(|c| end + c.len_utf8())
            .unwrap_or(end);
    }
    start..end
}

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-c", CopySelection, Some("SelectablePreview")),
        KeyBinding::new("cmd-a", SelectAll, Some("SelectablePreview")),
    ]);
}
