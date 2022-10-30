use std::io;
use std::io::BufRead;

/**
 * trait lets any BufRead impl be processed with a "for line in ..." style
 */

fn full_lines(mut input: impl BufRead) -> impl Iterator<Item = io::Result<String>> {
    std::iter::from_fn(move || {
        let mut vec = String::new();
        match input.read_line(&mut vec) {
            Ok(0) => None,
            Ok(_) => Some(Ok(vec)),
            Err(e) => Some(Err(e)),
        }
    })
}

pub trait FullLines: BufRead + Sized {
    fn full_lines<'a>(self) -> Box<dyn Iterator<Item = io::Result<String>> + 'a>
    where
        Self: 'a,
    {
        Box::new(full_lines(self))
    }
}

impl<T: BufRead> FullLines for T {}
