use crate::app::ScopeFrames;
use iced::{
    Color, Element, Length, Point, Rectangle, Size, Vector,
    advanced::{
        Layout,
        graphics::geometry,
        layout::{Limits, Node},
        mouse::Cursor,
        renderer::{self},
        widget::{Tree, Widget},
    },
    widget::canvas::{Frame, Path, Stroke},
};

pub struct PhaseScope {
    enabled: bool,
    frames: ScopeFrames,
}

impl PhaseScope {
    pub fn new(enabled: bool, frames: ScopeFrames) -> Self {
        Self { enabled, frames }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for PhaseScope
where
    Renderer: geometry::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn layout(&self, _tree: &mut Tree, _renderer: &Renderer, limits: &Limits) -> Node {
        let max = limits.max();
        let size = max.width.min(max.height);
        Node::new(Size::new(size, size))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let mut frame = Frame::new(renderer, bounds.size());
        let size = frame.size();

        // TODO: Figure out theming
        let bg_color = Color::from_rgb(0.2, 0.2, 0.2);
        let fg_color = Color::from_rgb(0.8, 0.8, 0.8);

        frame.fill_rectangle(Point::ORIGIN, size, bg_color);

        if self.enabled {
            let center = frame.center();
            let scale = size.width.min(size.height) / 2.0 * 0.9;

            let frames = self.frames.lock().unwrap();
            let point_count = frames.len() as f32;
            let mut points = frames
                .iter()
                .map(|(left, right)| Point::new(center.x + left * scale, center.y - right * scale));

            if let Some(mut previous_point) = points.next() {
                for (i, point) in points.enumerate() {
                    let alpha = (i as f32 / point_count) * 0.3;
                    frame.stroke(
                        &Path::line(previous_point, point),
                        Stroke::default().with_color(fg_color.scale_alpha(alpha)),
                    );
                    previous_point = point;
                }
            }
        }

        frame.stroke_rectangle(Point::ORIGIN, size, Stroke::default().with_color(fg_color));

        let geometry = frame.into_geometry();

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }
}

impl<'a, Message, Theme, Renderer> From<PhaseScope> for Element<'a, Message, Theme, Renderer>
where
    Renderer: geometry::Renderer,
{
    fn from(value: PhaseScope) -> Self {
        Self::new(value)
    }
}
