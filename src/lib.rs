//! This crate defines a
//! [Wadler-style](http://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf)
//! pretty-printing API.
//!
//! Start with with the static functions of [Doc](enum.Doc.html).
//!
//! ## Quick start
//!
//! Let's pretty-print simple sexps!  We want to pretty print sexps like
//!
//! ```lisp
//! (1 2 3)
//! ```
//! or, if the line would be too long, like
//!
//! ```lisp
//! ((1)
//!  (2 3)
//!  (4 5 6))
//! ```
//!
//! A _simple symbolic expression_ consists of a numeric _atom_ or a nested ordered _list_ of
//! symbolic expression children.
//!
//! ```rust
//! # extern crate pretty;
//! # use pretty::*;
//! enum SExp {
//!     Atom(u32),
//!     List(Vec<SExp>),
//! }
//! use SExp::*;
//! # fn main() { }
//! ```
//!
//! We define a simple conversion to a [Doc](enum.Doc.html).  Atoms are rendered as strings; lists
//! are recursively rendered, with spaces between children where appropriate.  Children are
//! [nested]() and [grouped](), allowing them to be laid out in a single line as appropriate.
//!
//! ```rust
//! # extern crate pretty;
//! # use pretty::*;
//! # enum SExp {
//! #     Atom(u32),
//! #     List(Vec<SExp>),
//! # }
//! # use SExp::*;
//! impl SExp {
//!     /// Return a pretty printed format of self.
//!     pub fn to_doc(&self) -> Doc<BoxDoc> {
//!         match self {
//!             &Atom(x) => Doc::as_string(x),
//!             &List(ref xs) =>
//!                 Doc::text("(")
//!                     .append(Doc::intersperse(xs.into_iter().map(|x| x.to_doc()), Doc::space()).nest(1).group())
//!                     .append(Doc::text(")"))
//!         }
//!     }
//! }
//! # fn main() { }
//! ```
//!
//! Next, we convert the [Doc](enum.Doc.html) to a plain old string.
//!
//! ```rust
//! # extern crate pretty;
//! # use pretty::*;
//! # enum SExp {
//! #     Atom(u32),
//! #     List(Vec<SExp>),
//! # }
//! # use SExp::*;
//! # impl SExp {
//! #     /// Return a pretty printed format of self.
//! #     pub fn to_doc(&self) -> Doc<BoxDoc> {
//! #         match self {
//! #             &Atom(x) => Doc::as_string(x),
//! #             &List(ref xs) =>
//! #                 Doc::text("(")
//! #                     .append(Doc::intersperse(xs.into_iter().map(|x| x.to_doc()), Doc::space()).nest(1).group())
//! #                     .append(Doc::text(")"))
//! #         }
//! #     }
//! # }
//! impl SExp {
//!     pub fn to_pretty(&self, width: usize) -> String {
//!         let mut w = Vec::new();
//!         self.to_doc().render(width, &mut w).unwrap();
//!         String::from_utf8(w).unwrap()
//!     }
//! }
//! # fn main() { }
//! ```
//!
//! And finally we can test that the nesting and grouping behaves as we expected.
//!
//! ```rust
//! # extern crate pretty;
//! # use pretty::*;
//! # enum SExp {
//! #     Atom(u32),
//! #     List(Vec<SExp>),
//! # }
//! # use SExp::*;
//! # impl SExp {
//! #     /// Return a pretty printed format of self.
//! #     pub fn to_doc(&self) -> Doc<BoxDoc> {
//! #         match self {
//! #             &Atom(x) => Doc::as_string(x),
//! #             &List(ref xs) =>
//! #                 Doc::text("(")
//! #                     .append(Doc::intersperse(xs.into_iter().map(|x| x.to_doc()), Doc::space()).nest(1).group())
//! #                     .append(Doc::text(")"))
//! #         }
//! #     }
//! # }
//! # impl SExp {
//! #     pub fn to_pretty(&self, width: usize) -> String {
//! #         let mut w = Vec::new();
//! #         self.to_doc().render(width, &mut w).unwrap();
//! #         String::from_utf8(w).unwrap()
//! #     }
//! # }
//! # fn main() {
//! let atom = SExp::Atom(5);
//! assert_eq!("5", atom.to_pretty(10));
//! let list = SExp::List(vec![SExp::Atom(1), SExp::Atom(2), SExp::Atom(3)]);
//! assert_eq!("(1 2 3)", list.to_pretty(10));
//! assert_eq!("\
//! (1
//!  2
//!  3)", list.to_pretty(5));
//! # }
//! ```
//!
//! ## Advanced usage
//!
//! There's a more efficient pattern that uses the [DocAllocator](trait.DocAllocator.html) trait, as
//! implemented by [BoxAllocator](struct.BoxAllocator.html), to allocate
//! [DocBuilder](struct.DocBuilder.html) instances.  See
//! [examples/trees.rs](https://github.com/freebroccolo/pretty.rs/blob/master/examples/trees.rs#L39)
//! for this approach.

