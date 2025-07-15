use std::{
    fmt,
    iter::Sum,
    ops::{Add, AddAssign},
    rc::Rc,
};

use crate::{felt::IntoPrime, vars::VarKind, Program};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ListPunctuation {
    None,
    Parens,
    Brackets,
    SquareBrackets,
}

impl ListPunctuation {
    pub fn pre(&self) -> &'static str {
        match self {
            ListPunctuation::None => "",
            ListPunctuation::Parens => "(",
            ListPunctuation::Brackets => "[",
            ListPunctuation::SquareBrackets => "{",
        }
    }

    pub fn post(&self) -> &'static str {
        match self {
            ListPunctuation::None => "",
            ListPunctuation::Parens => ")",
            ListPunctuation::Brackets => "]",
            ListPunctuation::SquareBrackets => "}",
        }
    }
}

impl From<&'static str> for ListPunctuation {
    fn from(value: &'static str) -> Self {
        match value {
            "()" => ListPunctuation::Parens,
            "[]" => ListPunctuation::SquareBrackets,
            "{}" => ListPunctuation::Brackets,
            "" => ListPunctuation::None,
            x => panic!(
                "can't create list punctuation with {x:?}. Valid options: \"()\", \"[]\", \"{{}}\", and \"\""
            ),
        }
    }
}

impl Default for ListPunctuation {
    fn default() -> Self {
        Self::Parens
    }
}

struct TRListBase<L> {
    lst: L,
    punct: ListPunctuation,
    breaks_line: bool,
}

impl<L> TRListBase<L> {
    pub fn new(lst: L) -> Self {
        Self {
            lst,
            punct: Default::default(),
            breaks_line: false,
        }
    }

    pub fn with_punct(self, punct: ListPunctuation) -> Self {
        Self {
            lst: self.lst,
            punct,
            breaks_line: self.breaks_line,
        }
    }

    pub fn break_line(self) -> Self {
        self.set_breaks_line(true)
    }

    pub fn no_break_line(self) -> Self {
        self.set_breaks_line(false)
    }

    fn set_breaks_line(self, value: bool) -> Self {
        let mut s = self;
        s.breaks_line = value;
        s
    }

    fn width_common(&self, elt_count: usize, w: usize) -> usize {
        {
            2 + // Opening and closing brackets
                elt_count - 1 + // The spaces between items
                w // The width of each item
        }
    }
}

impl<L: Clone> Clone for TRListBase<L> {
    fn clone(&self) -> Self {
        Self {
            lst: self.lst.clone(),
            punct: self.punct,
            breaks_line: self.breaks_line,
        }
    }
}

impl<L: Copy> Copy for TRListBase<L> {}

impl<L> IntoIterator for TRListBase<L>
where
    L: IntoIterator,
{
    type Item = L::Item;

    type IntoIter = L::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.lst.into_iter()
    }
}

#[derive(Clone)]
pub enum ListItem<'a> {
    Concrete(TextRepresentation<'a>),
    Reference(&'a dyn TextRepresentable),
}

impl ListItem<'_> {
    pub fn width_hint(&self) -> usize {
        match self {
            ListItem::Concrete(c) => c.width(),
            ListItem::Reference(r) => r.width_hint(),
        }
    }
}

impl<'a> From<TextRepresentation<'a>> for ListItem<'a> {
    fn from(value: TextRepresentation<'a>) -> Self {
        Self::Concrete(value)
    }
}

impl<'a> From<&'a dyn TextRepresentable> for ListItem<'a> {
    fn from(value: &'a dyn TextRepresentable) -> Self {
        Self::Reference(value)
    }
}

impl<'a, T: TextRepresentable> From<&'a T> for ListItem<'a> {
    fn from(value: &'a T) -> Self {
        Self::Reference(value)
    }
}

impl<'a> From<&'a str> for ListItem<'a> {
    fn from(value: &'a str) -> Self {
        Self::Concrete(TextRepresentation::atom(value))
    }
}

type TRList<'a> = TRListBase<&'a [&'a dyn TextRepresentable]>;
type TROwnedList<'a> = TRListBase<Vec<ListItem<'a>>>;

impl<'a> From<TRList<'a>> for TROwnedList<'a> {
    fn from(value: TRList<'a>) -> Self {
        Self::new(value.lst.iter().copied().map(Into::into).collect())
    }
}

