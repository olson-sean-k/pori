use nom::error::{Error as NomError, ErrorKind, ParseError};
use nom::{
    AsBytes, Compare, CompareResult, Err as ErrorMode, ExtendInto, IResult, Input, Needed, Offset,
    Parser,
};
use std::borrow::{Cow, ToOwned};
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

pub trait Location {
    fn location(&self) -> usize;
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct Located<'i, I>
where
    I: ?Sized,
{
    data: &'i I,
    location: usize,
}

impl<'i, I> Located<'i, I>
where
    I: ?Sized,
{
    pub fn into_data(self) -> &'i I {
        self.data
    }

    fn to_data_offset(self, data: &'i I) -> Self
    where
        I: Offset,
    {
        let offset = self.data.offset(data);
        Located {
            data,
            location: self.location + offset,
        }
    }
}

impl<'i, I> AsBytes for Located<'i, I>
where
    I: ?Sized,
    &'i I: AsBytes,
{
    fn as_bytes(&self) -> &[u8] {
        self.data.as_bytes()
    }
}

impl<I> AsRef<I> for Located<'_, I>
where
    I: ?Sized,
{
    fn as_ref(&self) -> &I {
        self.data
    }
}

impl<I> Clone for Located<'_, I>
where
    I: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'i, 'u, I, U> Compare<&'u U> for Located<'i, I>
where
    I: ?Sized,
    U: ?Sized,
    &'i I: Compare<&'u U>,
    &'u U: Into<Located<'u, U>>,
{
    fn compare(&self, other: &'u U) -> CompareResult {
        self.data.compare(other.into().data)
    }

    fn compare_no_case(&self, other: &'u U) -> CompareResult {
        self.data.compare_no_case(other.into().data)
    }
}

impl<I> Copy for Located<'_, I> where I: ?Sized {}

impl<I> Deref for Located<'_, I>
where
    I: ?Sized,
{
    type Target = I;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<I> Display for Located<'_, I>
where
    I: Display + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.data, f)
    }
}

impl<'i, I> ExtendInto for Located<'i, I>
where
    I: ?Sized,
    &'i I: ExtendInto,
{
    type Item = <&'i I as ExtendInto>::Item;
    type Extender = <&'i I as ExtendInto>::Extender;

    fn new_builder(&self) -> Self::Extender {
        self.data.new_builder()
    }

    fn extend_into(&self, extender: &mut Self::Extender) {
        self.data.extend_into(extender)
    }
}

impl<'i, I> From<Located<'i, I>> for Cow<'i, I>
where
    I: ?Sized + ToOwned,
{
    fn from(fragment: Located<'i, I>) -> Self {
        Cow::Borrowed(fragment.data)
    }
}

impl<'i, I> From<&'i I> for Located<'i, I>
where
    I: ?Sized,
{
    fn from(data: &'i I) -> Self {
        Located { data, location: 0 }
    }
}

