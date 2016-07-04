use std::cmp::Ordering;
use std::cmp::Ordering::{Less, Equal, Greater};

/// Takes two *sorted* vectors and a comparison function
/// Gives back a tuple of vectors:
///  - one for the elements unique to the first vector
///  - one for the pairs of elements found equal
///  - one of the elements unique to the second vector
pub fn find_overlap<F, T>(mut left: Vec<T>,
                          mut right: Vec<T>,
                          mut fun: F)
                          -> (Vec<T>, Vec<(T, T)>, Vec<T>)
    where F: FnMut(&T, &T) -> Ordering
{
    let mut res_left = Vec::new();
    let mut res_right = Vec::new();
    let mut overlap = Vec::new();

    loop {
        match (left.pop(), right.pop()) {
            (Some(left_item), Some(right_item)) => {
                // sorted from small to large but pop takes from the end (large) side!
                match fun(&right_item, &left_item) {
                    Less => {
                        res_left.push(left_item);
                        right.push(right_item);
                    }
                    Equal => overlap.push((left_item, right_item)),
                    Greater => {
                        res_right.push(right_item);
                        left.push(left_item);
                    }
                }
            }
            (None, Some(right_item)) => res_right.push(right_item),
            (Some(left_item), None) => res_left.push(left_item),
            (None, None) => break,
        }
    }

    (res_left, overlap, res_right)
}

// The following code has been picked from the Rust programming language main repository:
// https://github.com/rust-lang/rust/blob/20183f498fbd8465859bf47611e1165768b9cc59/src/libtest/lib.rs#L664-L686
// To comply with the license of the code, the license is copied here. It only applies to the
//  function `fmt_thousands_sep`.
//
// Copyright (c) 2010 The Rust Project Developers
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//
// Format a number with thousands separators
pub fn fmt_thousands_sep(mut n: usize, sep: char) -> String {
    use std::fmt::Write;
    let mut output = String::new();
    let mut trailing = false;
    for &pow in &[9, 6, 3, 0] {
        let base = 10_usize.pow(pow);
        if pow == 0 || trailing || n / base != 0 {
            if !trailing {
                output.write_fmt(format_args!("{}", n / base)).unwrap();
            } else {
                output.write_fmt(format_args!("{:03}", n / base)).unwrap();
            }
            if pow != 0 {
                output.push(sep);
            }
            trailing = true;
        }
        n %= base;
    }

    output
}

fn drop_commas(s: &str) -> String {
    s.chars()
        .filter(|&b| b != ',')
        .collect::<String>()
}

pub fn drop_commas_and_parse(s: &str) -> Option<usize> {
    drop_commas(s).parse::<usize>().ok()
}
