use std::borrow::Cow;
use std::cmp;
use std::fmt;
use std::io;
use std::ops::Deref;

pub use self::Doc::{Nil, Append, Space, Group, Nest, Block, Newline, Text, Union};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Mode {
    Break,
    Flat,
}

/// The concrete document type. This type is not meant to be used directly. Instead use the static
/// functions on `Doc` or the methods on an `DocAllocator`.
///
/// The `B` parameter is used to abstract over pointers to `Doc`. See `RefDoc` and `BoxDoc` for how
/// it is used
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Doc<'a, B> {
    Nil,
    Append(B, B),
    Group(B),
    Nest(usize, B),
    Block(usize, B),
    Space,
    Newline,
    Text(Cow<'a, str>),
    Union(B, B),
}

impl<'a, B, S> From<S> for Doc<'a, B>
where
    S: Into<Cow<'a, str>>,
{
    fn from(s: S) -> Doc<'a, B> {
        Doc::Text(s.into())
    }
}

trait Render {
    type Error;
    fn write_str(&mut self, s: &str) -> Result<usize, Self::Error>;
    fn write_str_all(&mut self, s: &str) -> Result<(), Self::Error>;
}

struct IoWrite<W>(W);
impl<W> Render for IoWrite<W>
where
    W: io::Write,
{
    type Error = io::Error;

    fn write_str(&mut self, s: &str) -> io::Result<usize> {
        self.0.write(s.as_bytes())
    }
    fn write_str_all(&mut self, s: &str) -> io::Result<()> {
        self.0.write_all(s.as_bytes())
    }
}

struct FmtWrite<W>(W);
impl<W> Render for FmtWrite<W>
where
    W: fmt::Write,
{
    type Error = fmt::Error;

    fn write_str(&mut self, s: &str) -> Result<usize, fmt::Error> {
        self.write_str_all(s).map(|_| s.len())
    }
    fn write_str_all(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s)
    }
}

pub struct Pretty<'a, D>
where
    D: 'a,
{
    doc: &'a Doc<'a, D>,
    width: usize,
}

impl<'a, D> fmt::Display for Pretty<'a, D>
where
    D: Deref<Target = Doc<'a, D>>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.doc.render_fmt(self.width, f)
    }
}

impl<'a, B> Doc<'a, B> {
    /// Writes a rendered document to a `std::io::Write` object.
    #[inline]
    pub fn render<'b, W>(&'b self, width: usize, out: &mut W) -> io::Result<()>
    where
        B: Deref<Target = Doc<'b, B>>,
        W: ?Sized + io::Write,
    {
        best(self, width, &mut IoWrite(out))
    }

    /// Writes a rendered document to a `std::fmt::Write` object.
    #[inline]
    pub fn render_fmt<'b, W>(&'b self, width: usize, out: &mut W) -> fmt::Result
    where
        B: Deref<Target = Doc<'b, B>>,
        W: ?Sized + fmt::Write,
    {
        best(self, width, &mut FmtWrite(out))
    }

    /// Returns a value which implements `std::fmt::Display`
    ///
    /// ```
    /// use pretty::Doc;
    /// let doc = Doc::group(
    ///     Doc::text("hello").append(Doc::space()).append(Doc::text("world"))
    /// );
    /// assert_eq!(format!("{}", doc.pretty(80)), "hello world");
    /// ```
    #[inline]
    pub fn pretty<'b>(&'b self, width: usize) -> Pretty<'b, B>
    where
        B: Deref<Target = Doc<'b, B>>,
    {
        Pretty {
            doc: self,
            width: width,
        }
    }
}

type Cmd<'a, B> = (usize, Mode, &'a Doc<'a, B>);

fn write_newline<W>(ind: usize, out: &mut W) -> Result<(), W::Error>
where
    W: ?Sized + Render,
{
    try!(out.write_str_all("\n"));
    write_spaces(ind, out)
}

fn write_spaces<W>(spaces: usize, out: &mut W) -> Result<(), W::Error>
where
    W: ?Sized + Render,
{
    macro_rules! make_spaces {
        () => {
            ""
        };
        ($s: tt $($t: tt)*) => {
            concat!("          ", make_spaces!($($t)*))
        };
    }
    const SPACES: &str = make_spaces!(,,,,,,,,,,);
    let mut inserted = 0;
    while inserted < spaces {
        let insert = cmp::min(SPACES.len(), spaces - inserted);
        inserted += try!(out.write_str(&SPACES[..insert]));
    }
    Ok(())
}

#[inline]
fn fitting<'a, B>(
    next: Cmd<'a, B>,
    bcmds: &Vec<Cmd<'a, B>>,
    fcmds: &mut Vec<Cmd<'a, B>>,
    bidx: usize,
    mut rem: isize,
) -> bool
where
    B: Deref<Target = Doc<'a, B>>,
{
    let fcmds_start_len = fcmds.len();
    let result = fitting_(next, bcmds, fcmds, bidx, rem);
    fcmds.truncate(fcmds_start_len);
    result
}