impl<'i, I> Input for Located<'i, I>
where
    I: Offset + ?Sized,
    &'i I: Input,
{
    type Item = <&'i I as Input>::Item;
    type Iter = <&'i I as Input>::Iter;
    type IterIndices = <&'i I as Input>::IterIndices;

    fn input_len(&self) -> usize {
        self.data.input_len()
    }

    fn take(&self, count: usize) -> Self {
        let taken = self.data.take(count);
        self.to_data_offset(taken)
    }

    fn take_from(&self, index: usize) -> Self {
        let taken = self.data.take(index);
        self.to_data_offset(taken)
    }

    fn take_split(&self, index: usize) -> (Self, Self) {
        let (left, right) = self.data.take_split(index);
        (self.to_data_offset(left), self.to_data_offset(right))
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.data.position(predicate)
    }

    fn iter_elements(&self) -> Self::Iter {
        self.data.iter_elements()
    }

    fn iter_indices(&self) -> Self::IterIndices {
        self.data.iter_indices()
    }

    fn slice_index(&self, count: usize) -> Result<usize, Needed> {
        self.data.slice_index(count)
    }

    fn split_at_position<P, E>(&self, predicate: P) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
        E: ParseError<Self>,
    {
        match self.data.position(predicate) {
            Some(n) => Ok(self.take_split(n)),
            None => Err(ErrorMode::Incomplete(Needed::new(1))),
        }
    }

    fn split_at_position1<P, E>(&self, predicate: P, kind: ErrorKind) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
        E: ParseError<Self>,
    {
        match self.data.position(predicate) {
            Some(0) => Err(ErrorMode::Error(E::from_error_kind(*self, kind))),
            Some(n) => Ok(self.take_split(n)),
            None => Err(ErrorMode::Incomplete(Needed::new(1))),
        }
    }

    fn split_at_position_complete<P, E>(&self, predicate: P) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
        E: ParseError<Self>,
    {
        match self.split_at_position(predicate) {
            Err(ErrorMode::Incomplete(_)) => Ok(self.take_split(self.input_len())),
            result => result,
        }
    }

    fn split_at_position1_complete<P, E>(
        &self,
        predicate: P,
        kind: ErrorKind,
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
        E: ParseError<Self>,
    {
        match self.data.position(predicate) {
            Some(0) => Err(ErrorMode::Error(E::from_error_kind(*self, kind))),
            Some(n) => Ok(self.take_split(n)),
            None => {
                if self.data.input_len() == 0 {
                    Err(ErrorMode::Error(E::from_error_kind(*self, kind)))
                }
                else {
                    Ok(self.take_split(self.input_len()))
                }
            }
        }
    }

    // TODO: Write tests that confirm that the default implementations of these  modal
    //       `split_at_position` functions function properly.

    //fn split_at_position_mode<M, P, E>(&self, predicate: P) -> PResult<M, Self, Self, E>
    //where
    //    M: OutputMode,
    //    P: Fn(Self::Item) -> bool,
    //    E: ParseError<Self>,
    //{
    //}

    //fn split_at_position_mode1<M, P, E>(
    //    &self,
    //    predicate: P,
    //    kind: ErrorKind,
    //) -> PResult<M, Self, Self, E>
    //where
    //    M: OutputMode,
    //    P: Fn(Self::Item) -> bool,
    //    E: ParseError<Self>,
    //{
    //}
}

impl<I> Location for Located<'_, I>
where
    I: ?Sized,
{
    fn location(&self) -> usize {
        self.location
    }
}

impl<I> Offset for Located<'_, I>
where
    I: ?Sized,
{
    fn offset(&self, other: &Self) -> usize {
        other.location.saturating_sub(self.location)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Stateful<I, T> {
    data: I,
    pub state: T,
}

impl<I, T> Stateful<I, T> {
    pub fn new(data: I, state: T) -> Self {
        Stateful { data, state }
    }

    fn clone_map<F>(&self, mut f: F) -> Self
    where
        T: Clone,
        F: FnMut(&I) -> I,
    {
        Stateful {
            data: f(&self.data),
            state: self.state.clone(),
        }
    }

    fn clone_map_iresult<E, F>(&self, f: F) -> IResult<Self, Self, E>
    where
        E: ParseError<Self>,
        T: Clone,
        F: FnOnce(&I) -> IResult<I, I>,
    {
        let map_error = |error: NomError<I>| {
            E::from_error_kind(
                Stateful {
                    data: error.input,
                    state: self.state.clone(),
                },
                error.code,
            )
        };
        f(&self.data)
            .map(|(remaining, output)| {
                (
                    Stateful {
                        data: remaining,
                        state: self.state.clone(),
                    },
                    Stateful {
                        data: output,
                        state: self.state.clone(),
                    },
                )
            })
            .map_err(|error| match error {
                ErrorMode::Error(error) => ErrorMode::Error(map_error(error)),
                ErrorMode::Failure(error) => ErrorMode::Failure(map_error(error)),
                ErrorMode::Incomplete(needed) => ErrorMode::Incomplete(needed),
            })
    }
}

impl<I, T> AsBytes for Stateful<I, T>
where
    I: AsBytes,
{
    fn as_bytes(&self) -> &[u8] {
        self.data.as_bytes()
    }
}

impl<I, T> AsRef<I> for Stateful<I, T> {
    fn as_ref(&self) -> &I {
        &self.data
    }
}

impl<I, T, U> Compare<U> for Stateful<I, T>
where
    I: Compare<U>,
{
    fn compare(&self, other: U) -> CompareResult {
        self.data.compare(other)
    }

    fn compare_no_case(&self, other: U) -> CompareResult {
        self.data.compare_no_case(other)
    }
}

impl<I, T> Deref for Stateful<I, T> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<I, T> Display for Stateful<I, T>
where
    I: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.data, f)
    }
}

