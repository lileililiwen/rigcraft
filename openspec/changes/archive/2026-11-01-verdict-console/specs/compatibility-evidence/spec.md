# compatibility-evidence deltas

## ADDED Requirements

### Requirement: Missing required parts are named once with an actionable follow-up
When a build is incomplete, every missing required category SHALL be reported with
a `reason` that is a complete sentence ending in a period and a `followUp` that
names the action to take. The follow-up SHALL NOT repeat the reason's clause, SHALL
NOT contain the phrase `is required`, and SHALL read as a direct instruction — for
example reason `A motherboard is required.` with follow-up `Select a motherboard
from this snapshot.` The same rule SHALL apply to CPU, motherboard, memory, power
supply, case, startup storage, graphics card and cooler entries, including the
graphics-card and cooler variants that offer a sourced-evidence alternative.

#### Scenario: A missing motherboard is named once
Given a selection that contains a CPU but no motherboard, when the build is checked, then the motherboard entry's reason is `A motherboard is required.` and its follow-up is `Select a motherboard from this snapshot.`

#### Scenario: No follow-up repeats the requirement phrase
Given any incomplete build, when every missing entry's follow-up is searched for `is required`, then no follow-up contains that phrase and no follow-up duplicates its reason.

#### Scenario: The graphics-card variant stays actionable
Given a build with no graphics card and no sourced integrated-graphics evidence, when the build is checked, then the entry's reason names the missing evidence and its follow-up offers selecting a card or recording `igpu=yes` on the CPU with its source.
