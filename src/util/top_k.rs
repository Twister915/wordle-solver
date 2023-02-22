/*
 * MIT License
 *
 * Copyright (c) 2022 Joseph Sacchini
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use std::iter::FusedIterator;
use std::ops::Range;

pub struct TopK<E, const K: usize> {
    items: [Option<E>; K],
    alive: Range<usize>,
}

impl<Element, const K: usize> TopK<Element, K> {
    pub fn new<Itr, Score, ScoringFunc>(iter: Itr, f: ScoringFunc) -> Self
    where
        Itr: Iterator<Item = Element>,
        ScoringFunc: Fn(&Element) -> Score,
        Score: PartialOrd<Score>,
        [Option<Element>; K]: Default,
        [Option<Score>; K]: Default,
    {
        // these two arrays are coordinated such that if scores[x].is_some() then items[x].is_some()
        // scores[x] is f(&items[x])
        let mut items: [Option<Element>; K] = Default::default();
        let mut scores: [Option<Score>; K] = Default::default();
        let mut size = 0;

        // exhaust the iterator (look at every item)
        for next in iter {
            // compute score
            let score = f(&next);

            // find if the score is larger than anything in our array currently
            for i in 0..K {
                // we should insert if we are larger OR if the slot is available
                if if let Some(other) = &scores[i] {
                    other < &score
                } else {
                    true
                } {
                    // insert score and item
                    array_insert(&mut scores, Some(score), i);
                    array_insert(&mut items, Some(next), i);

                    // ensure size is correct
                    if size < K {
                        size += 1;
                    }

                    // this break combined with the structure of this loop ensures that
                    // the arrays are always sorted from greatest -> least score value
                    break;
                }
            }
        }

        Self {
            items,
            alive: 0..size,
        }
    }
}

impl<Element, const K: usize> Iterator for TopK<Element, K> {
    type Item = Element;

    fn next(&mut self) -> Option<Self::Item> {
        self.alive.next().and_then(|idx| self.items[idx].take())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.alive.end))
    }
}

impl<Element, const K: usize> ExactSizeIterator for TopK<Element, K> {}
impl<Element, const K: usize> FusedIterator for TopK<Element, K> {}

#[inline]
fn array_insert<E, const N: usize>(elems: &mut [E; N], mut tmp: E, idx: usize) {
    #[allow(clippy::needless_range_loop)]
    for i in idx..N {
        std::mem::swap(&mut tmp, &mut elems[i]);
    }
}

pub trait TopKExt: Iterator + Sized {
    fn top_k<Score, ScoreFn, const N: usize>(self, score_f: ScoreFn) -> TopK<Self::Item, N>
    where
        ScoreFn: Fn(&Self::Item) -> Score,
        Score: PartialOrd<Score>,
        [Option<Self::Item>; N]: Default,
        [Option<Score>; N]: Default,
    {
        TopK::new(self, score_f)
    }
}

impl<I> TopKExt for I where I: Iterator + Sized {}
