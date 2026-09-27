//! Media query evaluation for one viewport bucket (plan §2.3) and
//! breakpoint collection.
//!
//! Fixed answers (deterministic, low-fingerprint): `prefers-color-scheme:
//! light`, `hover: none`, `pointer: coarse`, `prefers-reduced-motion:
//! no-preference`, `screen`/`all` match, `print` does not.

use std::collections::BTreeSet;

use lightningcss::media_query::{
    MediaCondition, MediaFeature, MediaFeatureComparison, MediaFeatureId, MediaFeatureName,
    MediaFeatureValue, MediaList, MediaType, Operator, Qualifier, QueryFeature,
};
use lightningcss::values::length::{Length, LengthValue};

/// The viewport the page is cascaded for, in CSS px.
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    /// Width bucket.
    pub width: f32,
    /// Height (only used for `height`/`orientation` queries).
    pub height: f32,
}

/// Accumulates every width breakpoint seen while evaluating queries.
#[derive(Debug, Default)]
pub struct Breakpoints(pub BTreeSet<u16>);

impl Breakpoints {
    fn add(&mut self, px: f32) {
        if px.is_finite() && px >= 0.0 {
            self.0.insert(px.round().min(f32::from(u16::MAX)) as u16);
        }
    }

    /// Sorted, deduplicated breakpoints.
    pub fn into_vec(self) -> Vec<u16> {
        self.0.into_iter().collect()
    }
}

/// Evaluates a media list. An empty list matches.
pub fn evaluate(list: &MediaList<'_>, vp: &Viewport, bps: &mut Breakpoints) -> bool {
    if list.media_queries.is_empty() {
        return true;
    }
    let mut any = false;
    for q in &list.media_queries {
        let type_ok = match &q.media_type {
            MediaType::All | MediaType::Screen => true,
            MediaType::Print | MediaType::Custom(_) => false,
        };
        let cond_ok = q
            .condition
            .as_ref()
            .map(|c| eval_condition(c, vp, bps))
            .unwrap_or(true);
        let mut result = type_ok && cond_ok;
        if matches!(q.qualifier, Some(Qualifier::Not)) {
            result = !result;
        }
        any |= result;
    }
    any
}

fn eval_condition(c: &MediaCondition<'_>, vp: &Viewport, bps: &mut Breakpoints) -> bool {
    match c {
        MediaCondition::Feature(f) => eval_feature(f, vp, bps),
        MediaCondition::Not(inner) => !eval_condition(inner, vp, bps),
        MediaCondition::Operation {
            operator,
            conditions,
        } => {
            // Evaluate every branch so all breakpoints are collected.
            let results: Vec<bool> = conditions
                .iter()
                .map(|c| eval_condition(c, vp, bps))
                .collect();
            match operator {
                Operator::And => results.iter().all(|&b| b),
                Operator::Or => results.iter().any(|&b| b),
            }
        }
        MediaCondition::Unknown(_) => false,
    }
}

/// Converts a media-query length to px. `em`/`rem` use the initial 16px.
fn length_px(l: &Length) -> Option<f32> {
    match l {
        Length::Value(v) => match v {
            LengthValue::Em(e) | LengthValue::Rem(e) => Some(e * 16.0),
            LengthValue::Vw(v) => Some(*v),
            other => other.to_px(),
        },
        Length::Calc(_) => None,
    }
}

fn compare(op: MediaFeatureComparison, actual: f32, wanted: f32) -> bool {
    match op {
        MediaFeatureComparison::Equal => (actual - wanted).abs() < 0.5,
        MediaFeatureComparison::GreaterThan => actual > wanted,
        MediaFeatureComparison::GreaterThanEqual => actual >= wanted,
        MediaFeatureComparison::LessThan => actual < wanted,
        MediaFeatureComparison::LessThanEqual => actual <= wanted,
    }
}

fn ident_is(v: &MediaFeatureValue<'_>, s: &str) -> bool {
    matches!(v, MediaFeatureValue::Ident(i) if i.0.eq_ignore_ascii_case(s))
}