#[inline]
fn fitting_<'a, B>(next: Cmd<'a, B>,
                   bcmds: &Vec<Cmd<'a, B>>,
                   fcmds: &mut Vec<Cmd<'a, B>>,
                   mut bidx: usize,
                   mut rem: isize)
                   -> bool
    where B: Deref<Target = Doc<'a, B>>
{
    let fcmds_start_len = fcmds.len();
    fcmds.push(next);
    while rem >= 0 {
        if fcmds.len() <= fcmds_start_len {
            if bidx == 0 {
                // All commands have been processed
                return true;
            } else {
                fcmds.push(bcmds[bidx - 1]);
                bidx -= 1;
            }
        } else {
            let (ind, mode, doc) = fcmds.pop().unwrap();
            match doc {
                &Nil => {}
                &Append(ref ldoc, ref rdoc) => {
                    fcmds.push((ind, mode, rdoc));
                    // Since appended documents often appear in sequence on the left side we
                    // gain a slight performance increase by batching these pushes (avoiding
                    // to push and directly pop `Append` documents)
                    let mut doc = ldoc;
                    while let Append(ref l, ref r) = **doc {
                        fcmds.push((ind, mode, r));
                        doc = l;
                    }
                    fcmds.push((ind, mode, doc));
                }
                &Group(ref doc) => {
                    fcmds.push((ind, mode, doc));
                }
                &Nest(off, ref doc) |
                &Block(off, ref doc) => {
                    fcmds.push((ind + off, mode, doc));
                }
                &Space => {
                    match mode {
                        Mode::Flat => {
                            rem -= 1;
                        }
                        Mode::Break => {
                            return true;
                        }
                    }
                }
                &Newline => return true,
                &Text(ref str) => {
                    rem -= str.len() as isize;
                }
                &Union(ref x, ref y) => {
                    if fitting((ind, Mode::Flat, x), bcmds, fcmds, bidx, rem) {
                        return true;
                    } else {
                        fcmds.push((ind, mode, y));
                    }
                }
            }
        }
    }
    false
}

#[inline]
fn best<'a, W, B>(doc: &'a Doc<'a, B>, width: usize, out: &mut W) -> Result<(), W::Error>
where
    B: Deref<Target = Doc<'a, B>>,
    W: ?Sized + Render,
{
    let mut pos = 0usize;
    let mut bcmds = vec![(0usize, Mode::Break, doc)];
    let mut fcmds = vec![];
    let mut current_indentation = 0usize;
    while let Some((ind, mode, doc)) = bcmds.pop() {
        match doc {
            &Nil => {}
            &Append(ref ldoc, ref rdoc) => {
                bcmds.push((ind, mode, rdoc));
                let mut doc = ldoc;
                while let Append(ref l, ref r) = **doc {
                    bcmds.push((ind, mode, r));
                    doc = l;
                }
                bcmds.push((ind, mode, doc));
            }
            &Group(ref doc) => {
                match mode {
                    Mode::Flat => {
                        bcmds.push((ind, Mode::Flat, doc));
                    }
                    Mode::Break => {
                        let next = (ind, Mode::Flat, &**doc);
                        let rem = width as isize - pos as isize;
                        fcmds.clear();
                        if fitting(next, &bcmds, &mut fcmds, bcmds.len(), rem) {
                            bcmds.push(next);
                        } else {
                            bcmds.push((ind, Mode::Break, doc));
                        }
                    }
                }
            }
            &Nest(off, ref doc) => {
                bcmds.push((ind + off, mode, doc));
            }
            &Block(off, ref doc) => {
                bcmds.push((if ind == current_indentation {
                                ind + off
                            } else {
                                ind
                            },
                            mode,
                            doc));
            }
            &Space => {
                match mode {
                    Mode::Flat => {
                        try!(write_spaces(1, out));
                    }
                    Mode::Break => {
                        try!(write_newline(ind, out));
                        current_indentation = ind;
                    }
                }
                pos = ind;
            }
            &Newline => {
                try!(write_newline(ind, out));
                current_indentation = ind;
                pos = ind;
            }
            &Text(ref s) => {
                try!(out.write_str_all(s));
                pos += s.len();
            }
            &Union(ref x, ref y) => {
                fcmds.clear();
                let rem = width as isize - pos as isize;
                let doc = if fitting((ind, Mode::Flat, x), &bcmds, &mut fcmds, bcmds.len(), rem) {
                    x
                } else {
                    y
                };
                bcmds.push((ind, mode, doc));
            }
        }
    }
    Ok(())
}
