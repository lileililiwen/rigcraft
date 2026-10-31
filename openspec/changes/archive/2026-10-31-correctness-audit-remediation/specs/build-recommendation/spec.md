# Recommendation export correctness delta

## MODIFIED Requirements

### Requirement: Export round-trips the shown report with its versions
The system SHALL export a stored recommendation as versioned machine-readable JSON and as a human-readable Markdown plan. The JSON export SHALL contain `exportVersion` `recommend-export-v1`, the export time, the snapshot id, revision, currency, source filename and checksum, the exact stored report object, and one entry per candidate naming its check report id, rule-set version, overall status, summary and unknown follow-ups. The Markdown plan SHALL repeat the same options, prices, stock, compatibility verdicts, reasons, alternatives, unknown follow-ups, evidence source references and a manual-confirmation notice. Every dynamic Markdown table cell SHALL escape backslash as `\\` and pipe as `\|`, replace line breaks with a space, and neutralise `<` and `>` as `&lt;` and `&gt;`, so operator text can neither add a column nor inject HTML. The JSON export SHALL remain exact data and SHALL NOT apply these escapes. Export SHALL NOT recompute prices or verdicts. An unsupported `format` query value SHALL return HTTP 400 with `export_format_unsupported`, and an unknown report SHALL return HTTP 404 with `recommendation_not_found`.

#### Scenario: Export matches the stored report
Given a generated recommendation, when the JSON export is downloaded, then its `report` object equals the stored report and each candidate's `checkReportId` matches the embedded check report.

#### Scenario: Human-readable export carries the same facts
Given a generated recommendation, when the Markdown export is downloaded, then it names the options, budget use, compatibility verdicts, check report identifiers, evidence sources and the manual-confirmation notice.

#### Scenario: Operator text cannot break the Markdown table
Given a part model containing a pipe, a backslash and an HTML tag, when the Markdown export is generated, then the pipe and backslash are escaped, the tag is rendered as text, and every row of the parts table still has the same declared column count.
