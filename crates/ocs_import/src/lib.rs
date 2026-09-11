//! UI-independent vector import for Mac2CAM.

use tiny_skia_path::{PathSegment, Point, Transform};

const MAX_SVG_BYTES: usize = 32 * 1024 * 1024;
const CURVE_STEPS: usize = 16;
const PX_TO_MM: f64 = 25.4 / 96.0;

#[derive(Clone, Debug, PartialEq)]
pub struct ImportedPath {
    pub source_id: String,
    pub closed: bool,
    pub points_mm: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SvgImport {
    pub width_mm: f64,
    pub height_mm: f64,
    pub paths: Vec<ImportedPath>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReliefHeightField {
    pub width: u32,
    pub height: u32,
    /// Row-major normalized relief values: black = 0, white = 1.
    pub samples: Vec<f32>,
}

pub fn bitmap_relief(bytes: &[u8]) -> Result<ReliefHeightField, String> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| format!("unsupported bitmap: {error}"))?
        .to_luma8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 100_000_000 {
        return Err("bitmap dimensions are empty or exceed 100 megapixels".to_string());
    }
    Ok(ReliefHeightField {
        width,
        height,
        samples: image
            .pixels()
            .map(|pixel| f32::from(pixel[0]) / 255.0)
            .collect(),
    })
}

pub fn import_svg(bytes: &[u8]) -> Result<SvgImport, String> {
    if bytes.is_empty() || bytes.len() > MAX_SVG_BYTES {
        return Err("SVG is empty or exceeds the 32 MiB import limit".to_string());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "SVG is not UTF-8")?;
    let lower = text.to_ascii_lowercase();
    if lower.contains("<!doctype") || lower.contains("<!entity") {
        return Err("SVG document types and entities are not accepted".to_string());
    }
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default())
        .map_err(|error| format!("invalid SVG: {error}"))?;
    let height_px = tree.size().height() as f64;
    let mut paths = Vec::new();
    collect_group(tree.root(), height_px, &mut paths);
    paths.retain(|path| path.points_mm.len() >= 2);
    if paths.is_empty() {
        return Err("SVG contains no importable vector paths".to_string());
    }
    let mut warnings = Vec::new();
    if tree.has_text_nodes() {
        warnings.push("Text was ignored; convert text to paths before import.".to_string());
    }
    Ok(SvgImport {
        width_mm: tree.size().width() as f64 * PX_TO_MM,
        height_mm: height_px * PX_TO_MM,
        paths,
        warnings,
    })
}

fn collect_group(group: &usvg::Group, height_px: f64, output: &mut Vec<ImportedPath>) {
    for node in group.children() {
        match node {
            usvg::Node::Group(group) => collect_group(group, height_px, output),
            usvg::Node::Path(path) if path.is_visible() => flatten_path(path, height_px, output),
            _ => {}
        }
    }
}

fn flatten_path(path: &usvg::Path, height_px: f64, output: &mut Vec<ImportedPath>) {
    let transform = path.abs_transform();
    let mut current = Point::from_xy(0.0, 0.0);
    let mut start = current;
    let mut points = Vec::new();
    let mut part = 0usize;
    for segment in path.data().segments() {
        match segment {
            PathSegment::MoveTo(point) => {
                finish_subpath(path.id(), part, false, &mut points, output);
                part += 1;
                current = point;
                start = point;
                points.push(to_mm(point, transform, height_px));
            }
            PathSegment::LineTo(point) => {
                points.push(to_mm(point, transform, height_px));
                current = point;
            }
            PathSegment::QuadTo(control, end) => {
                let from = current;
                for index in 1..=CURVE_STEPS {
                    let t = index as f32 / CURVE_STEPS as f32;
                    let mt = 1.0 - t;
                    let point = Point::from_xy(
                        mt * mt * from.x + 2.0 * mt * t * control.x + t * t * end.x,
                        mt * mt * from.y + 2.0 * mt * t * control.y + t * t * end.y,
                    );
                    points.push(to_mm(point, transform, height_px));
                }
                current = end;
            }
            PathSegment::CubicTo(c1, c2, end) => {
                let from = current;
                for index in 1..=CURVE_STEPS {
                    let t = index as f32 / CURVE_STEPS as f32;
                    let mt = 1.0 - t;
                    let point = Point::from_xy(
                        mt.powi(3) * from.x
                            + 3.0 * mt * mt * t * c1.x
                            + 3.0 * mt * t * t * c2.x
                            + t.powi(3) * end.x,
                        mt.powi(3) * from.y
                            + 3.0 * mt * mt * t * c1.y
                            + 3.0 * mt * t * t * c2.y
                            + t.powi(3) * end.y,
                    );
                    points.push(to_mm(point, transform, height_px));
                }
                current = end;
            }
            PathSegment::Close => {
                if current != start {
                    points.push(to_mm(start, transform, height_px));
                }
                finish_subpath(path.id(), part, true, &mut points, output);
                current = start;
            }
        }
    }
    finish_subpath(path.id(), part, false, &mut points, output);
}

fn finish_subpath(
    id: &str,
    part: usize,
    closed: bool,
    points: &mut Vec<[f64; 2]>,
    output: &mut Vec<ImportedPath>,
) {
    if closed && points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    if points.len() >= 2 {
        output.push(ImportedPath {
            source_id: if id.is_empty() {
                format!("svg-path-{part}")
            } else {
                format!("{id}-{part}")
            },
            closed,
            points_mm: std::mem::take(points),
        });
    } else {
        points.clear();
    }
}

fn to_mm(mut point: Point, transform: Transform, height_px: f64) -> [f64; 2] {
    transform.map_point(&mut point);
    [
        point.x as f64 * PX_TO_MM,
        (height_px - point.y as f64) * PX_TO_MM,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_primitives_paths_transforms_and_physical_units() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="48" viewBox="0 0 96 48"><g transform="translate(10 5)"><rect id="part" x="0" y="0" width="20" height="10"/></g><path d="M0 0 C10 0 10 10 20 10"/></svg>"#;
        let imported = import_svg(svg).unwrap();
        assert!((imported.width_mm - 25.4).abs() < 1.0e-6);
        assert_eq!(imported.paths.len(), 2);
        assert!(imported.paths.iter().any(|path| path.closed));
        assert!(imported.paths.iter().any(|path| path.points_mm.len() > 10));
    }

    #[test]
    fn rejects_xml_entity_documents() {
        let svg = br#"<!DOCTYPE svg [<!ENTITY x "bad">]><svg xmlns="http://www.w3.org/2000/svg">&x;</svg>"#;
        assert!(import_svg(svg).unwrap_err().contains("entities"));
    }

    #[test]
    fn grayscale_bitmap_becomes_a_normalized_relief_field() {
        let mut png = Vec::new();
        image::DynamicImage::ImageLuma8(image::GrayImage::from_raw(2, 1, vec![0, 255]).unwrap())
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let relief = bitmap_relief(&png).unwrap();
        assert_eq!(relief.samples, vec![0.0, 1.0]);
    }
}
