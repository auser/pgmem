#[macro_export]
macro_rules! serde_json_eq {
    ($struct:ident, $s:literal, $field:ident, $result:expr) => {
        let a: $struct = serde_json::from_str($s).unwrap();
        assert_eq!(a.$field, $result);
    };
}

#[allow(unused)]
macro_rules! cmp_eq_option {
    ($left:expr, $right:expr) => {{
        match (&$left, &$right) {
            (Some(left_val), Some(right_val)) => *left_val == *right_val,
            (None, None) => true,
            _ => false,
        }
    }};
}

#[macro_export]
macro_rules! assert_eq_option {
  ($left:expr, $right:expr) => ({
      if !cmp_eq_option!($left, $right) {
          panic!(r#"assertion failed: `(left == right)`
left: `{:?}`,
right: `{:?}`"#, $left, $right)
      }
  });
  ($left:expr, $right:expr,) => ({
      assert_eq_option!($left, $right)
  });
  ($left:expr, $right:expr, $($arg:tt)+) => ({
      if !cmp_eq_option!($left, $right) {
          panic!(r#"assertion failed: `(left == right)`
left: `{:?}`,
right: `{:?}`: {}"#, $left, $right, format_args!($($arg)+))
      }
  });
}
