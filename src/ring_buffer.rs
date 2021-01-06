use std::ops::Range;
use std::fmt::Debug;

#[derive(Debug)]
pub struct RingBuffer<T> {
    data: Vec<T>,
    size: usize,
    start: usize,
    len: usize
}
impl<T: Copy + Default> RingBuffer<T> {
    pub fn new(size: usize, filled: bool) -> Self {
        RingBuffer {
            data: vec![Default::default(); size],
            size,
            start: 0,
            len: if filled { size } else { 0 }
        }
    }
}
impl<T> RingBuffer<T> {
    pub fn len(&self) -> usize {
        self.len
    }
    fn end(&self) -> usize {
        let end = self.start + self.len;
        if end >= self.size {
            return end - self.size;
        }
        return end;
    }
    // pub fn clear_fill(&mut self) {
    //     self.start = 0;
    //     self.len = self.size;
    // }
    // pub fn clear(&mut self) {
    //     self.start = 0;
    //     self.len = 0;
    // }
    pub fn remove(&mut self, count: usize) {
        assert!(count <= self.len);
        self.start += count;
        self.len -= count;
        if self.start > self.size {
            self.start -= self.size;
        }
    }
    fn split_range(&self, start: usize, len: usize) -> (usize, Range<usize>) {
        let end = start + len;
        if end <= self.size {
            (0, start..end)
        } else {
            let end = end - self.size;
            (end, start - end..self.size - end)
        }
    }
    fn split_range_relative(&self, start: isize) -> (usize, Range<usize>) {
        assert!(start < self.len as isize);
        assert!(-start <= self.len as isize);
        let count = if start >= 0 { self.len - start as usize} else { -start as usize };

        let mut start = self.start + self.len - count;
        if start > self.size {
            start -= self.size;
        }

        self.split_range(start, count)
    }
    fn slices_append(&mut self, len: usize) -> (&mut [T], &mut [T]) {
        // dbg!(len, self.size, self.len);
        assert!(len <= self.size - self.len, "not enough space");

        let (split, range) = self.split_range(self.end(), len);
        let (tail, head) = self.data.split_at_mut(split);
        let head = &mut head[range];

        self.len += len;

        (head, tail)
    }
    fn slices_remove(&mut self, len: usize) -> (&[T], &[T]) {
        // dbg!(len, self.len);
        assert!(len <= self.len, "removing too many");

        let (split, range) = self.split_range(self.start, len);
        let (tail, head) = self.data.split_at(split);
        let head = &head[range];

        self.start += len;
        self.len -= len;
        if self.start > self.size {
            self.start -= self.size;
        }

        (head, tail)
    }
    fn slices_replace(&mut self, len: usize) -> (&mut [T], &mut [T]) {
        assert_eq!(self.len, self.size, "needs to be full");

        let (split, range) = self.split_range(self.start, len);
        let (tail, head) = self.data.split_at_mut(split);
        let head = &mut head[range];

        self.start += len;
        if self.start > self.size {
            self.start -= self.size;
        }

        (head, tail)
    }
    pub fn iter_append(&mut self, len: usize) -> impl Iterator<Item = &mut T> {
        let (head, tail) = self.slices_append(len);
        head.iter_mut().chain(tail.iter_mut())
    }
    pub fn iter_remove(&mut self, len: usize) -> impl Iterator<Item = &T> {
        let (head, tail) = self.slices_remove(len);
        head.iter().chain(tail.iter())
    }
    pub fn iter_replace(&mut self, len: usize) -> impl Iterator<Item = &mut T> {
        let (head, tail) = self.slices_replace(len);
        head.iter_mut().chain(tail.iter_mut())
    }
    pub fn iter(&self, start: isize) -> impl Iterator<Item = &T> {
        let (split, range) = self.split_range_relative(start);
        let (tail, head) = self.data.split_at(split);
        let head = &head[range];

        head.iter().chain(tail.iter())
    }
    pub fn iter_mut(&mut self, start: isize) -> impl Iterator<Item = &mut T> {
        let (split, range) = self.split_range_relative(start);
        let (tail, head) = self.data.split_at_mut(split);
        let head = &mut head[range];

        head.iter_mut().chain(tail.iter_mut())
    }
}

impl<T: Copy> RingBuffer<T> {
    pub fn copy_append(&mut self, src: &[T]) {
        let (head, tail) = self.slices_append(src.len());

        let split = head.len();
        head.copy_from_slice(&src[..split]);
        tail.copy_from_slice(&src[split..]);
    }
    pub fn copy_remove(&mut self, dst: &mut [T]) {
        let (head, tail) = self.slices_remove(dst.len());

        let split = head.len();
        dst[..split].copy_from_slice(&head);
        dst[split..].copy_from_slice(&tail);
    }
}

impl<T: Copy + Default> RingBuffer<T> {
    pub fn copy_replace(&mut self, src: Option<&[T]>, dst: Option<&mut [T]>) {
        let len = match (&src, &dst) {
            (Some(src), Some(dst)) => {
                assert_eq!(src.len(), dst.len());
                src.len()
            },
            (Some(src), None) => src.len(),
            (None, Some(dst)) => dst.len(),
            (None, None) => panic!("nothing to copy")
        };
        let (head, tail) = self.slices_replace(len);
        let split = head.len();

        if let Some(dst) = dst {
            dst[..split].copy_from_slice(&head);
            dst[split..].copy_from_slice(&tail);
        }
        if let Some(src) = src {
            head.copy_from_slice(&src[..split]);
            tail.copy_from_slice(&src[split..]);
        } else {
            for x in head.iter_mut().chain(tail.iter_mut()) {
                *x = Default::default();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ring_buffer() {
        let mut buf = RingBuffer::<u8>::new(8, false);

        for (y, x) in buf.iter_append(5).zip(1..6) {
            *y = x;
        }
        assert_eq!(buf.len(), 5);

        buf.remove(3);
        assert_eq!(buf.len(), 2);

        for (y, x) in buf.iter_append(6).zip(6..12) {
            *y = x;
        }
        dbg!(&buf);
        assert!(buf.iter(0).copied().eq(4..12));
        assert!(buf.iter(2).copied().eq(6..12));
        assert!(buf.iter(-2).copied().eq(10..12));

        buf.iter_mut(-2).for_each(|x| *x = 42);

        dbg!(&buf);

        assert!(buf.iter(-2).copied().eq(std::iter::repeat(42).take(2)));

        assert_eq!(buf.len(), 8);

        for x in buf.iter_remove(4) {
            dbg!(x);
        }

        for x in buf.iter_remove(4) {
            dbg!(x);
        }

        buf.copy_append(&[13; 7]);
        dbg!(&buf);

        assert!(buf.iter(0).copied().eq(std::iter::repeat(13).take(7)));

        let mut out = [0u8; 4];
        buf.copy_remove(&mut out);

        assert_eq!(out, [13u8; 4]);

        dbg!(&buf);

        // panic!("##########################");
    }

    #[test]
    fn ring_buffer_full() {
        let mut buf = RingBuffer::<u8>::new(4, true);

        buf.copy_replace(Some(&[1;2]), None);

        let out = &mut [0; 4];
        buf.copy_replace(None, Some(out));

        assert_eq!(out, &[0, 0, 1, 1]);

        dbg!(&buf);
        buf.copy_replace(Some(&[2;2]), None);
    }
}