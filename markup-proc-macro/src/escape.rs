pub fn escape(str: &[u8], writer: &mut impl std::io::Write) -> std::io::Result<()> {
    let mut last = 0;
    for (index, byte) in str.iter().enumerate() {
        macro_rules! go {
            ($expr:expr) => {{
                // SAFETY: We know that last < index and that index is valid
                unsafe {
                    writer.write_all(&str.get_unchecked(last..index))?;
                }
                writer.write_all($expr)?;
                // This will only wrap if index reaches usize::MAX
                last = index.wrapping_add(1);
            }};
        }

        match byte {
            b'&' => go!(b"&amp;"),
            b'<' => go!(b"&lt;"),
            b'>' => go!(b"&gt;"),
            b'"' => go!(b"&quot;"),
            _ => {}
        }
    }

    // SAFETY: last can only overflow if str.len() == usize::MAX but slices can at max be isize::MAX
    unsafe {
        writer.write_all(str.get_unchecked(last..))
    }
}

pub struct Escape<'a, W>(pub &'a mut W);

impl<W: std::io::Write> std::io::Write for Escape<'_, W> {
    #[inline]
    fn write(&mut self, s: &[u8]) -> std::io::Result<usize> {
        escape(s, &mut self.0).map(|()| s.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

#[test]
fn test() {
    t("", "");
    t("<", "&lt;");
    t("a<", "a&lt;");
    t("<b", "&lt;b");
    t("a<b", "a&lt;b");
    t("a<>b", "a&lt;&gt;b");
    t("<>", "&lt;&gt;");
    t("≤", "≤");
    t("a≤", "a≤");
    t("≤b", "≤b");
    t("a≤b", "a≤b");
    t("a≤≥b", "a≤≥b");
    t("≤≥", "≤≥");
    t(
        r#"foo &<>" bar&bar<bar>bar"bar baz&&<<baz>>""baz"#,
        r#"foo &amp;&lt;&gt;&quot; bar&amp;bar&lt;bar&gt;bar&quot;bar baz&amp;&amp;&lt;&lt;baz&gt;&gt;&quot;&quot;baz"#,
    );

    fn t(input: &str, output: &str) {
        let mut string = Vec::new();
        escape(input.as_bytes(), &mut string).unwrap();
        assert_eq!(string, output.as_bytes());
    }
}

#[test]
fn test_arguments() {
    use std::io::Write;

    t("", "&quot;&quot;");
    t("<", "&quot;&lt;&quot;");
    t("a<", "&quot;a&lt;&quot;");
    t("<b", "&quot;&lt;b&quot;");
    t("a<b", "&quot;a&lt;b&quot;");
    t("a<>b", "&quot;a&lt;&gt;b&quot;");
    t("<>", "&quot;&lt;&gt;&quot;");
    t("≤", "&quot;≤&quot;");
    t("a≤", "&quot;a≤&quot;");
    t("≤b", "&quot;≤b&quot;");
    t("a≤b", "&quot;a≤b&quot;");
    t("a≤≥b", "&quot;a≤≥b&quot;");
    t("≤≥", "&quot;≤≥&quot;");
    t(
        r#"foo &<>" bar&bar<bar>bar"bar baz&&<<baz>>""baz"#,
        r#"&quot;foo &amp;&lt;&gt;\&quot; bar&amp;bar&lt;bar&gt;bar\&quot;bar baz&amp;&amp;&lt;&lt;baz&gt;&gt;\&quot;\&quot;baz&quot;"#,
    );
    t('<', "'&lt;'");

    fn t(input: impl std::fmt::Debug, output: &str) {
        let mut string = Vec::new();
        write!(Escape(&mut string), "{}", format_args!("{:?}", input)).unwrap();
        assert_eq!(string, output.as_bytes());
    }
}
