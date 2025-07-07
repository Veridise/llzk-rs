use std::{
    collections::{HashMap, HashSet},
    fmt,
    hash::Hash,
};

use regex::Regex;

use crate::display::{TextRepresentable, TextRepresentation};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct VarStr(String);

impl TryFrom<String> for VarStr {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let re = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
        if !re.is_match(value.as_str()) {
            anyhow::bail!("String \"{value}\" is not a valid Picus identifier");
        }
        Ok(Self(value))
    }
}

impl fmt::Display for VarStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TextRepresentable for VarStr {
    fn to_repr(&self) -> TextRepresentation {
        self.0.to_repr()
    }

    fn width_hint(&self) -> usize {
        self.0.len()
    }
}

pub trait VarKind: Hash + Eq + PartialEq {
    fn is_input(&self) -> bool;

    fn is_output(&self) -> bool;

    fn is_temp(&self) -> bool;
}

pub trait Temp: VarKind + Sized {
    type Output: Into<Self> + Into<VarStr> + Clone;

    fn temp() -> Self::Output;
}

pub trait VarAllocator {
    type Kind: VarKind;

    fn allocate<K: Into<Self::Kind> + Into<VarStr> + Clone>(&self, kind: K) -> VarStr;
}

#[derive(Clone, Default)]
pub struct Vars<K: VarKind>(HashMap<K, VarStr>);

impl<K: VarKind> Vars<K> {
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &VarStr)> {
        self.0.iter()
    }

    pub fn inputs(&self) -> impl Iterator<Item = &str> {
        self.filter(|k, _| k.is_input())
    }

    pub fn outputs(&self) -> impl Iterator<Item = &str> {
        self.filter(|k, _| k.is_output())
    }

    pub fn temporaries(&self) -> impl Iterator<Item = &str> {
        self.filter(|k, _| k.is_temp())
    }

    fn filter<'a, P>(&'a self, p: P) -> impl Iterator<Item = &'a str>
    where
        P: Fn(&'a K, &'a VarStr) -> bool + 'a,
    {
        self.0.iter().filter_map(move |(k, v)| -> Option<&str> {
            if p(k, v) {
                Some(&v.0)
            } else {
                None
            }
        })
    }

    /// Inserts a variable using the given VarStr. The behavior mimics `HashMap::insert`.
    pub fn insert_with_value(&mut self, key: K, v: VarStr) {
        self.0.insert(key, v);
    }
}

impl<K: VarKind> IntoIterator for Vars<K> {
    type Item = <HashMap<K, VarStr> as IntoIterator>::Item;

    type IntoIter = <HashMap<K, VarStr> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K: VarKind> IntoIterator for &'a Vars<K> {
    type Item = <&'a HashMap<K, VarStr> as IntoIterator>::Item;

    type IntoIter = <&'a HashMap<K, VarStr> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<K: VarKind + Clone> Vars<K> {
    /// Inserts a variable deriving its value from the key seed. If the key creates a var name that is
    /// already in use it gets uniqued. If the key was already in the vars table returns the
    /// preexisting value.
    pub fn insert<S: Into<K> + Into<VarStr> + Clone>(&mut self, seed: S) -> VarStr {
        let key = seed.clone().into();

        if self.0.contains_key(&key) {
            return self.0[&key].clone();
        }
        let unique_names = self
            .0
            .values()
            .map(|v| v.0.as_str())
            .collect::<HashSet<_>>();
        let v = [seed.into()]
            .into_iter()
            .cycle()
            .zip(0..)
            .map(|(v, c): (VarStr, i32)| {
                if c == 0 {
                    v
                } else {
                    format!("{}{}", v, c + 1)
                        .try_into()
                        .expect("valid identifier")
                }
            })
            .find(|v| !unique_names.contains(v.0.as_str()))
            .unwrap();

        self.0.insert(key, v.clone());
        v
    }
}
