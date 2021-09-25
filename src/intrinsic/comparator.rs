use crate::{ast::Comparator, vm::bytecode::NamedFn2, Value};
use itertools::Itertools;
use std::cmp::Ordering;

pub(crate) fn comparator(comparator: &Comparator) -> NamedFn2 {
    match comparator {
        Comparator::Eq => NamedFn2 {
            name: "Equal",
            func: Box::new(|lhs, rhs| {
                Ok(if compare(lhs, rhs).is_eq() {
                    Value::True
                } else {
                    Value::False
                })
            }),
        },
        Comparator::Neq => NamedFn2 {
            name: "NotEqual",
            func: Box::new(|lhs, rhs| {
                Ok(if compare(lhs, rhs).is_ne() {
                    Value::True
                } else {
                    Value::False
                })
            }),
        },
        Comparator::Gt => NamedFn2 {
            name: "GreaterThan",
            func: Box::new(|lhs, rhs| {
                Ok(if compare(lhs, rhs).is_gt() {
                    Value::True
                } else {
                    Value::False
                })
            }),
        },
        Comparator::Ge => NamedFn2 {
            name: "GreaterOrEqual",
            func: Box::new(|lhs, rhs| {
                Ok(if compare(lhs, rhs).is_ge() {
                    Value::True
                } else {
                    Value::False
                })
            }),
        },
        Comparator::Lt => NamedFn2 {
            name: "LessThan",
            func: Box::new(|lhs, rhs| {
                Ok(if compare(lhs, rhs).is_lt() {
                    Value::True
                } else {
                    Value::False
                })
            }),
        },
        Comparator::Le => NamedFn2 {
            name: "LessOrEqual",
            func: Box::new(|lhs, rhs| {
                Ok(if compare(lhs, rhs).is_le() {
                    Value::True
                } else {
                    Value::False
                })
            }),
        },
    }
}

fn compare(lhs: Value, rhs: Value) -> Ordering {
    Ord::cmp(&ComparableValue(&lhs), &ComparableValue(&rhs))
}

struct ComparableValue<'a>(&'a Value);

impl<'a> Eq for ComparableValue<'a> {}

impl<'a> PartialEq<Self> for ComparableValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        Ord::cmp(self, other).is_eq()
    }
}

impl<'a> PartialOrd<Self> for ComparableValue<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl<'a> Ord for ComparableValue<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        use Ordering::*;
        use Value::*;
        fn type_ord(value: &Value) -> u8 {
            match value {
                Null => 0,
                False => 1,
                True => 2,
                Number(_) => 3,
                String(_) => 4,
                Array(_) => 5,
                Object(_) => 6,
            }
        }
        if let res @ (Less | Greater) = type_ord(self.0).cmp(&type_ord(other.0)) {
            return res;
        }
        match (&self.0, &other.0) {
            (Null, Null) | (True, True) | (False, False) => Equal,
            (Number(lhs), Number(rhs)) => Ord::cmp(&lhs, &rhs),
            (String(lhs), String(rhs)) => Ord::cmp(&lhs, &rhs),
            (Array(lhs), Array(rhs)) => Iterator::cmp(
                lhs.iter().map(ComparableValue),
                rhs.iter().map(ComparableValue),
            ),
            (Object(lhs), Object(rhs)) => {
                let lhs_keys = lhs.keys().sorted().collect_vec();
                let rhs_keys = rhs.keys().sorted().collect_vec();
                if let res @ (Less | Greater) = Iterator::cmp(lhs_keys.iter(), rhs_keys.iter()) {
                    return res;
                }
                for key in lhs_keys {
                    if let res @ (Less | Greater) = Ord::cmp(
                        &lhs.get(key).map(ComparableValue),
                        &rhs.get(key).map(ComparableValue),
                    ) {
                        return res;
                    }
                }
                Equal
            }
            (Null | True | False | Number(_) | String(_) | Array(_) | Object(_), _) => {
                unreachable!()
            }
        }
    }
}