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
    pub fn apply_offset(&mut self, offset: (u16, u16)) {
        self.offset.0 += offset.0;
        self.offset.1 += offset.1;
    }
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ConvertValueInApi)]
pub struct SizeRange(pub u16, pub u16);
impl std::ops::Add for SizeRange {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0), self.1.saturating_add(rhs.1))
    }
}
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
impl std::ops::Add for SizeConstraint {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            weight: (self.weight.0 + rhs.weight.0, self.weight.1 + rhs.weight.1),
            range: (self.range.0 + rhs.range.0, self.range.1 + rhs.range.1),
        }
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

    SizeRange(start, last)
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
    fn zero() -> Self {
        Self {
            weight: (0.0, 0.0),
            range: (SizeRange(0, 0), SizeRange(0, 0)),
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
        children: Vec<LayoutNode>,
    },
}
impl LayoutNode {
    pub fn pane_ids(&self) -> Vec<PaneId> {
        match self {
            LayoutNode::Pane(id) => vec![*id],
            LayoutNode::Split { children, .. } => {
                children.iter().flat_map(Self::pane_ids).collect()
            }
        }
    }
    pub fn split(&mut self, new_id: PaneId, source_id: PaneId, direction: Direction) {
        match self {
            LayoutNode::Pane(pane_id) => {
                if source_id == *pane_id {
                    *self = LayoutNode::Split {
                        direction,
                        children: vec![LayoutNode::Pane(*pane_id), LayoutNode::Pane(new_id)],
                    };
                }
            }
            LayoutNode::Split {
                direction: split_direction,
                children,
            } => {
                if *split_direction == direction {
                    let position = children.iter().position(
                        |node| matches!(node, LayoutNode::Pane(pane_id) if *pane_id == source_id),
                    );
                    if let Some(position) = position {
                        children.insert(position + 1, LayoutNode::Pane(new_id));
                    } else {
                        for node in children {
                            node.split(new_id, source_id, direction);
                        }
                    }
                } else {
                    for node in children {
                        node.split(new_id, source_id, direction);
                    }
                }
            }
        }
    }
    pub fn with_size_constraint(
        &self,
        size_constraints: &[(PaneId, SizeConstraint)],
    ) -> LayoutNodeWithSizeConstraint {
        match self {
            LayoutNode::Pane(pane_id) => {
                let size_constraint = size_constraints
                    .iter()
                    .find_map(|(id, size_constraints)| (id == pane_id).then_some(*size_constraints))
                    .unwrap_or_default();
                LayoutNodeWithSizeConstraint::Pane((*pane_id, size_constraint))
            }
            LayoutNode::Split {
                direction,
                children,
            } => {
                let children: Vec<_> = children
                    .iter()
                    .map(|node| {
                        let node_with_size_constraint = node.with_size_constraint(size_constraints);
                        let size_constraint = node_with_size_constraint.size_constraint();
                        (size_constraint, node_with_size_constraint)
                    })
                    .collect();
                LayoutNodeWithSizeConstraint::Split {
                    direction: *direction,
                    children,
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ConvertValueInApi)]
pub enum LayoutNodeWithSizeConstraint {
    Pane((PaneId, SizeConstraint)),
    Split {
        direction: Direction,
        children: Vec<(SizeConstraint, LayoutNodeWithSizeConstraint)>,
    },
}
impl LayoutNodeWithSizeConstraint {
    pub fn size_constraint(&self) -> SizeConstraint {
        match self {
            Self::Pane((_, size_constraint)) => *size_constraint,
            Self::Split { children, .. } => children
                .iter()
                .map(|(size_constraint, _)| *size_constraint)
                .fold(SizeConstraint::zero(), |acc, x| acc + x),
        }
    }
}
