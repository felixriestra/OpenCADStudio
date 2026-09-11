use crate::{verify_program, CamError, Motion, Point2, Program, Units};

pub fn parse_grbl(source: &str) -> Result<Program, CamError> {
    let mut units = None;
    let mut motions = Vec::new();
    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('(') {
            continue;
        }
        let words = line.split_whitespace().collect::<Vec<_>>();
        if words.contains(&"G21") {
            units = Some(Units::Millimeters);
        }
        if words.contains(&"G20") {
            units = Some(Units::Inches);
        }
        if words.contains(&"M3") {
            let rpm = words
                .iter()
                .find_map(|word| word.strip_prefix('S'))
                .and_then(|value| value.parse().ok())
                .ok_or(CamError::ParseError)?;
            motions.push(Motion::SpindleOn { rpm });
            continue;
        }
        if words.contains(&"M5") {
            motions.push(Motion::SpindleOff);
            continue;
        }
        if words.contains(&"M30") {
            motions.push(Motion::End);
            continue;
        }
        let code = words.first().copied().unwrap_or_default();
        match code {
            "G0" | "G00" => motions.push(Motion::Rapid {
                x: axis(&words, 'X')?,
                y: axis(&words, 'Y')?,
                z: axis(&words, 'Z')?,
            }),
            "G1" | "G01" => motions.push(Motion::Linear {
                x: axis(&words, 'X')?,
                y: axis(&words, 'Y')?,
                z: axis(&words, 'Z')?,
                feed: required_axis(&words, 'F')?,
            }),
            "G2" | "G02" | "G3" | "G03" => motions.push(Motion::Arc {
                clockwise: matches!(code, "G2" | "G02"),
                end: Point2::new(required_axis(&words, 'X')?, required_axis(&words, 'Y')?),
                center_offset: Point2::new(
                    required_axis(&words, 'I')?,
                    required_axis(&words, 'J')?,
                ),
                feed: required_axis(&words, 'F')?,
            }),
            _ if words
                .iter()
                .all(|word| matches!(*word, "G17" | "G90" | "G94" | "G20" | "G21")) => {}
            _ => return Err(CamError::ParseError),
        }
    }
    let program = Program {
        name: "Imported GRBL program".to_string(),
        units: units.ok_or(CamError::ParseError)?,
        motions,
    };
    verify_program(&program)?;
    Ok(program)
}

fn axis(words: &[&str], letter: char) -> Result<Option<f64>, CamError> {
    words
        .iter()
        .find_map(|word| word.strip_prefix(letter))
        .map(|value| value.parse::<f64>().map_err(|_| CamError::ParseError))
        .transpose()
}

fn required_axis(words: &[&str], letter: char) -> Result<f64, CamError> {
    axis(words, letter)?.ok_or(CamError::ParseError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{drill, post_grbl, ProfileParameters};

    #[test]
    fn parses_the_postprocessors_own_output() {
        let original = drill(
            &[Point2::new(2.0, 3.0), Point2::new(8.0, 9.0)],
            ProfileParameters::default(),
        )
        .unwrap();
        let parsed = parse_grbl(&post_grbl(&original)).unwrap();
        assert_eq!(parsed.units, original.units);
        assert_eq!(parsed.motions, original.motions);
    }

    #[test]
    fn rejects_unknown_or_incomplete_code() {
        assert_eq!(parse_grbl("G21\nG1 X1\nM30\n"), Err(CamError::ParseError));
        assert_eq!(parse_grbl("G21\nG28\nM30\n"), Err(CamError::ParseError));
    }
}