#[cfg(feature = "termcolor")]
pub extern crate termcolor;
extern crate typed_arena;

use doc::Doc::{Annotated, Append, Group, Nest, Newline, Nil, Space, Text};
use std::borrow::Cow;
use std::fmt;
use std::ops::Deref;

mod doc;

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct BoxDoc<'a, A>(Box<doc::Doc<'a, A, BoxDoc<'a, A>>>);

impl<'a, A> fmt::Debug for BoxDoc<'a, A>
where
    A: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a, A> BoxDoc<'a, A> {
    fn new(doc: doc::Doc<'a, A, BoxDoc<'a, A>>) -> BoxDoc<'a, A> {
        BoxDoc(Box::new(doc))
    }
}

impl<'a, A> Deref for BoxDoc<'a, A> {
    type Target = doc::Doc<'a, A, BoxDoc<'a, A>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The `DocBuilder` type allows for convenient appending of documents even for arena allocated
/// documents by storing the arena inline.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct DocBuilder<'a, A, D: ?Sized>(pub &'a D, pub doc::Doc<'a, A, D::Doc>)
where
    D: DocAllocator<'a, A> + 'a;

impl<'a, A, D: DocAllocator<'a, A> + 'a> Clone for DocBuilder<'a, A, D>
where
    A: Clone,
    D::Doc: Clone,
{
    fn clone(&self) -> Self {
        DocBuilder(self.0, self.1.clone())
    }
}

impl<'a, A, D: ?Sized> Into<doc::Doc<'a, A, D::Doc>> for DocBuilder<'a, A, D>
where
    D: DocAllocator<'a, A>,
{
    fn into(self) -> doc::Doc<'a, A, D::Doc> {
        self.1
    }
}

/// The `DocAllocator` trait abstracts over a type which can allocate (pointers to) `Doc`.
pub trait DocAllocator<'a, A> {
    type Doc: Deref<Target = doc::Doc<'a, A, Self::Doc>>;

    fn alloc(&'a self, doc::Doc<'a, A, Self::Doc>) -> Self::Doc;

    /// Allocate an empty document.
    #[inline]
    fn nil(&'a self) -> DocBuilder<'a, A, Self> {
        DocBuilder(self, Nil)
    }

    /// Allocate a single newline.
    #[inline]
    fn newline(&'a self) -> DocBuilder<'a, A, Self> {
        DocBuilder(self, Newline)
    }

    /// Allocate a single space.
    #[inline]
    fn space(&'a self) -> DocBuilder<'a, A, Self> {
        DocBuilder(self, Space)
    }

    /// Allocate a document containing the text `t.to_string()`.
    ///
    /// The given text must not contain line breaks.
    #[inline]
    fn as_string<T: ToString>(&'a self, t: T) -> DocBuilder<'a, A, Self> {
        self.text(t.to_string())
    }

    /// Allocate a document containing the given text.
    ///
    /// The given text must not contain line breaks.
    #[inline]
    fn text<T: Into<Cow<'a, str>>>(&'a self, data: T) -> DocBuilder<'a, A, Self> {
        let text = data.into();
        debug_assert!(!text.contains(|c: char| c == '\n' || c == '\r'));
        DocBuilder(self, Text(text))
    }

    /// Allocate a document concatenating the given documents.
    #[inline]
    fn concat<I>(&'a self, docs: I) -> DocBuilder<'a, A, Self>
    where
        I: IntoIterator,
        I::Item: Into<doc::Doc<'a, A, Self::Doc>>,
    {
        docs.into_iter().fold(self.nil(), |a, b| a.append(b))
    }

    /// Allocate a document that intersperses the given separator `S` between the given documents
    /// `[A, B, C, ..., Z]`, yielding `[A, S, B, S, C, S, ..., S, Z]`.
    ///
    /// Compare [the `intersperse` method from the `itertools` crate](https://docs.rs/itertools/0.5.9/itertools/trait.Itertools.html#method.intersperse).
    #[inline]
    fn intersperse<I, S>(&'a self, docs: I, separator: S) -> DocBuilder<'a, A, Self>
    where
        I: IntoIterator,
        I::Item: Into<doc::Doc<'a, A, Self::Doc>>,
        S: Into<doc::Doc<'a, A, Self::Doc>> + Clone,
    {
        let mut result = self.nil();
        let mut iter = docs.into_iter();
        if let Some(first) = iter.next() {
            result = result.append(first);
        }
        for doc in iter {
            result = result.append(separator.clone());
            result = result.append(doc);
        }
        result
    }
}

impl<'a, 's, A, D: ?Sized> DocBuilder<'a, A, D>
where
    D: DocAllocator<'a, A>,
{
    /// Append the given document after this document.
    #[inline]
    pub fn append<B>(self, that: B) -> DocBuilder<'a, A, D>
    where
        B: Into<doc::Doc<'a, A, D::Doc>>,
    {
        let DocBuilder(allocator, this) = self;
        let that = that.into();
        let doc = match (this, that) {
            (Nil, that) => that,
            (this, Nil) => this,
            (this, that) => Append(allocator.alloc(this), allocator.alloc(that)),
        };
        DocBuilder(allocator, doc)
    }

    /// Mark this document as a group.
    ///
    /// Groups are layed out on a single line if possible.  Within a group, all basic documents with
    /// several possible layouts are assigned the same layout, that is, they are all layed out
    /// horizontally and combined into a one single line, or they are each layed out on their own
    /// line.
    #[inline]
    pub fn group(self) -> DocBuilder<'a, A, D> {
        let DocBuilder(allocator, this) = self;
        DocBuilder(allocator, Group(allocator.alloc(this)))
    }

    /// Increase the indentation level of this document.
    #[inline]
    pub fn nest(self, offset: usize) -> DocBuilder<'a, A, D> {
        if offset == 0 {
            return self;
        }
        let DocBuilder(allocator, this) = self;
        DocBuilder(allocator, Nest(offset, allocator.alloc(this)))
    }

    #[inline]
    pub fn annotate(self, ann: A) -> DocBuilder<'a, A, D> {
        let DocBuilder(allocator, this) = self;
        DocBuilder(allocator, Annotated(ann, allocator.alloc(this)))
    }
}

