use anyhow::Result;

use crate::backend::lowering::{Lowering, lowerable::LowerableStmt};

pub struct Comment(String);

impl Comment {
    pub fn new(s: impl AsRef<str>) -> Self {
        Self(s.as_ref().to_owned())
    }

    pub fn value(&self) -> &str {
        self.0.as_str()
    }
}

impl LowerableStmt for Comment {
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        l.generate_comment(self.0)
    }
}

impl Clone for Comment {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl PartialEq for Comment {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::fmt::Debug for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "comment '{}'", self.0)
    }
}
