pub fn assert_str_eq_by_lines(
  expected: &str,
  actual: &str,
) {
  if expected == actual {
    return;
  }

  let exp_lines: Vec<&str> = expected.lines().collect();
  let act_lines: Vec<&str> = actual.lines().collect();
  let max_len = exp_lines.len().max(act_lines.len());

  let mut diffs = Vec::new();
  for i in 0..max_len {
    let e = exp_lines.get(i).copied();
    let a = act_lines.get(i).copied();
    match (e, a) {
      (Some(e), Some(a)) if e == a => {}
      (Some(e), Some(a)) => diffs.push(format!("{i}: -{e}\n   +{a}")),
      (Some(e), None) => diffs.push(format!("{i}: -{e}\n   +<missing>")),
      (None, Some(a)) => diffs.push(format!("{i}: -<missing>\n   +{a}")),
      _ => {}
    }
  }

  let shown = diffs.iter().take(30).cloned().collect::<Vec<_>>().join("\n");
  let more =
    if diffs.len() > 30 { format!("\n... and {} more differing lines", diffs.len() - 30) } else { String::new() };

  panic!(
    "String mismatch ({} vs {} lines). Diff by line:\n{}{}\n\nactual:{}",
    exp_lines.len(),
    act_lines.len(),
    shown,
    more,
    actual
  );
}