/// Newtype wrapper for `&doc::Doc`
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct RefDoc<'a, A: 'a>(&'a doc::Doc<'a, A, RefDoc<'a, A>>);

impl<'a, A> fmt::Debug for RefDoc<'a, A>
where
    A: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a, A> Deref for RefDoc<'a, A> {
    type Target = doc::Doc<'a, A, RefDoc<'a, A>>;

    fn deref(&self) -> &doc::Doc<'a, A, RefDoc<'a, A>> {
        &self.0
    }
}

/// An arena which can be used to allocate `Doc` values.
pub type Arena<'a, A> = typed_arena::Arena<doc::Doc<'a, A, RefDoc<'a, A>>>;

impl<'a, A, D> DocAllocator<'a, A> for &'a D
where
    D: ?Sized + DocAllocator<'a, A>,
{
    type Doc = D::Doc;

    #[inline]
    fn alloc(&'a self, doc: doc::Doc<'a, A, Self::Doc>) -> Self::Doc {
        (**self).alloc(doc)
    }
}

impl<'a, A> DocAllocator<'a, A> for Arena<'a, A> {
    type Doc = RefDoc<'a, A>;

    #[inline]
    fn alloc(&'a self, doc: doc::Doc<'a, A, Self::Doc>) -> Self::Doc {
        RefDoc(match doc {
            Space => &Doc::Space,
            Newline => &Doc::Newline,
            _ => Arena::alloc(self, doc),
        })
    }
}

pub struct BoxAllocator;

static BOX_ALLOCATOR: BoxAllocator = BoxAllocator;

impl<'a, A> DocAllocator<'a, A> for BoxAllocator {
    type Doc = BoxDoc<'a, A>;

    #[inline]
    fn alloc(&'a self, doc: doc::Doc<'a, A, Self::Doc>) -> Self::Doc {
        BoxDoc::new(doc)
    }
}

pub use doc::Doc;

impl<'a, A, B> Doc<'a, A, B> {
    /// An empty document.
    #[inline]
    pub fn nil() -> Doc<'a, A, B> {
        Nil
    }

    /// The text `t.to_string()`.
    ///
    /// The given text must not contain line breaks.
    #[inline]
    pub fn as_string<T: ToString>(t: T) -> Doc<'a, A, B> {
        Doc::text(t.to_string())
    }

    /// A single newline.
    #[inline]
    pub fn newline() -> Doc<'a, A, B> {
        Newline
    }

    /// The given text, which must not contain line breaks.
    #[inline]
    pub fn text<T: Into<Cow<'a, str>>>(data: T) -> Doc<'a, A, B> {
        let text = data.into();
        debug_assert!(!text.contains(|c: char| c == '\n' || c == '\r'));
        Text(text)
    }

    /// A space.
    #[inline]
    pub fn space() -> Doc<'a, A, B> {
        Space
    }
}

