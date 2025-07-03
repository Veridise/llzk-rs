use std::{fmt, rc::Rc};

use super::traits::StmtDisplay;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ListPunctuation {
    Parens,
    Brackets,
    SquareBrackets,
}

impl ListPunctuation {
    pub fn pre(&self) -> &'static str {
        match self {
            ListPunctuation::Parens => "(",
            ListPunctuation::Brackets => "[",
            ListPunctuation::SquareBrackets => "{",
        }
    }

    pub fn post(&self) -> &'static str {
        match self {
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
            x => panic!(
                "can't create list punctuation with {x:?}. Valid options: \"()\", \"[]\", and \"{{}}\" "
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

    fn width_impl(&self, items: &[&dyn TextRepresentable]) -> usize {
        let w: usize = items.iter().copied().map(|i| i.width_hint()).sum();
        {
            2 + // Opening and closing brackets
                items.len() - 1 + // The spaces between items
                w // The width of each item
        }
    }
}

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

type TRList<'a> = TRListBase<&'a [&'a dyn TextRepresentable]>;
type TROwnedList<'a> = TRListBase<Vec<&'a dyn TextRepresentable>>;

impl TRList<'_> {
    pub fn width(&self) -> usize {
        self.width_impl(self.lst)
    }
}

impl TROwnedList<'_> {
    pub fn width(&self) -> usize {
        self.width_impl(&self.lst)
    }
}

enum TRInner<'a> {
    Atom(&'a str),
    OwnedAtom(String),
    Comment(&'a str),
    List(TRList<'a>),
    OwnedList(TROwnedList<'a>),
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

    pub fn list(lst: &'a [&'a dyn TextRepresentable]) -> Self {
        TRList::new(lst).into()
    }

    pub fn owned_list(lst: Vec<&'a dyn TextRepresentable>) -> Self {
        TROwnedList::new(lst).into()
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
            TRInner::Comment(_) => true,
            TRInner::List(l) => l.breaks_line,
            TRInner::OwnedList(l) => l.breaks_line,
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
            TRInner::Comment(_) => self,
            TRInner::List(l) => l.break_line().into(),
            TRInner::OwnedList(l) => l.break_line().into(),
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
            TRInner::Comment(_) => self, // Ignore that order
            TRInner::List(l) => l.no_break_line().into(),
            TRInner::OwnedList(l) => l.no_break_line().into(),
        }
    }

    pub fn with_punct(self, punct: ListPunctuation) -> Self {
        match self.inner {
            TRInner::List(lst) => lst.with_punct(punct).into(),
            TRInner::OwnedList(lst) => lst.with_punct(punct).into(),
            x => x.into(),
        }
    }

    pub fn width(&self) -> usize {
        match &self.inner {
            TRInner::Atom(s) | TRInner::Comment(s) => s.len(),
            TRInner::OwnedAtom(s) => s.len(),
            TRInner::List(lst) => lst.width(),
            TRInner::OwnedList(lst) => lst.width(),
        }
    }
}

pub trait TextRepresentable {
    fn to_repr(&self) -> TextRepresentation;

    fn width_hint(&self) -> usize;
}

impl TextRepresentable for String {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::atom(self.as_str())
    }

    fn width_hint(&self) -> usize {
        self.len()
    }
}

impl TextRepresentable for &str {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::atom(self)
    }

    fn width_hint(&self) -> usize {
        self.len()
    }
}

pub(crate) fn to_repr_ref<T: TextRepresentable>(t: &T) -> &dyn TextRepresentable {
    t
}

impl<T: TextRepresentable> TextRepresentable for Vec<T> {
    fn to_repr(&self) -> TextRepresentation {
        TextRepresentation::owned_list(self.iter().map(to_repr_ref).collect())
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
        let vec = self.iter().map(AsRef::as_ref).copied().collect::<Vec<_>>();
        TextRepresentation::owned_list(vec)
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

pub struct Display<S: StmtDisplay> {
    stmt: S,
}

impl<S: StmtDisplay> Display<S> {
    pub(crate) fn new(stmt: S) -> Self {
        Self { stmt }
    }
}

impl<S: StmtDisplay> fmt::Display for Display<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut displayer = Displayer::new(f);
        displayer.fmt(self.stmt.as_ref())
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

    fn fmt_list(&mut self, lst: &[&dyn TextRepresentable], punct: ListPunctuation) -> fmt::Result {
        write!(self.f, "{}", punct.pre())?;
        for (idx, tr) in lst.iter().copied().enumerate() {
            self.fmt(tr)?;
            if (idx + 1) < lst.len() {
                write!(self.f, " ")?;
            }
        }
        write!(self.f, "{}", punct.post())
    }

    fn fmt_repr(&mut self, repr: TextRepresentation) -> fmt::Result {
        let breaks_line = repr.breaks_line();
        match repr.inner {
            TRInner::Atom(s) => write!(self.f, "{s}"),
            TRInner::OwnedAtom(s) => write!(self.f, "{s}"),
            TRInner::Comment(c) => write!(self.f, "; {c}"),
            TRInner::List(lst) => self.fmt_list(lst.lst, lst.punct),
            TRInner::OwnedList(lst) => self.fmt_list(&lst.lst, lst.punct),
        }?;
        if breaks_line {
            writeln!(self.f)
        } else {
            write!(self.f, "")
        }
    }
}