impl<I, T> ExtendInto for Stateful<I, T>
where
    I: ExtendInto,
{
    type Item = I::Item;
    type Extender = I::Extender;

    fn new_builder(&self) -> Self::Extender {
        self.data.new_builder()
    }

    fn extend_into(&self, extender: &mut Self::Extender) {
        self.data.extend_into(extender)
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
        self.data.input_len()
    }

    fn take(&self, count: usize) -> Self {
        self.clone_map(move |data| data.take(count))
    }

    fn take_from(&self, index: usize) -> Self {
        self.clone_map(move |data| data.take_from(index))
    }

    fn take_split(&self, index: usize) -> (Self, Self) {
        let (left, right) = self.data.take_split(index);
        (
            Stateful {
                data: left,
                state: self.state.clone(),
            },
            Stateful {
                data: right,
                state: self.state.clone(),
            },
        )
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.data.position(predicate)
    }

    fn iter_elements(&self) -> Self::Iter {
        self.data.iter_elements()
    }

    fn iter_indices(&self) -> Self::IterIndices {
        self.data.iter_indices()
    }

    fn slice_index(&self, count: usize) -> Result<usize, Needed> {
        self.data.slice_index(count)
    }

    fn split_at_position<P, E>(&self, predicate: P) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
        E: ParseError<Self>,
    {
        self.clone_map_iresult(move |data| data.split_at_position(predicate))
    }

    fn split_at_position1<P, E>(&self, predicate: P, kind: ErrorKind) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
        E: ParseError<Self>,
    {
        self.clone_map_iresult(move |data| data.split_at_position1(predicate, kind))
    }

    fn split_at_position_complete<P, E>(&self, predicate: P) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
        E: ParseError<Self>,
    {
        self.clone_map_iresult(move |data| data.split_at_position_complete(predicate))
    }

    fn split_at_position1_complete<P, E>(
        &self,
        predicate: P,
        kind: ErrorKind,
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
        E: ParseError<Self>,
    {
        self.clone_map_iresult(move |data| data.split_at_position1_complete(predicate, kind))
    }

    // TODO: Write tests that confirm that the default implementations of these  modal
    //       `split_at_position` functions function properly.

    //fn split_at_position_mode<M, P, E>(&self, predicate: P) -> PResult<M, Self, Self, E>
    //where
    //    M: OutputMode,
    //    P: Fn(Self::Item) -> bool,
    //    E: ParseError<Self>,
    //{
    //}

    //fn split_at_position_mode1<M, P, E>(
    //    &self,
    //    predicate: P,
    //    kind: ErrorKind,
    //) -> PResult<M, Self, Self, E>
    //where
    //    M: OutputMode,
    //    P: Fn(Self::Item) -> bool,
    //    E: ParseError<Self>,
    //{
    //}
}

impl<I, T> Location for Stateful<I, T>
where
    I: Location,
{
    fn location(&self) -> usize {
        self.data.location()
    }
}

impl<I, T> Offset for Stateful<I, T>
where
    I: Offset,
{
    fn offset(&self, other: &Self) -> usize {
        self.data.offset(&other.data)
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
) -> impl Parser<I, Output = ((usize, usize), F::Output), Error = F::Error>
where
    I: Clone + Location,
    F: Parser<I>,
{
    move |input: I| {
        let start = input.location();
        parser.parse(input).map(move |(remaining, output)| {
            let end = remaining.location();
            (remaining, ((start, end.saturating_sub(start)), output))
        })
    }
}