impl TRList<'_> {
    pub fn width(&self) -> usize {
        let w: usize = self.lst.iter().map(|i| i.width_hint()).sum();
        self.width_common(self.lst.len(), w)
    }
}

impl<'a> TROwnedList<'a> {
    pub fn width(&self) -> usize {
        let w: usize = self.lst.iter().map(|i| i.width_hint()).sum();
        self.width_common(self.lst.len(), w)
    }

    pub fn push<I>(&mut self, i: I)
    where
        I: Into<ListItem<'a>>,
    {
        self.lst.push(i.into())
    }
}

impl<'a> From<TRInner<'a>> for ListItem<'a> {
    fn from(value: TRInner<'a>) -> Self {
        Self::Concrete(value.into())
    }
}

#[derive(Clone)]
enum TRInner<'a> {
    Nothing,
    Atom(&'a str),
    OwnedAtom(String),
    Comment(&'a str),
    OwnedComment(String),
    List(TRList<'a>),
    OwnedList(TROwnedList<'a>),
    Br,
    //Concat(Box<TextRepresentation<'a>>, Box<TextRepresentation<'a>>),
}

impl TRInner<'_> {
    pub fn width(&self) -> usize {
        match self {
            TRInner::Br | TRInner::Nothing => 0,
            TRInner::Atom(s) | TRInner::Comment(s) => s.len(),
            TRInner::OwnedAtom(s) | TRInner::OwnedComment(s) => s.len(),
            TRInner::List(lst) => lst.width(),
            TRInner::OwnedList(lst) => lst.width(),
            //TRInner::Concat(rhs, lhs) => {
            //    rhs.width() + if lhs.breaks_line() { 0 } else { lhs.width() }
            //}
        }
    }
}

impl TextRepresentable for TRInner<'_> {
    fn to_repr(&self) -> TextRepresentation {
        self.clone().into()
    }

    fn width_hint(&self) -> usize {
        self.width()
    }
}

pub struct TextRepresentation<'a> {
    inner: TRInner<'a>,
    force_break: bool,
}

impl<'a> From<TRInner<'a>> for TextRepresentation<'a> {
    fn from(inner: TRInner<'a>) -> Self {
        Self::new(inner)
    }
}

impl<'a> From<TRList<'a>> for TextRepresentation<'a> {
    fn from(value: TRList<'a>) -> Self {
        TextRepresentation::new(TRInner::List(value))
    }
}

impl<'a> From<TROwnedList<'a>> for TextRepresentation<'a> {
    fn from(value: TROwnedList<'a>) -> Self {
        TextRepresentation::new(TRInner::OwnedList(value))
    }
}

macro_rules! owned_list {
    ( $( $x:expr ),* $(,)? ) => {
        TextRepresentation::owned_list(&[ $( $x.into() ),* ])
     };
}

impl<'a> TextRepresentation<'a> {
    pub fn atom(s: &'a str) -> Self {
        TRInner::Atom(s).into()
    }

    pub fn owned_atom(s: String) -> Self {
        TRInner::OwnedAtom(s).into()
    }

    pub fn comment(s: &'a str) -> Self {
        TRInner::Comment(s).into()
    }

    pub fn owned_comment(s: String) -> Self {
        TRInner::OwnedComment(s).into()
    }

