#[macro_export]
macro_rules! assert_equiv {
    ($left:expr, $right:expr) => {
        let left = $left;
        let right = $right;

        assert!(
            left.len() == right.len(),
            "left length ({}) did not match right length ({})",
            left.len(),
            right.len()
        );

        for v in left {
            assert!(
                right.contains(&v),
                "element {:?} from left not in right (left = {:?}, right = {:?})",
                v,
                left,
                right
            );
        }
    };
}
