use crate::core::{Attributes, AttributesData, Delta, Interval, Operation};
use std::{cmp::min, slice::Iter};

pub struct Cursor<'a> {
    delta: &'a Delta,
    interval: Interval,
    iterator: Iter<'a, Operation>,
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(delta: &'a Delta, interval: Interval) -> Cursor<'a> {
        let mut cursor = Self {
            delta,
            interval,
            iterator: delta.ops.iter(),
            offset: 0,
        };
        cursor
    }

    pub fn next_op(&mut self) -> Option<Operation> {
        let mut next_op = self.iterator.next();
        let mut find_op = None;

        while find_op.is_none() && next_op.is_some() {
            let op = next_op.unwrap();
            if self.offset < self.interval.start {
                let intersect =
                    Interval::new(self.offset, self.offset + op.length()).intersect(self.interval);
                if intersect.is_empty() {
                    self.offset += op.length();
                } else {
                    if let Some(new_op) = op.shrink(intersect.translate_neg(self.offset)) {
                        // shrink the op to fit the intersect range
                        // ┌──────────────┐
                        // │ 1 2 3 4 5 6  │
                        // └───────▲───▲──┘
                        //         │   │
                        //        [3, 5)
                        // op = "45"
                        find_op = Some(new_op);
                    }
                    self.offset = intersect.end;
                }
            } else {
                // the interval passed in the shrink function is base on the op not the delta.
                if let Some(new_op) = op.shrink(self.interval.translate_neg(self.offset)) {
                    find_op = Some(new_op);
                }
                // for example: extract the ops from three insert ops with interval [2,5). the
                // interval size is larger than the op. Moving the offset to extract each part.
                // Each step would be the small value between interval.size() and
                // next_op.length(). Checkout the delta_get_ops_in_interval_4 for more details.
                //
                // ┌──────┐  ┌──────┐  ┌──────┐
                // │ 1 2  │  │ 3 4  │  │ 5 6  │
                // └──────┘  └─▲────┘  └───▲──┘
                //             │  [2, 5)   │
                //
                self.offset += min(self.interval.size(), op.length());
            }

            match find_op {
                None => next_op = self.iterator.next(),
                Some(_) => self.interval.start = self.offset,
            }
        }

        find_op
    }
}

pub struct DeltaIter<'a> {
    cursor: Cursor<'a>,
    interval: Interval,
}

impl<'a> DeltaIter<'a> {
    pub fn new(delta: &'a Delta) -> Self {
        let interval = Interval::new(0, usize::MAX);
        Self::from_interval(delta, interval)
    }

    pub fn from_interval(delta: &'a Delta, interval: Interval) -> Self {
        let cursor = Cursor::new(delta, interval);
        Self { cursor, interval }
    }

    pub fn ops(&mut self) -> Vec<Operation> { self.collect::<Vec<_>>() }
}

impl<'a> Iterator for DeltaIter<'a> {
    type Item = Operation;
    fn next(&mut self) -> Option<Self::Item> { self.cursor.next_op() }
}

pub struct DeltaAttributesIter<'a> {
    delta_iter: DeltaIter<'a>,
    interval: Interval,
}

impl<'a> DeltaAttributesIter<'a> {
    pub fn new(delta: &'a Delta) -> Self {
        let interval = Interval::new(0, usize::MAX);
        Self::from_interval(delta, interval)
    }

    pub fn from_interval(delta: &'a Delta, interval: Interval) -> Self {
        let delta_iter = DeltaIter::from_interval(delta, interval);
        Self {
            delta_iter,
            interval,
        }
    }
}

impl<'a> Iterator for DeltaAttributesIter<'a> {
    type Item = (usize, Attributes);
    fn next(&mut self) -> Option<Self::Item> {
        let next_op = self.delta_iter.next();
        if next_op.is_none() {
            return None;
        }
        let mut length: usize = 0;
        let mut attributes_data = AttributesData::new();

        match next_op.unwrap() {
            Operation::Delete(_n) => {},
            Operation::Retain(retain) => {
                if let Attributes::Custom(data) = &retain.attributes {
                    log::debug!("extend retain attributes with {} ", &data);
                    attributes_data.extend(Some(data.clone()));
                }
                length = retain.n;
            },
            Operation::Insert(insert) => {
                if let Attributes::Custom(data) = &insert.attributes {
                    log::debug!("extend insert attributes with {} ", &data);
                    attributes_data.extend(Some(data.clone()));
                }
                length = insert.num_chars();
            },
        }

        let attribute: Attributes = attributes_data.into();
        Some((length, attribute))
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test() {}
}