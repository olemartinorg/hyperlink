use std::io::{self, BufReader, Read, BufRead};
use std::mem;

/// A BufReader whose underlying buffer can be reused between various readers.
pub struct ReusableBufRead<R: Read> {
    reader: BufReader<Inner<R>>
}

impl<R: Read> ReusableBufRead<R> {
    pub fn new() -> Self {
        println!("bufread!");
        ReusableBufRead {
            reader: BufReader::with_capacity(5_000_000, Inner(mem::MaybeUninit::uninit()))
        }
    }

    pub fn lease<'a>(&'a mut self, read: R) -> Lease<'a, R> {
        unsafe {
            self.reader.get_mut().0.as_mut_ptr().write(read);
        }
        Lease(&mut self.reader)
    }
}


struct Inner<R>(mem::MaybeUninit<R>);

impl<R: Read> Read for Inner<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe { &mut *self.0.as_mut_ptr() }.read(buf)
    }
}

pub struct Lease<'a, R>(&'a mut BufReader<Inner<R>>);

impl<'a, R: Read> Read for Lease<'a, R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl<'a, R: Read> BufRead for Lease<'a, R> {
    #[inline]
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.0.fill_buf()
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        self.0.consume(amt)
    }
}

impl<'a, R> Drop for Lease<'a, R> {
    fn drop(&mut self) {
        unsafe { self.0.get_mut().0.as_mut_ptr().drop_in_place() }
    }
}
