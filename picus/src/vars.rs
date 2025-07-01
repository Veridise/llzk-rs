use std::{
    collections::{HashMap, HashSet},
    fmt,
    hash::Hash,
};

use regex::Regex;

#[derive(Clone)]
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

pub trait VarKind: Hash + Eq + PartialEq {
    fn is_input(&self) -> bool;

    fn is_output(&self) -> bool;

    fn is_temp(&self) -> bool;

    fn temp() -> Self;
}

pub trait VarAllocator {
    type Kind: VarKind + Into<VarStr>;

    fn allocate<K: Into<Self::Kind>>(&self, kind: K) -> VarStr;
}

#[derive(Clone, Default)]
pub struct Vars<K: VarKind>(HashMap<K, VarStr>);

impl<K: VarKind> Vars<K> {
    pub fn inputs<'a>(&'a self) -> impl Iterator<Item = &'a str> {
        self.filter(|k, _| k.is_input())
    }

    pub fn outputs<'a>(&'a self) -> impl Iterator<Item = &'a str> {
        self.filter(|k, _| k.is_output())
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

impl<K: VarKind + Into<VarStr> + Clone> Vars<K> {
    /// Inserts a variable deriving its value from the key. If the key creates a var name that is
    /// already in use it gets uniqued. If the key was already in the vars table returns the
    /// preexisting value.
    pub fn insert(&mut self, key: K) -> VarStr {
        if self.0.contains_key(&key) {
            return self.0[&key].clone();
        }
        let unique_names = self
            .0
            .iter()
            .map(|(_, v)| v.0.as_str())
            .collect::<HashSet<_>>();
        let v = [key.clone().into()]
            .into_iter()
            .cycle()
            .zip(-1..)
            .map(|(v, c)| {
                if c < 0 {
                    v
                } else {
                    format!("{}{}", v.0, c)
                        .try_into()
                        .expect("valid identifier")
                }
            })
            .skip_while(|v| unique_names.contains(v.0.as_str()))
            .next()
            .unwrap();

        self.0.insert(key, v.clone());
        v
    }
}
