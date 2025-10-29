use std::{
    collections::{HashMap, HashSet},
    fmt,
    hash::Hash,
    ops::Index,
};

use patricia_tree::StringPatriciaSet;

use crate::{
    display::{TextRepresentable, TextRepresentation},
    ident::{Ident, VALID_IDENT},
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct VarStr(String);

impl TryFrom<String> for VarStr {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if !VALID_IDENT.is_match(value.as_str()) {
            anyhow::bail!("String \"{value}\" is not a valid Picus identifier");
        }
        Ok(Self(value))
    }
}

impl AsRef<str> for VarStr {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl From<Ident> for VarStr {
    fn from(value: Ident) -> Self {
        Self(value.value().clone())
    }
}

impl fmt::Display for VarStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TextRepresentable for VarStr {
    fn to_repr(&self) -> TextRepresentation<'_> {
        self.0.to_repr()
    }

    fn width_hint(&self) -> usize {
        self.0.len()
    }
}

pub trait VarKind: Hash + Eq + PartialEq + fmt::Debug {
    fn is_input(&self) -> bool;

    fn get_input_no(&self) -> Option<usize>;

    fn is_output(&self) -> bool;

    fn get_output_no(&self) -> Option<usize>;

    fn is_temp(&self) -> bool;
}

pub trait Temp<'o>: VarKind + Sized {
    type Ctx: Copy;
    type Output: Into<Self> + Into<VarStr> + Clone + 'o;

    fn temp(ctx: Self::Ctx) -> Self::Output;
}

pub trait VarAllocator {
    type Kind: VarKind;

    fn allocate<K: Into<Self::Kind> + Into<VarStr> + Clone>(&self, kind: K) -> VarStr;
}

#[derive(Clone, Debug, Default)]
pub struct Vars<K: VarKind> {
    map: HashMap<K, VarStr>,
    unique: HashSet<String>,
    //unique: StringPatriciaSet,
}

impl<K: VarKind> Vars<K> {
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.map.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &VarStr)> {
        self.map.iter()
    }

    /// Lookup a var's key in the vars table. This operation is linear.
    pub fn lookup_key(&self, var: &VarStr) -> Option<&K> {
        self.map.iter().find(|(_, v)| **v == *var).map(|(k, _)| k)
    }

    /// Returns the inputs in the environment sorted by their key (which matches declaration
    /// order).
    pub fn inputs(&self) -> impl Iterator<Item = &str> {
        let mut inputs = self
            .map
            .iter()
            .filter_map(|(k, v)| k.get_input_no().map(|no| (no, v)))
            .collect::<Vec<_>>();
        inputs.sort_by_key(|(k, _)| *k);
        inputs.into_iter().map(|(_, v)| v.0.as_str())
    }

    /// Returns the outputs in the environment sorted by their key (which matches declaration
    /// order).
    pub fn outputs(&self) -> impl Iterator<Item = &str> {
        let mut outputs = self
            .map
            .iter()
            .filter_map(|(k, v)| k.get_output_no().map(|no| (no, v)))
            .collect::<Vec<_>>();
        outputs.sort_by_key(|(k, _)| *k);
        outputs.into_iter().map(|(_, v)| v.0.as_str())
    }

    pub fn temporaries(&self) -> impl Iterator<Item = &str> {
        self.filter(|k, _| k.is_temp())
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    fn filter<'a, P>(&'a self, p: P) -> impl Iterator<Item = &'a str>
    where
        P: Fn(&'a K, &'a VarStr) -> bool + 'a,
    {
        self.map.iter().filter_map(move |(k, v)| -> Option<&str> {
            if p(k, v) { Some(&v.0) } else { None }
        })
    }

    /// Inserts a variable using the given VarStr. The behavior mimics `HashMap::insert`.
    pub fn insert_with_value(&mut self, key: K, v: VarStr) {
        self.map.insert(key, v.clone());
        self.unique.insert(v.0);
    }
}

impl<K: VarKind> Index<&K> for Vars<K> {
    type Output = VarStr;

    fn index(&self, index: &K) -> &Self::Output {
        &self.map[index]
    }
}

impl<K: VarKind> IntoIterator for Vars<K> {
    type Item = <HashMap<K, VarStr> as IntoIterator>::Item;

    type IntoIter = <HashMap<K, VarStr> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<K: VarKind, S: Into<K> + Into<VarStr> + Clone> FromIterator<S> for Vars<K> {
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        let iter: Vec<(K, VarStr)> = iter
            .into_iter()
            .map(|seed| (seed.clone().into(), seed.into()))
            .collect();
        let unique: HashSet<String> = iter.iter().map(|(_, v)| v.0.clone()).collect();

        let map = HashMap::from_iter(iter);
        assert_eq!(map.len(), unique.len());
        Self { map, unique }
    }
}

impl<K: VarKind, S: Into<K> + Into<VarStr> + Clone> Extend<S> for Vars<K> {
    fn extend<T: IntoIterator<Item = S>>(&mut self, iter: T) {
        let iter: Vec<(K, VarStr)> = iter
            .into_iter()
            .map(|seed| (seed.clone().into(), seed.into()))
            .inspect(|(key, var)| log::debug!("Priming vars db with key {key:?} and var {var:?}"))
            .collect();
        self.unique.extend(iter.iter().map(|(_, v)| v.0.clone()));
        self.map.extend(iter);

        assert_eq!(self.map.len(), self.unique.len());
    }
}

impl<'a, K: VarKind> IntoIterator for &'a Vars<K> {
    type Item = <&'a HashMap<K, VarStr> as IntoIterator>::Item;

    type IntoIter = <&'a HashMap<K, VarStr> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl<K: VarKind + Clone + fmt::Debug> Vars<K> {
    /// Inserts a variable deriving its value from the key seed. If the key creates a var name that is
    /// already in use it gets uniqued. If the key was already in the vars table returns the
    /// preexisting value.
    pub fn insert<S: Into<K> + Into<VarStr> + Clone>(&mut self, seed: S) -> VarStr {
        let key = seed.clone().into();
        log::debug!("[{self:p}] Inserting var key {key:?}");
        if self.map.contains_key(&key) {
            let prev_name = self.map[&key].clone();
            let new_name: VarStr = seed.into();
            log::debug!(
                "[{self:p}]  Key {key:?} was already inserted. Cached name is {prev_name:?} and the generated name is {new_name:?}"
            );
            return prev_name;
        }
        log::debug!("[{self:p}]  Unique names: {:?}", self.unique);
        let v = [seed.into()]
            .into_iter()
            .cycle()
            .zip(0..)
            .map(|(v, c): (VarStr, i32)| {
                if c == 0 {
                    v
                } else {
                    format!("{v}__{c}").try_into().expect("valid identifier")
                }
            })
            .inspect(|v| log::debug!("[{self:p}]  Testing if {v:?} is a fresh name"))
            .find(|v| !self.unique.contains(&v.0))
            .unwrap();

        self.insert_with_value(key.clone(), v.clone());
        log::debug!("[{self:p}]  Returning {v:?} as the variable name for key {key:?}");
        v
    }
}
