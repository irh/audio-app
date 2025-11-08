use egui::{Color32, Response, Sense, Stroke, StrokeKind, Ui, Widget, pos2};

pub struct PhaseScope<T>
where
    T: ExactSizeIterator<Item = (f32, f32)>,
{
    enabled: bool,
    frames: T,
}

impl<T> PhaseScope<T>
where
    T: ExactSizeIterator<Item = (f32, f32)>,
{
    pub fn new(enabled: bool, frames: T) -> Self {
        Self { enabled, frames }
    }
}

impl<T> Widget for PhaseScope<T>
where
    T: ExactSizeIterator<Item = (f32, f32)>,
{
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            ui.set_clip_rect(rect);
            let painter = ui.painter();

            // Background
            painter.rect_filled(rect, 0.0, ui.visuals().noninteractive().bg_fill);

            // Scope
            if self.enabled {
                ui.ctx().request_repaint();

                let center = rect.center();
                let scale = rect.width().min(rect.height()) / 2.0 * 0.9;

                let point_count = self.frames.len() as f32;
                let mut points = self
                    .frames
                    .map(|(left, right)| pos2(center.x + left * scale, center.y - right * scale));

                if let Some(mut previous_point) = points.next() {
                    let stroke = ui.visuals().noninteractive().fg_stroke;
                    let color = stroke.color;
                    for (i, point) in points.enumerate() {
                        let alpha = ((i as f32 / point_count) * 255.0 * 0.5) as u8;
                        let color =
                            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
                        painter.line_segment(
                            [previous_point, point],
                            Stroke::new(stroke.width, color),
                        );
                        previous_point = point;
                    }
                }
            }

            // Border
            painter.rect_stroke(
                rect,
                ui.visuals().noninteractive().corner_radius,
                ui.visuals().noninteractive().bg_stroke,
                StrokeKind::Inside,
            );
        }

        response
    }
}
