use std::{
    borrow::{Borrow, Cow},
    ops::Deref,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RegionNameStatus {
    Unassigned,
    Auto,
    Assigned,
}

impl Default for RegionNameStatus {
    fn default() -> Self {
        Self::Unassigned
    }
}

#[derive(Default, Debug)]
pub struct RegionName {
    status: RegionNameStatus,
    name: Option<Cow<'static, str>>,
}

impl AsRef<str> for RegionName {
    fn as_ref(&self) -> &str {
        match &self.name {
            Some(name) => name.as_ref(),
            None => match self.status {
                RegionNameStatus::Unassigned => "<Unassigned>",
                s => panic!("Inconsistency detected: No name was assigned but has status {s:?}"),
            },
        }
    }
}

impl std::fmt::Display for RegionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl Deref for RegionName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Borrow<str> for RegionName {
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}

impl<T> PartialEq<T> for RegionName
where
    for<'s> &'s str: PartialEq<T>,
{
    fn eq(&self, other: &T) -> bool {
        self.as_ref() == *other
    }
}

impl RegionName {
    pub fn is_unassigned(&self) -> bool {
        self.status == RegionNameStatus::Unassigned
    }

    pub fn is_assigned(&self) -> bool {
        self.status == RegionNameStatus::Assigned
    }

    pub fn has_value(&self) -> bool {
        !self.is_unassigned() && self.name.is_some()
    }

    pub fn auto<N, NR>(&mut self, name: &N)
    where
        N: Fn() -> NR,
        NR: Into<String>,
    {
        // Only assign if nothing has been assigned so far.
        if !self.is_unassigned() {
            return;
        }
        self.assign_with_status(RegionNameStatus::Auto, Cow::Owned(name().into()));
    }

    fn assign_with_status(&mut self, status: RegionNameStatus, name: Cow<'static, str>) {
        self.status = status;
        self.name = Some(name);
    }

    fn assign(&mut self, name: Cow<'static, str>) {
        assert!(!self.is_assigned(), "Double manual assignment!");
        self.assign_with_status(RegionNameStatus::Assigned, name)
    }

    /// Assigns a name to the region. Panics if the name has been previously assigned manually.
    /// If the name wasn't assigned or it was assigned automatically overrides it.
    pub fn assign_from_ref(&mut self, name: &'static str) {
        self.assign(Cow::Borrowed(name))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default_is_unassigned() {
        let name = RegionName::default();
        assert!(name.is_unassigned());
    }

    #[test]
    fn auto_overrides_unassigned() {
        let mut name = RegionName::default();
        name.auto(&|| "test name");

        assert!(name.has_value());
        assert_eq!(name, "test name");
    }

    #[test]
    fn auto_doesnt_override_assigned() {
        let mut name = RegionName {
            status: RegionNameStatus::Assigned,
            name: Some(Cow::Borrowed("assigned name")),
        };
        name.auto(&|| "test name");

        assert!(name.is_assigned());
        assert_eq!(name, "assigned name");
    }

    #[test]
    fn auto_doesnt_override_auto() {
        let mut name = RegionName::default();
        name.auto(&|| "test name");
        name.auto(&|| "test name 2");

        assert!(name.has_value());
        assert_eq!(name, "test name");
    }

    #[test]
    fn assigned_overrides_unassigned() {
        let mut name = RegionName::default();
        name.assign_from_ref("test name");

        assert!(name.has_value());
        assert_eq!(name, "test name");
    }

    #[test]
    fn assigned_overrides_auto() {
        let mut name = RegionName::default();
        name.auto(&|| "test name");
        name.assign_from_ref("assigned name");

        assert!(name.has_value());
        assert_eq!(name, "assigned name");
    }

    #[test]
    #[should_panic]
    fn double_assigned_panics() {
        let mut name = RegionName::default();
        name.assign_from_ref("test name");
        name.assign_from_ref("assigned name"); // Panics here
    }
}
