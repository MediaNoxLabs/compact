//! Adapter boundary: a validated codec supplies rows regardless of wire schema.
use crate::origins::binding::{Error, MAX_ADDRESSES};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Edge<'a> {
    Root,
    Field(&'a str),
    Index(usize),
}

#[derive(Clone, Copy, Debug)]
pub struct Address<'a> {
    pub parent: Option<usize>,
    pub edge: Edge<'a>,
    pub kind: Option<&'a str>,
}

/// Origin/file/index validity is the codec's premise. This binder additionally
/// validates the complete occurrence shape against the actual IR bytes.
pub trait AddressAccess {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn address(&self, id: usize) -> Address<'_>;
}

pub(crate) struct Graph<'a, A: AddressAccess> {
    pub(crate) source: &'a A,
    children: HashMap<(usize, Edge<'a>), usize>,
    counts: Vec<usize>,
}
impl<'a, A: AddressAccess> Graph<'a, A> {
    pub(crate) fn new(source: &'a A) -> Result<Self, Error> {
        if source.is_empty() || source.len() > MAX_ADDRESSES {
            return Err(Error::Addresses("address count"));
        }
        let mut children = HashMap::with_capacity(source.len().saturating_sub(1));
        let mut counts = vec![0; source.len()];
        for id in 0..source.len() {
            let row = source.address(id);
            if id == 0 {
                if row.parent.is_some() || row.edge != Edge::Root {
                    return Err(Error::Addresses("root"));
                }
            } else {
                let Some(parent) = row.parent else {
                    return Err(Error::Addresses("missing parent"));
                };
                if parent >= id || row.edge == Edge::Root {
                    return Err(Error::Addresses("parent ordering or non-root edge"));
                }
                if children.insert((parent, row.edge), id).is_some() {
                    return Err(Error::Addresses("duplicate child edge"));
                }
                counts[parent] += 1;
            }
        }
        Ok(Self {
            source,
            children,
            counts,
        })
    }
    pub(crate) fn child(&self, parent: usize, edge: Edge<'_>) -> Option<usize> {
        self.children.get(&(parent, edge)).copied()
    }
    pub(crate) fn count(&self, parent: usize) -> usize {
        self.counts[parent]
    }
}