impl<'a, A> Doc<'a, A, BoxDoc<'a, A>> {
    /// Append the given document after this document.
    #[inline]
    pub fn append(self, that: Doc<'a, A, BoxDoc<'a, A>>) -> Doc<'a, A, BoxDoc<'a, A>> {
        DocBuilder(&BOX_ALLOCATOR, self).append(that).into()
    }

    /// A single document concatenating all the given documents.
    #[inline]
    pub fn concat<I>(docs: I) -> Doc<'a, A, BoxDoc<'a, A>>
    where
        I: IntoIterator<Item = Doc<'a, A, BoxDoc<'a, A>>>,
    {
        docs.into_iter().fold(Doc::nil(), |a, b| a.append(b))
    }

    /// A single document interspersing the given separator `S` between the given documents.  For
    /// example, if the documents are `[A, B, C, ..., Z]`, this yields `[A, S, B, S, C, S, ..., S, Z]`.
    ///
    /// Compare [the `intersperse` method from the `itertools` crate](https://docs.rs/itertools/0.5.9/itertools/trait.Itertools.html#method.intersperse).
    #[inline]
    pub fn intersperse<I, S>(docs: I, separator: S) -> Doc<'a, A, BoxDoc<'a, A>>
    where
        I: IntoIterator<Item = Doc<'a, A, BoxDoc<'a, A>>>,
        S: Into<Doc<'a, A, BoxDoc<'a, A>>> + Clone,
        A: Clone,
    {
        let separator = separator.into();
        let mut result = Doc::nil();
        let mut iter = docs.into_iter();
        if let Some(first) = iter.next() {
            result = result.append(first);
        }
        for doc in iter {
            result = result.append(separator.clone());
            result = result.append(doc);
        }
        result
    }

    /// Mark this document as a group.
    ///
    /// Groups are layed out on a single line if possible.  Within a group, all basic documents with
    /// several possible layouts are assigned the same layout, that is, they are all layed out
    /// horizontally and combined into a one single line, or they are each layed out on their own
    /// line.
    #[inline]
    pub fn group(self) -> Doc<'a, A, BoxDoc<'a, A>> {
        DocBuilder(&BOX_ALLOCATOR, self).group().into()
    }

    /// Increase the indentation level of this document.
    #[inline]
    pub fn nest(self, offset: usize) -> Doc<'a, A, BoxDoc<'a, A>> {
        DocBuilder(&BOX_ALLOCATOR, self).nest(offset).into()
    }

    #[inline]
    pub fn annotate(self, ann: A) -> Doc<'a, A, BoxDoc<'a, A>> {
        DocBuilder(&BOX_ALLOCATOR, self).annotate(ann).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test {
        ($size: expr, $actual: expr, $expected: expr) => {
            let mut s = String::new();
            $actual.render_fmt($size, &mut s).unwrap();
            assert_eq!(s, $expected);
        };
        ($actual: expr, $expected: expr) => {
            test!(70, $actual, $expected)
        }
    }

    #[test]
    fn box_doc_inference() {
        let doc = Doc::<(), _>::group(
            Doc::text("test")
                .append(Doc::space())
                .append(Doc::text("test")),
        );

        test!(doc, "test test");
    }

    #[test]
    fn forced_newline() {
        let doc = Doc::<(), _>::group(
            Doc::text("test")
                .append(Doc::newline())
                .append(Doc::text("test")),
        );

        test!(doc, "test\ntest");
    }

    #[test]
    fn space_do_not_reset_pos() {
        let doc = Doc::<(), _>::group(Doc::text("test").append(Doc::space()))
            .append(Doc::text("test"))
            .append(Doc::group(Doc::space()).append(Doc::text("test")));

        test!(9, doc, "test test\ntest");
    }

    // Tests that the `Doc::newline()` does not cause the rest of document to think that it fits on
    // a single line but instead breaks on the `Doc::space()` to fit with 6 columns
    #[test]
    fn newline_does_not_cause_next_line_to_be_to_long() {
        let doc = Doc::<(), _>::group(
            Doc::text("test").append(Doc::newline()).append(
                Doc::text("test")
                    .append(Doc::space())
                    .append(Doc::text("test")),
            ),
        );

        test!(6, doc, "test\ntest\ntest");
    }

    #[test]
    fn block() {
        let doc = Doc::<(), _>::group(
            Doc::text("{")
                .append(
                    Doc::space()
                        .append(Doc::text("test"))
                        .append(Doc::space())
                        .append(Doc::text("test"))
                        .nest(2),
                )
                .append(Doc::space())
                .append(Doc::text("}")),
        );

        test!(5, doc, "{\n  test\n  test\n}");
    }

    #[test]
    fn annotation_no_panic() {
        let doc = Doc::group(
            Doc::text("test")
                .annotate(())
                .append(Doc::newline())
                .annotate(())
                .append(Doc::text("test")),
        );

        test!(doc, "test\ntest");
    }
}
