use std::ops::RangeBounds;

use clm_macros::ConvertValueInApi;
use serde::{Deserialize, Serialize};

use super::id::PaneId;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ConvertValueInApi)]
pub struct Rect {
    pub offset: (u16, u16),
    pub size: (u16, u16),
}
impl Rect {
    pub fn clip(&mut self, outer: Rect) {
        self.offset.0 = self.offset.0.min(outer.size.0);
        self.offset.1 = self.offset.1.min(outer.size.1);
        self.size.0 = self.size.0.min(outer.size.0 - self.offset.0);
        self.size.1 = self.size.1.min(outer.size.1 - self.offset.1);
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ConvertValueInApi)]
pub enum Direction {
    Horizontal,
    Vertical,
}
pub type SizeRange = (u16, u16);
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ConvertValueInApi)]
pub struct SizeConstraint {
    pub weight: (f64, f64),
    pub range: (SizeRange, SizeRange),
}
impl Default for SizeConstraint {
    fn default() -> Self {
        Self::new((1.0, 1.0), (.., ..))
    }
}
fn range_bounds_into_range(range: impl std::ops::RangeBounds<u16>) -> SizeRange {
    use std::ops::Bound;
    let start = match range.start_bound() {
        Bound::Included(&n) => n,
        Bound::Excluded(&n) => n.checked_add(1).expect("start overflow"),
        Bound::Unbounded => u16::MIN,
    };
    let last = match range.end_bound() {
        Bound::Included(&n) => n,
        Bound::Excluded(&n) => n.checked_sub(1).expect("end underflow (empty range)"),
        Bound::Unbounded => u16::MAX,
    };

    (start, last)
}
impl SizeConstraint {
    pub fn new(weight: (f64, f64), range: (impl RangeBounds<u16>, impl RangeBounds<u16>)) -> Self {
        assert!(!weight.0.is_nan() && !weight.1.is_nan());
        SizeConstraint {
            weight: (
                weight.0.clamp(f64::EPSILON, f64::MAX),
                weight.1.clamp(f64::EPSILON, f64::MAX),
            ),
            range: (
                range_bounds_into_range(range.0),
                range_bounds_into_range(range.1),
            ),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ConvertValueInApi)]
pub struct PaneEntry {
    pub parent: Option<PaneId>,
    pub handler: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ConvertValueInApi)]
pub enum LayoutNode {
    Pane(PaneId),
    Split {
        direction: Direction,
        children: Vec<(SizeConstraint, LayoutNode)>,
    },
}
impl LayoutNode {
    pub fn split(&mut self, new_id: PaneId, source_id: PaneId, direction: Direction) {
        match self {
            LayoutNode::Pane(pane_id) => {
                if source_id == *pane_id {
                    *self = LayoutNode::Split {
                        direction,
                        children: vec![
                            (SizeConstraint::default(), LayoutNode::Pane(*pane_id)),
                            (SizeConstraint::default(), LayoutNode::Pane(new_id)),
                        ],
                    };
                }
            }
            LayoutNode::Split {
                direction: split_direction,
                children,
            } => {
                if *split_direction == direction {
                    let position = children.iter().position(
                    |(_, node)| matches!(node, LayoutNode::Pane(pane_id) if *pane_id == source_id),
                );
                    if let Some(position) = position {
                        children.insert(
                            position + 1,
                            (SizeConstraint::default(), LayoutNode::Pane(new_id)),
                        );
                    } else {
                        for (_, node) in children {
                            node.split(new_id, source_id, direction);
                        }
                    }
                } else {
                    for (_, node) in children {
                        node.split(new_id, source_id, direction);
                    }
                }
            }
        }
    }
}