    pub fn list(lst: &'a [&'a dyn TextRepresentable]) -> Self {
        TRList::new(lst).into()
    }

    pub fn owned_list(lst: &[ListItem<'a>]) -> Self {
        TROwnedList::new(lst.into()).into()
    }

    pub fn br() -> Self {
        TRInner::Br.into()
    }

    fn new(inner: TRInner<'a>) -> Self {
        Self {
            inner,
            force_break: false,
        }
    }

    pub fn breaks_line(&self) -> bool {
        if self.force_break {
            return true;
        }
        match &self.inner {
            TRInner::Comment(_) | TRInner::Br | TRInner::OwnedComment(_) => true,
            TRInner::List(l) => l.breaks_line,
            TRInner::OwnedList(l) => l.breaks_line,
            //TRInner::Concat(_, rhs) => rhs.breaks_line(),
            _ => false,
        }
    }

    pub fn break_line(self) -> Self {
        match self.inner {
            inner @ TRInner::Atom(_) => Self {
                inner,
                force_break: true,
            },
            inner @ TRInner::OwnedAtom(_) => Self {
                inner,
                force_break: true,
            },
            TRInner::Comment(_) | TRInner::OwnedComment(_) | TRInner::Br | TRInner::Nothing => self,
            TRInner::List(l) => l.break_line().into(),
            TRInner::OwnedList(l) => l.break_line().into(),
            //TRInner::Concat(lhs, rhs) => TRInner::Concat(lhs, Box::new(rhs.break_line())).into(),
        }
    }

    pub fn no_break_line(self) -> Self {
        match self.inner {
            inner @ TRInner::Atom(_) => Self {
                inner,
                force_break: false,
            },
            inner @ TRInner::OwnedAtom(_) => Self {
                inner,
                force_break: false,
            },
            TRInner::Comment(_) | TRInner::OwnedComment(_) | TRInner::Br | TRInner::Nothing => self, // Ignore that order
            TRInner::List(l) => l.no_break_line().into(),
            TRInner::OwnedList(l) => l.no_break_line().into(),
            //TRInner::Concat(lhs, rhs) => TRInner::Concat(lhs, Box::new(rhs.no_break_line())).into(),
        }
    }

    pub fn with_punct(self, punct: ListPunctuation) -> Self {
        match self.inner {
            TRInner::List(lst) => lst.with_punct(punct).into(),
            TRInner::OwnedList(lst) => lst.with_punct(punct).into(),
            //TRInner::Concat(lhs, rhs) => {
            //    TRInner::Concat(lhs, Box::new(rhs.with_punct(punct))).into()
            //}
            x => x.into(),
        }
    }

    pub fn width(&self) -> usize {
        self.inner.width()
    }
}

impl<'a> Add for TextRepresentation<'a> {
    type Output = TextRepresentation<'a>;

    fn add(self, rhs: Self) -> Self::Output {
        fn add_inner<'a>(lhs: TRInner<'a>, rhs: TRInner<'a>) -> TextRepresentation<'a> {
            match (lhs, rhs) {
                (TRInner::Nothing, rhs) => rhs.into(),
                (lhs, TRInner::Nothing) => lhs.into(),
                (lhs, TRInner::Br) => TextRepresentation::from(lhs).break_line(),

                (TRInner::List(lhs), rhs) => add_inner(TRInner::OwnedList(lhs.into()), rhs),
                (TRInner::OwnedList(mut lhs), rhs) => {
                    match rhs {
                        TRInner::List(rhs) => lhs.lst.extend_from_slice(
                            &rhs.lst.iter().copied().map(Into::into).collect::<Vec<_>>(),
                        ),
                        TRInner::OwnedList(rhs) => lhs.lst.extend_from_slice(&rhs.lst),
                        x => lhs.push(x),
                    }
                    lhs.into()
                }
                (lhs, rhs) => {
                    let mut lst = TROwnedList::new(vec![]);
                    lst.push(lhs);
                    add_inner(TRInner::OwnedList(lst), rhs)
                }
            }
        }
        add_inner(self.inner, rhs.inner)
    }
}

impl<'a> Clone for TextRepresentation<'a> {
    fn clone(&self) -> Self {
        match &self.inner {
            TRInner::Nothing => TRInner::Nothing,
            TRInner::Atom(s) => TRInner::Atom(s),
            TRInner::OwnedAtom(s) => TRInner::OwnedAtom(s.clone()),
            TRInner::Comment(s) => TRInner::Comment(s),
            TRInner::OwnedComment(s) => TRInner::OwnedComment(s.clone()),
            TRInner::List(lst) => TRInner::List(*lst),
            TRInner::OwnedList(lst) => TRInner::OwnedList(lst.clone()),
            TRInner::Br => TRInner::Br,
            //TRInner::Concat(lhs, rhs) => TRInner::Concat(lhs.clone(), rhs.clone()),
        }
        .into()
    }
}

impl<'a> AddAssign for TextRepresentation<'a> {
    fn add_assign(&mut self, rhs: Self) {
        match &mut self.inner {
            TRInner::OwnedList(lst) => {
                lst.push(rhs);
            }
            _ => *self = self.clone() + rhs,
        }
    }
}

impl<'a> Sum for TextRepresentation<'a> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(TRInner::Nothing.into(), |acc, tr| acc + tr)
    }
}

pub trait TextRepresentable {
    fn to_repr(&self) -> TextRepresentation;

    fn width_hint(&self) -> usize;

    fn as_tr(&self) -> &dyn TextRepresentable
    where
        Self: Sized,
    {
        self
    }
}

