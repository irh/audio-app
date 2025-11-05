use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
};
use vizia::{
    prelude::*,
    vg::{self, Point},
};

pub struct PhaseScope<L, L2>
where
    L: Lens<Target = bool>,
    L2: Lens<Target = ScopeFrames>,
{
    enabled: L,
    frames: L2,
}

impl<L, L2> PhaseScope<L, L2>
where
    L: Lens<Target = bool>,
    L2: Lens<Target = ScopeFrames>,
{
    pub fn new(cx: &mut Context, enabled: L, frames: L2) -> Handle<'_, Self> {
        Self { enabled, frames }
            .build(cx, |_| ())
            .bind(enabled, |mut handle, _| handle.needs_redraw())
            .bind(frames, |mut handle, _| handle.needs_redraw())
    }
}

impl<L, L2> View for PhaseScope<L, L2>
where
    L: Lens<Target = bool>,
    L2: Lens<Target = ScopeFrames>,
{
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let bounds = cx.bounds();
        let rect: vg::Rect = bounds.into();

        let bg_color = Color::darkgray();
        let fg_color = Color::white();

        // Background
        let mut paint = vg::Paint::default();
        paint.set_style(vg::PaintStyle::Fill);
        paint.set_color(bg_color);
        canvas.draw_rect(rect, &paint);

        // Scope
        if self.enabled.get(cx) {
            let center = rect.center();
            let size = rect.size();
            let scale = size.width.min(size.height) / 2.0 * 0.9;

            let frames = self.frames.get(cx);
            let point_count = frames.len() as f32;
            let mut points = frames
                .iter()
                .map(|(left, right)| Point::new(center.x + left * scale, center.y - right * scale));

            if let Some(mut previous_point) = points.next() {
                for (i, point) in points.enumerate() {
                    let alpha = ((i as f32 / point_count) * 255.0 * 0.5) as u8;
                    let mut paint = vg::Paint::default();
                    paint.set_stroke_width(1.0);
                    paint.set_color(Color::rgba(fg_color.r(), fg_color.g(), fg_color.b(), alpha));
                    canvas.draw_line(previous_point, point, &paint);
                    previous_point = point;
                }
            }
        }

        // Border
        let mut paint = vg::Paint::default();
        paint.set_style(vg::PaintStyle::Stroke);
        paint.set_color(fg_color);
        canvas.draw_rect(rect, &paint);
    }
}

#[derive(Clone)]
pub struct ScopeFrames(VecDeque<(f32, f32)>);

impl ScopeFrames {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(VecDeque::with_capacity(capacity))
    }
}

impl Data for ScopeFrames {
    fn same(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        for (a, b) in self.iter().zip(other.iter()) {
            if !a.same(b) {
                return false;
            }
        }

        true
    }
}

impl Deref for ScopeFrames {
    type Target = VecDeque<(f32, f32)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ScopeFrames {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
