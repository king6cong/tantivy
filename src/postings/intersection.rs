use postings::{DocSet, DocSetGroup};
use postings::SkipResult;
use DocId;

/// Creates a `DocSet` that iterates through the intersection of two or more `DocSet`s.
pub struct IntersectionDocSet<TDocSet: DocSet> {
    docsets: Vec<TDocSet>,
    finished: bool,
    doc: DocId,
}

impl<TDocSet: DocSet> From<Vec<TDocSet>> for IntersectionDocSet<TDocSet> {
    fn from(mut docsets: Vec<TDocSet>) -> Self {
        assert!(docsets.len() >= 2);
        docsets.sort_by_key(|docset| docset.size_hint());
        IntersectionDocSet {
            docsets: docsets,
            finished: false,
            doc: 0u32,
        }
    }
}

impl<TDocSet: DocSet> DocSetGroup<TDocSet> for IntersectionDocSet<TDocSet> {
    fn docsets(&self) -> &[TDocSet] {
        &self.docsets[..]
    }
}

impl<TDocSet: DocSet> DocSet for IntersectionDocSet<TDocSet> {
    fn size_hint(&self) -> usize {
        self.docsets
            .iter()
            .map(|docset| docset.size_hint())
            .min()
            .unwrap() // safe as docsets cannot be empty.
    }

    #[allow(never_loop)]
    fn advance(&mut self) -> bool {
        if self.finished {
            return false;
        }

        let mut candidate_doc = self.doc;
        let mut candidate_ord = self.docsets.len();

        'outer: loop {

            for (ord, docset) in self.docsets.iter_mut().enumerate() {
                if ord != candidate_ord {
                    // `candidate_ord` is already at the
                    // right position.
                    //
                    // Calling `skip_next` would advance this docset
                    // and miss it.
                    match docset.skip_next(candidate_doc) {
                        SkipResult::Reached => {}
                        SkipResult::OverStep => {
                            // this is not in the intersection,
                            // let's update our candidate.
                            candidate_doc = docset.doc();
                            candidate_ord = ord;
                            continue 'outer;
                        }
                        SkipResult::End => {
                            self.finished = true;
                            return false;
                        }
                    }
                }
            }

            self.doc = candidate_doc;
            return true;
        }
    }

    fn skip_next(&mut self, target: DocId) -> SkipResult {
        if self.finished {
            return SkipResult::End;
        }

        for docset in &mut self.docsets {
            match docset.skip_next(target) {
                SkipResult::Reached => {}
                SkipResult::OverStep => {
                    self.doc = docset.doc();
                    return SkipResult::OverStep;
                }
                SkipResult::End => {
                    self.finished = true;
                    return SkipResult::End;
                }
            }
        }

        self.doc = target;
        SkipResult::Reached
    }

    fn doc(&self) -> DocId {
        self.doc
    }
}


#[cfg(test)]
mod tests {

    use postings::{DocSet, SkipResult, VecPostings, IntersectionDocSet};

    #[test]
    fn test_intersection() {
        {
            let left = VecPostings::from(vec![1, 3, 9]);
            let right = VecPostings::from(vec![3, 4, 9, 18]);
            let mut intersection = IntersectionDocSet::from(vec![left, right]);
            assert!(intersection.advance());
            assert_eq!(intersection.doc(), 3);
            assert!(intersection.advance());
            assert_eq!(intersection.doc(), 9);
            assert!(!intersection.advance());
        }
        {
            let a = VecPostings::from(vec![1, 3, 9]);
            let b = VecPostings::from(vec![3, 4, 9, 18]);
            let c = VecPostings::from(vec![1, 5, 9, 111]);
            let mut intersection = IntersectionDocSet::from(vec![a, b, c]);
            assert!(intersection.advance());
            assert_eq!(intersection.doc(), 9);
            assert!(!intersection.advance());
        }
    }

    #[test]
    fn test_intersection_zero() {
        let left = VecPostings::from(vec![0]);
        let right = VecPostings::from(vec![0]);
        let mut intersection = IntersectionDocSet::from(vec![left, right]);
        assert!(intersection.advance());
        assert_eq!(intersection.doc(), 0);
    }

    #[test]
    fn test_intersection_empty() {
        let a = VecPostings::from(vec![1, 3]);
        let b = VecPostings::from(vec![1, 4]);
        let c = VecPostings::from(vec![3, 9]);
        let mut intersection = IntersectionDocSet::from(vec![a, b, c]);
        assert!(!intersection.advance());
    }

    #[test]
    fn test_intersection_skip_next() {
        let a = VecPostings::from(vec![1, 3, 7, 10, 12, 14, 20]);
        let b = VecPostings::from(vec![1, 2, 3, 8, 10, 12, 14, 15, 20]);
        let c = VecPostings::from(vec![1, 2, 3, 9, 10, 12, 14, 20]);
        let mut intersection = IntersectionDocSet::from(vec![a, b, c]);
        assert_eq!(intersection.skip_next(1), SkipResult::Reached);
        assert_eq!(intersection.doc(), 1);
        assert_eq!(intersection.skip_next(2), SkipResult::OverStep);
        assert_eq!(intersection.doc(), 3);
        assert_eq!(intersection.skip_next(9), SkipResult::OverStep);
        assert_eq!(intersection.doc(), 10);
        assert!(intersection.advance());
        assert_eq!(intersection.doc(), 12);
        assert_eq!(intersection.skip_next(14), SkipResult::Reached);
        assert_eq!(intersection.doc(), 14);
        assert_eq!(intersection.skip_next(20), SkipResult::Reached);
        assert_eq!(intersection.doc(), 20);
        assert_eq!(intersection.skip_next(21), SkipResult::End);
        assert!(!intersection.advance());
        assert_eq!(intersection.skip_next(22), SkipResult::End);
    }
}
