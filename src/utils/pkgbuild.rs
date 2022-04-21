pub fn sanitize_dependency(dep: &str) -> String {
    let mut char_index = 0;
    for c in vec![">", "<", "="] {
        let found = dep.find(c).unwrap_or(0);
        if char_index == 0 || found < char_index {
            char_index = found;
        }
    }
    if char_index > 0 {
        return dep[..char_index - 1].to_string();
    }
    dep.to_string()
}

#[test]
fn sanitizes_correctly()
{
    assert_eq!("glibc", sanitize_dependency("glibc>=2.63"))
}