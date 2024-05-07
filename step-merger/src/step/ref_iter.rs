use std::{fs::File, io::Read, path::Path};

use crate::error::{Error, Result};
use circular::Buffer;
use winnow::ascii::digit1;
use winnow::error::{ContextError, ErrMode, Needed, StrContext};
use winnow::stream::Stream;
use winnow::token::{take, take_till, take_until};
use winnow::{prelude::*, Partial};

pub(crate) type Input<'i> = Partial<&'i str>;

static BUFFER_SIZE_START: usize = 1024;
static BUFFER_GROWTH_FACTOR: usize = 2;
static BUFFER_GROWTH_MIN: usize = 100;

pub struct RefIter {
    file: File,
    buffer: Buffer,
}

impl RefIter {
    pub fn new<T: AsRef<Path>>(path: T) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::open(path)
            .map_err(|e| Error::IO(format!("Failed to open file {path_str}: {e}")))?;
        let buffer = Buffer::with_capacity(BUFFER_SIZE_START);

        let mut r = RefIter { file, buffer };

        r.fill_buffer()?;
        r.skip_header()?;

        Ok(r)
    }

    fn parse_header(input: &mut Input) -> PResult<()> {
        let _actual = "ISO-10303-21"
            .context(StrContext::Label("Magic Value"))
            .parse_next(input)?;
        let _header = (take_until(0.., "HEADER"), take(6u8))
            .context(StrContext::Label("Header"))
            .parse_next(input)?;
        let _tag = (take_until(0.., "ENDSEC"), take(6u8))
            .context(StrContext::Label("Header End"))
            .parse_next(input)?;
        Ok(())
    }

    fn find_sec(input: &mut Input) -> PResult<()> {
        let _data = (take_until(0.., "DATA"), take(4u8))
            .context(StrContext::Label("DATA"))
            .parse_next(input)?;
        let _skip_sem = (take_till(0.., ';'), take(1u8)).parse_next(input)?;
        Ok(())
    }

    fn parse_line(input: &mut Input) -> PResult<Vec<usize>> {
        let mut refs = Vec::new();
        let ref_start = (take_till(0.., '#'), take(1u8), digit1)
            .context(StrContext::Label("Find Ref"))
            .parse_next(input)?;

        refs.push(ref_start.2.parse::<usize>().unwrap());

        loop {
            let ref_or_end = (take_till(0.., ('#', ';')), take(1u8))
                .context(StrContext::Label("Find Ref or End"))
                .parse_next(input)?;

            if ref_or_end.1 == ";" {
                break;
            } else {
                // Need to continue on backtrack as there are things like #Final Part
                let d = match digit1
                    .context(StrContext::Label("Ref Digits"))
                    .parse_next(input)
                {
                    Ok(d) => d,
                    Err(ErrMode::Backtrack(_)) => continue,
                    Err(e) => return Err(e),
                };
                refs.push(d.parse::<usize>().unwrap());
            }
        }

        Ok(refs)
    }

    fn run_parser<T, F>(&mut self, f: F) -> Result<T>
    where
        F: Fn(&mut Input) -> PResult<T>,
    {
        loop {
            let mut input = Input::new(
                std::str::from_utf8(self.buffer.data())
                    .map_err(|e| Error::IO(format!("Failed to parse UTF8: {e}")))?,
            );
            match f(&mut input) {
                Ok(r) => {
                    self.buffer
                        .consume(self.buffer.available_data() - input.eof_offset());
                    return Ok(r);
                }
                Err(e) => self.handle_errors(e)?,
            }
            self.fill_buffer()?;
        }
    }

    pub fn next_line(&mut self) -> Result<Vec<usize>> {
        self.run_parser(Self::parse_line)
    }

    fn skip_header(&mut self) -> Result<()> {
        self.run_parser(Self::parse_header)?;
        self.run_parser(Self::find_sec)
    }

    fn handle_errors(&mut self, error: ErrMode<ContextError>) -> Result<()> {
        match error {
            ErrMode::Incomplete(i) => {
                self.update_buffer(i);
                Ok(())
            }
            ErrMode::Backtrack(e) => Err(Error::ParsingError(e.to_string())),
            ErrMode::Cut(e) => Err(Error::ParsingError(e.to_string())),
        }
    }

    fn update_buffer(&mut self, needed: Needed) {
        match needed {
            Needed::Unknown => {
                let new_capacity = BUFFER_GROWTH_FACTOR * self.buffer.capacity();
                self.buffer.grow(new_capacity);
            }
            Needed::Size(size) => {
                let head_room = size.get().max(BUFFER_GROWTH_MIN);
                let new_capacity = self.buffer.available_space() + head_room;
                self.buffer.grow(new_capacity);
                if self.buffer.available_space() < head_room {
                    self.buffer.shift();
                }
            }
        }
    }

    fn fill_buffer(&mut self) -> Result<()> {
        let read = self
            .file
            .read(self.buffer.space())
            .map_err(|e| Error::IO(format!("Failed to read from file: {e}")))?;

        if read == 0 {
            return Err(Error::IO("End of file".to_string()));
        } else {
            self.buffer.fill(read);
        }

        Ok(())
    }
}

impl Iterator for RefIter {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(i) = self.next_line() {
            Some(i)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use winnow::Parser;

    use super::Input;

    use super::RefIter;

    #[test]
    fn test_header_skip() {
        let s = "ISO-10303-21 Test 42; asdasdasd HEADER; sad sa ENDSEC 424242";
        let input = Input::new(s);
        assert_eq!(
            Ok((Input::new(" 424242"), ())),
            RefIter::parse_header.parse_peek(input)
        );
    }

    #[test]
    fn test_line_fetching_wiki() {
        let s = RefIter::new("../test_data/wiki.stp");
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(11, s.into_iter().count());
    }

    #[test]
    fn test_line_fetching_2() {
        let s = RefIter::new("../test_data/2.stp");
        assert!(s.is_ok());

        let s = s.unwrap();
        assert_eq!(9552, s.into_iter().count())
    }
}
