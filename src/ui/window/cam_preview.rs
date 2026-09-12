use crate::app::Message;
use iced::mouse;
use iced::widget::canvas::{self, Frame, Path, Program, Stroke};
use iced::widget::{column, container, text};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Theme};

#[derive(Clone)]
struct StockCanvas {
    field: ocs_cam_core::StockHeightField,
}

impl Program<Message> for StockCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let cols = self.field.columns.max(1);
        let rows = self.field.rows.max(1);
        let stride = (cols.max(rows) / 90).max(1);
        let scale = (bounds.width / (cols + rows) as f32 * 1.25)
            .min(bounds.height / (cols + rows) as f32 * 1.5);
        let center = Point::new(bounds.width * 0.5, bounds.height * 0.72);
        let project = |column: usize, row: usize, height: f32| {
            Point::new(
                center.x + (column as f32 - row as f32) * scale,
                center.y - (column as f32 + row as f32) * scale * 0.48 - height * scale * 2.5,
            )
        };
        let min_height = self
            .field
            .heights
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min);
        let top = self
            .field
            .heights
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        for row in (0..rows).step_by(stride) {
            let path = Path::new(|builder| {
                for column in 0..cols {
                    let h = self.field.heights[row * cols + column] - min_height;
                    let p = project(column, row, h);
                    if column == 0 {
                        builder.move_to(p);
                    } else {
                        builder.line_to(p);
                    }
                }
            });
            frame.stroke(
                &path,
                Stroke::default()
                    .with_width(1.0)
                    .with_color(Color::from_rgb(0.25, 0.65, 1.0)),
            );
        }
        for column in (0..cols).step_by(stride) {
            let path = Path::new(|builder| {
                for row in 0..rows {
                    let h = self.field.heights[row * cols + column] - min_height;
                    let p = project(column, row, h);
                    if row == 0 {
                        builder.move_to(p);
                    } else {
                        builder.line_to(p);
                    }
                }
            });
            frame.stroke(
                &path,
                Stroke::default()
                    .with_width(1.0)
                    .with_color(Color::from_rgb(0.85, 0.9, 0.98)),
            );
        }
        let _ = top;
        vec![frame.into_geometry()]
    }
}

pub fn view(
    field: Option<&ocs_cam_core::StockHeightField>,
    step: usize,
    total: usize,
) -> Element<'static, Message> {
    let content: Element<'static, Message> = match field {
        Some(field) => column![
            text(format!(
                "Machining stock simulation — segment {step}/{total}"
            ))
            .size(16),
            canvas::Canvas::new(StockCanvas {
                field: field.clone()
            })
            .width(Fill)
            .height(Fill),
        ]
        .spacing(8)
        .padding(12)
        .into(),
        None => container(text(
            "Run or play a CAM stock simulation to populate this window.",
        ))
        .center_x(Fill)
        .center_y(Fill)
        .into(),
    };
    content
}
