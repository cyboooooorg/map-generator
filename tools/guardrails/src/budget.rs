//! Test-tier budgets (CLAUDE.md, Tests), checked on nextest's JUnit report.

use quick_xml::Reader;
use quick_xml::events::Event;

/// Wall time of the run, in seconds: the `time` attribute of the root
/// `testsuites` element (not the sum of the test cases, which run in
/// parallel).
pub fn wall_seconds(junit_xml: &str) -> Result<f64, String> {
    let mut reader = Reader::from_str(junit_xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(element) | Event::Empty(element))
                if element.name().as_ref() == "testsuites" =>
            {
                for attribute in element.attributes().flatten() {
                    if attribute.key.as_ref() == "time" {
                        let text = attribute.value.to_string();
                        return text
                            .parse()
                            .map_err(|_| format!("testsuites time is not a number: {text}"));
                    }
                }
                return Err("testsuites has no time attribute".to_owned());
            }
            Ok(Event::Eof) => return Err("no testsuites element".to_owned()),
            Err(error) => return Err(format!("JUnit report does not parse: {error}")),
            Ok(_) => {}
        }
    }
}

/// `Err` with the message to print when `seconds` passes `budget`.
pub fn check(tier: &str, seconds: f64, budget: f64) -> Result<(), String> {
    if seconds > budget {
        Err(format!(
            "{tier} took {seconds:.1} s, budget {budget:.0} s: move, merge or delete tests (see CLAUDE.md, Tests)"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="nextest-run" tests="2" failures="0" errors="0" time="131.4">
  <testsuite name="world" tests="2" time="200.0">
    <testcase name="a" time="100.0"/>
    <testcase name="b" time="100.0"/>
  </testsuite>
</testsuites>"#;

    #[test]
    fn budget_reads_the_run_wall_time() {
        assert_eq!(wall_seconds(REPORT), Ok(131.4));
    }

    #[test]
    fn over_budget_message_names_the_tier() {
        assert_eq!(
            check("tier 1", 131.4, 120.0),
            Err("tier 1 took 131.4 s, budget 120 s: move, merge or delete tests (see CLAUDE.md, Tests)".to_owned())
        );
        assert_eq!(check("tier 1", 119.0, 120.0), Ok(()));
    }

    #[test]
    fn a_report_without_testsuites_is_an_error() {
        assert!(wall_seconds("<x/>").is_err());
    }
}
