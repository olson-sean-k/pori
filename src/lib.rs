use nom::error::{ErrorKind, ParseError};
use nom::{
    AsBytes, Compare, CompareResult, Err as ErrorMode, ExtendInto, IResult, Input, Needed, Offset,
    Parser,
};
use std::borrow::{Borrow, Cow, ToOwned};
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Range;

pub trait Location {
    fn location(&self) -> usize;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Located<I> {
    fragment: I,
    location: usize,
}

impl<I> Located<I> {
    pub fn into_fragment(self) -> I {
        self.fragment
    }

    fn slice_to_fragment(&self, fragment: I) -> Self
    where
        I: Offset,
    {
        let offset = self.fragment.offset(&fragment);
        Located {
            fragment,
            location: self.location + offset,
        }
    }
}

impl<I> AsBytes for Located<I>
where
    I: AsBytes,
{
    fn as_bytes(&self) -> &[u8] {
        self.fragment.as_bytes()
    }
}

impl<I> AsRef<I> for Located<I> {
    fn as_ref(&self) -> &I {
        &self.fragment
    }
}

impl<I> Borrow<I> for Located<&'_ I>
where
    I: ?Sized,
{
    fn borrow(&self) -> &I {
        self.fragment
    }
}

impl<I, U> Compare<U> for Located<I>
where
    I: Compare<U>,
    U: Into<Located<U>>,
{
    fn compare(&self, other: U) -> CompareResult {
        self.fragment.compare(other)
    }

    fn compare_no_case(&self, other: U) -> CompareResult {
        self.fragment.compare_no_case(other)
    }
}

impl<I> Display for Located<I>
where
    I: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.fragment, formatter)
    }
}

impl<I> ExtendInto for Located<I>
where
    I: ExtendInto,
{
    type Item = <I as ExtendInto>::Item;
    type Extender = <I as ExtendInto>::Extender;

    fn new_builder(&self) -> Self::Extender {
        self.fragment.new_builder()
    }

    fn extend_into(&self, extender: &mut Self::Extender) {
        self.fragment.extend_into(extender)
    }
}

impl<I> From<I> for Located<I> {
    fn from(fragment: I) -> Self {
        Located {
            fragment,
            location: 0,
        }
    }
}

impl<'i, I> From<Located<&'i I>> for Cow<'i, I>
where
    I: ToOwned,
{
    fn from(fragment: Located<&'i I>) -> Self {
        Cow::Borrowed(fragment.fragment)
    }
}

impl<I> Input for Located<I>
where
    I: AsBytes + Input + Offset,
{
    type Item = <I as Input>::Item;
    type Iter = <I as Input>::Iter;
    type IterIndices = <I as Input>::IterIndices;

    fn input_len(&self) -> usize {
        self.fragment.input_len()
    }

    fn take(&self, count: usize) -> Self {
        self.slice_to_fragment(self.fragment.take(count))
    }

    fn take_from(&self, index: usize) -> Self {
        self.slice_to_fragment(self.fragment.take_from(index))
    }

    fn take_split(&self, index: usize) -> (Self, Self) {
        (self.take_from(index), self.take(index))
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.fragment.position(predicate)
    }

    fn iter_elements(&self) -> Self::Iter {
        self.fragment.iter_elements()
    }

    fn iter_indices(&self) -> Self::IterIndices {
        self.fragment.iter_indices()
    }

    fn slice_index(&self, count: usize) -> Result<usize, Needed> {
        self.fragment.slice_index(count)
    }
}

impl<I> Location for Located<I> {
    fn location(&self) -> usize {
        self.location
    }
}