impl TextRepresentable for String {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::atom(self.as_str())
    }

    fn width_hint(&self) -> usize {
        self.len()
    }
}

impl TextRepresentable for str {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::atom(self)
    }

    fn width_hint(&self) -> usize {
        self.len()
    }
}

impl<T: TextRepresentable> TextRepresentable for Vec<T> {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::owned_list(
            &self
                .iter()
                .map(|i| {
                    let x: &dyn TextRepresentable = i;
                    x.into()
                })
                .collect::<Vec<_>>(),
        )
    }

    fn width_hint(&self) -> usize {
        let rec_width: usize = self.iter().map(|i| i.width_hint()).sum();
        {
            2 + // Opening and closing brackets
            self.len() - 1 + // The spaces between items
            rec_width // The width of each item
        }
    }
}

impl TextRepresentable for Vec<&dyn TextRepresentable> {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::list((self.as_slice()) as &[&dyn TextRepresentable])
    }

    fn width_hint(&self) -> usize {
        let rec_width: usize = self.iter().map(|i| i.width_hint()).sum();
        {
            2 + // Opening and closing brackets
            self.len() - 1 + // The spaces between items
            rec_width // The width of each item
        }
    }
}

impl TextRepresentable for Vec<Rc<&dyn TextRepresentable>> {
    fn to_repr(&self) -> TextRepresentation {
        let vec = self
            .iter()
            .map(AsRef::as_ref)
            .copied()
            .map(Into::into)
            .collect::<Vec<_>>();
        TextRepresentation::owned_list(&vec)
    }

    fn width_hint(&self) -> usize {
        let rec_width: usize = self.iter().map(|i| i.width_hint()).sum();
        {
            2 + // Opening and closing brackets
            self.len() - 1 + // The spaces between items
            rec_width // The width of each item
        }
    }
}

pub struct Display<'a, F, K: VarKind> {
    program: &'a Program<F, K>,
}

impl<'a, F, K: VarKind> Display<'a, F, K> {
    pub(crate) fn new(program: &'a Program<F, K>) -> Self {
        Self { program }
    }
}

impl<F: IntoPrime, K: VarKind> fmt::Display for Display<'_, F, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut displayer = Displayer::new(f);
        displayer.fmt(self.program)
    }
}

struct Displayer<'a, 'b> {
    f: &'a mut fmt::Formatter<'b>,
}

impl<'a, 'b> Displayer<'a, 'b> {
    pub fn new(f: &'a mut fmt::Formatter<'b>) -> Self {
        Self { f }
    }

    pub fn fmt(&mut self, repr: &dyn TextRepresentable) -> fmt::Result
where {
        self.fmt_repr(repr.to_repr())
    }

    fn fmt_list<'i, I: ExactSizeIterator + Iterator<Item = ListItem<'i>>>(
        &mut self,
        lst: I,
        punct: ListPunctuation,
    ) -> fmt::Result {
        write!(self.f, "{}", punct.pre())?;
        let len = lst.len();
        for (idx, item) in lst.enumerate() {
            let repr = match item {
                ListItem::Concrete(c) => c,
                ListItem::Reference(r) => r.to_repr(),
            };
            let breaks_line = repr.breaks_line();
            self.fmt_repr(repr)?;
            if (idx + 1) < len && !breaks_line {
                write!(self.f, " ")?;
            }
        }
        write!(self.f, "{}", punct.post())
    }

    fn fmt_repr<'i>(&mut self, repr: TextRepresentation<'i>) -> fmt::Result {
        let breaks_line = repr.breaks_line();
        match repr.inner {
            TRInner::Nothing => write!(self.f, ""),
            TRInner::Br => writeln!(self.f),
            TRInner::Atom(s) => write!(self.f, "{s}"),
            TRInner::OwnedAtom(s) => write!(self.f, "{s}"),
            TRInner::Comment(c) => write!(self.f, "; {c}"),
            TRInner::OwnedComment(c) => write!(self.f, "; {c}"),
            TRInner::List(lst) => self.fmt_list(lst.lst.iter().copied().map(Into::into), lst.punct),
            TRInner::OwnedList(lst) => self.fmt_list(lst.lst.into_iter(), lst.punct),
        }?;
        if breaks_line {
            writeln!(self.f)
        } else {
            write!(self.f, "")
        }
    }
}