fn eval_feature(f: &MediaFeature<'_>, vp: &Viewport, bps: &mut Breakpoints) -> bool {
    use MediaFeatureId as Id;
    let (name, op, value) = match f {
        QueryFeature::Plain { name, value } => {
            (name, Some(MediaFeatureComparison::Equal), Some(value))
        }
        QueryFeature::Boolean { name } => (name, None, None),
        QueryFeature::Range {
            name,
            operator,
            value,
        } => (name, Some(*operator), Some(value)),
        QueryFeature::Interval {
            name,
            start,
            start_operator,
            end,
            end_operator,
        } => {
            // (a < width < b): evaluate as two ranges.
            let lo = QueryFeature::Range {
                name: name.clone(),
                operator: flip(*start_operator),
                value: start.clone(),
            };
            let hi = QueryFeature::Range {
                name: name.clone(),
                operator: *end_operator,
                value: end.clone(),
            };
            return eval_feature(&lo, vp, bps) && eval_feature(&hi, vp, bps);
        }
    };
    let MediaFeatureName::Standard(id) = name else {
        return false;
    };
    let value_len = |bps: &mut Breakpoints, record: bool| -> Option<f32> {
        match value {
            Some(MediaFeatureValue::Length(l)) => {
                let px = length_px(l)?;
                if record {
                    bps.add(px);
                }
                Some(px)
            }
            Some(MediaFeatureValue::Number(n)) => Some(*n),
            _ => None,
        }
    };
    match id {
        Id::Width => match op {
            None => vp.width > 0.0,
            Some(op) => value_len(bps, true).is_some_and(|w| compare(op, vp.width, w)),
        },
        Id::Height => match op {
            None => vp.height > 0.0,
            Some(op) => value_len(bps, false).is_some_and(|h| compare(op, vp.height, h)),
        },
        Id::AspectRatio => op.is_none(),
        Id::Orientation => match value {
            None => true,
            Some(v) => {
                let landscape = vp.width >= vp.height;
                (ident_is(v, "landscape") && landscape) || (ident_is(v, "portrait") && !landscape)
            }
        },
        Id::Hover | Id::AnyHover => match value {
            None => false,
            Some(v) => ident_is(v, "none"),
        },
        Id::Pointer | Id::AnyPointer => match value {
            None => true,
            Some(v) => ident_is(v, "coarse"),
        },
        Id::PrefersColorScheme => match value {
            None => true,
            Some(v) => ident_is(v, "light"),
        },
        Id::PrefersReducedMotion | Id::PrefersReducedTransparency => match value {
            None => false,
            Some(v) => ident_is(v, "no-preference"),
        },
        Id::PrefersContrast => match value {
            None => false,
            Some(v) => ident_is(v, "no-preference"),
        },
        Id::ForcedColors => match value {
            None => false,
            Some(v) => ident_is(v, "none"),
        },
        Id::Color => match op {
            None => true,
            Some(op) => value_len(bps, false).is_some_and(|c| compare(op, 8.0, c)),
        },
        Id::Resolution => op.is_none(),
        Id::Scripting => match value {
            None => false,
            Some(v) => ident_is(v, "none"),
        },
        Id::Update => match value {
            None => true,
            Some(v) => ident_is(v, "slow"),
        },
        _ => false,
    }
}

fn flip(op: MediaFeatureComparison) -> MediaFeatureComparison {
    match op {
        MediaFeatureComparison::Equal => MediaFeatureComparison::Equal,
        MediaFeatureComparison::GreaterThan => MediaFeatureComparison::LessThan,
        MediaFeatureComparison::GreaterThanEqual => MediaFeatureComparison::LessThanEqual,
        MediaFeatureComparison::LessThan => MediaFeatureComparison::GreaterThan,
        MediaFeatureComparison::LessThanEqual => MediaFeatureComparison::GreaterThanEqual,
    }
}

/// Parses a media list from text (a `media=""` attribute or an `@import`
/// media list). Unparsable input matches nothing.
pub fn parse_media_list(text: &str) -> Option<MediaList<'_>> {
    let mut input = cssparser::ParserInput::new(text);
    let mut parser = cssparser::Parser::new(&mut input);
    let options = lightningcss::stylesheet::ParserOptions::default();
    MediaList::parse(&mut parser, &options).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(q: &str, w: f32) -> (bool, Vec<u16>) {
        let list = parse_media_list(q).expect("parse");
        let mut bps = Breakpoints::default();
        let r = evaluate(
            &list,
            &Viewport {
                width: w,
                height: 800.0,
            },
            &mut bps,
        );
        (r, bps.into_vec())
    }

    #[test]
    fn width_ranges_and_breakpoints() {
        assert_eq!(eval("(min-width: 600px)", 1280.0), (true, vec![600]));
        assert_eq!(eval("(max-width: 600px)", 1280.0), (false, vec![600]));
        assert_eq!(
            eval("screen and (max-width: 40em)", 500.0),
            (true, vec![640])
        );
        assert_eq!(eval("(width >= 700px)", 700.0), (true, vec![700]));
        assert_eq!(
            eval("(400px <= width < 800px)", 799.0),
            (true, vec![400, 800])
        );
        assert_eq!(
            eval("(400px <= width < 800px)", 800.0),
            (false, vec![400, 800])
        );
    }

    #[test]
    fn fixed_answers() {
        assert!(!eval("print", 1280.0).0);
        assert!(eval("not print", 1280.0).0);
        assert!(eval("screen", 1280.0).0);
        assert!(eval("(prefers-color-scheme: light)", 1280.0).0);
        assert!(!eval("(prefers-color-scheme: dark)", 1280.0).0);
        assert!(eval("(hover: none)", 1280.0).0);
        assert!(!eval("(hover: hover)", 1280.0).0);
        assert!(eval("(pointer: coarse)", 1280.0).0);
        assert!(!eval("(pointer: fine)", 1280.0).0);
        assert!(eval("(orientation: landscape)", 1280.0).0);
        assert!(!eval("(orientation: portrait)", 1280.0).0);
        assert!(eval("(prefers-reduced-motion: no-preference)", 1280.0).0);
        assert!(eval("", 1280.0).0);
    }

    #[test]
    fn logic() {
        assert_eq!(
            eval(
                "(min-width: 100px) and (max-width: 200px), (min-width: 1000px)",
                1280.0
            ),
            (true, vec![100, 200, 1000])
        );
        assert!(!eval("not all and (min-width: 100px)", 1280.0).0);
        assert!(!eval("(min-width: 100px) and (hover: hover)", 1280.0).0);
    }
}