impl<I> Offset for Located<I>
where
    I: Offset,
{
    fn offset(&self, other: &Self) -> usize {
        other.location.saturating_sub(self.location)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Stateful<I, T> {
    fragment: I,
    pub state: T,
}

impl<I, T> Stateful<I, T> {
    pub fn new(fragment: I, state: T) -> Self {
        Stateful { fragment, state }
    }

    fn mapped<F>(&self, mut f: F) -> Self
    where
        T: Clone,
        F: FnMut(&I) -> I,
    {
        Stateful {
            fragment: f(&self.fragment),
            state: self.state.clone(),
        }
    }
}

impl<I, T> AsBytes for Stateful<I, T>
where
    I: AsBytes,
{
    fn as_bytes(&self) -> &[u8] {
        self.fragment.as_bytes()
    }
}

impl<I, T> AsRef<I> for Stateful<I, T> {
    fn as_ref(&self) -> &I {
        &self.fragment
    }
}

impl<I, T> Borrow<I> for Stateful<&'_ I, T>
where
    I: ?Sized,
{
    fn borrow(&self) -> &I {
        self.fragment
    }
}

impl<I, T> Borrow<I> for Stateful<Located<&'_ I>, T>
where
    I: ?Sized,
{
    fn borrow(&self) -> &I {
        self.fragment.borrow()
    }
}

impl<I, T, U> Compare<U> for Stateful<I, T>
where
    I: Compare<U>,
{
    fn compare(&self, other: U) -> CompareResult {
        self.fragment.compare(other)
    }

    fn compare_no_case(&self, other: U) -> CompareResult {
        self.fragment.compare_no_case(other)
    }
}

impl<I, T> Display for Stateful<I, T>
where
    I: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.fragment, formatter)
    }
}

impl<I, T> ExtendInto for Stateful<I, T>
where
    I: ExtendInto,
{
    type Item = I::Item;
    type Extender = I::Extender;

    fn new_builder(&self) -> Self::Extender {
        self.fragment.new_builder()
    }

    fn extend_into(&self, extender: &mut Self::Extender) {
        self.fragment.extend_into(extender)
    }
}

impl<I, T> Input for Stateful<I, T>
where
    I: Input,
    T: Clone,
{
    type Item = I::Item;
    type Iter = I::Iter;
    type IterIndices = I::IterIndices;

    fn input_len(&self) -> usize {
        self.fragment.input_len()
    }

    fn take(&self, count: usize) -> Self {
        self.mapped(move |data| data.take(count))
    }

    fn take_from(&self, index: usize) -> Self {
        self.mapped(move |data| data.take_from(index))
    }

    fn take_split(&self, index: usize) -> (Self, Self) {
        let (left, right) = self.fragment.take_split(index);
        (
            Stateful {
                fragment: left,
                state: self.state.clone(),
            },
            Stateful {
                fragment: right,
                state: self.state.clone(),
            },
        )
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.fragment.position(predicate)
    }

    fn iter_elements(&self) -> Self::Iter {
        self.fragment.iter_elements()
    }

    fn iter_indices(&self) -> Self::IterIndices {
        self.fragment.iter_indices()
    }

    fn slice_index(&self, count: usize) -> Result<usize, Needed> {
        self.fragment.slice_index(count)
    }
}

impl<I, T> Location for Stateful<I, T>
where
    I: Location,
{
    fn location(&self) -> usize {
        self.fragment.location()
    }
}

impl<I, T> Offset for Stateful<I, T>
where
    I: Offset,
{
    fn offset(&self, other: &Self) -> usize {
        self.fragment.offset(&other.fragment)
    }
}

pub fn bof<I, E>(input: I) -> IResult<I, I, E>
where
    I: Clone + Location,
    E: ParseError<I>,
{
    if input.location() == 0 {
        Ok((input.clone(), input))
    }
    else {
        Err(ErrorMode::Error(E::from_error_kind(input, ErrorKind::Eof)))
    }
}

pub fn span<I, F>(
    mut parser: F,
) -> impl Parser<I, Output = (Range<usize>, F::Output), Error = F::Error>
where
    I: Clone + Location,
    F: Parser<I>,
{
    move |input: I| {
        let start = input.location();
        parser.parse(input).map(move |(remaining, output)| {
            let end = remaining.location();
            (remaining, (start..end, output))
        })
    }
}
