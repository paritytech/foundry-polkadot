<atom>
  <type>workflow</type>
  <diataxis_quadrant>how-to</diataxis_quadrant>
  <intent>create_new_skill_comprehensive</intent>
</atom>

# Skill Creation Workflows: S.T.A.R. Method

<nav>
  <tag name="overview" section="Overview" />
  <tag name="setup-phase" section="Phase 1: Setup — Structure the Skill" />
  <tag name="task-phase" section="Phase 2: Task — Define User Intent" />
  <tag name="analysis-phase" section="Phase 3: Analysis — Design Content Architecture" />
  <tag name="result-phase" section="Phase 4: Result — Validate and Ship" />
  <tag name="examples" section="Real-World Examples" />
  <tag name="anti-patterns" section="Anti-Patterns to Avoid" />
  <tag name="checklists" section="Validation Checklists" />
</nav>

## Overview

**What This Is**: Comprehensive workflow for creating new OpenCode skills from scratch using the S.T.A.R. method (Setup → Task → Analysis → Result). Covers end-to-end process with decision matrices, validation checklists, and production examples.

**When to Use**:

- Creating a new skill for the first time
- Migrating content from another format to OpenCode skills
- Establishing skill architecture for a technical domain
- Onboarding team members to skill creation process

**Target Outcome**: Production-ready skill (SkillScore > 90) with clear navigation, comprehensive guides, and validated quality gates

**Success Criteria**:

- S.T.A.R. phases completed in order
- SKILL.md with functional decision matrix
- All 10 boolean gates passing
- SkillScore ≥ 90/100
- Clear semantic domains identified

---

## Phase 1: Setup — Structure the Skill

### Objective

Establish the directory structure and foundational files for a new skill.

### Step 1.1: Initialize Skill Directory

```bash
# From repository root
mkdir -p .claude/skills/<skill-name>/references
cd .claude/skills/<skill-name>

# Example
mkdir -p .claude/skills/terraform-cookbook/references
```

### Step 1.2: Create SKILL.md Skeleton

**Template**:

```markdown
---
name: skill-name
description: "Concise description with keywords. Triggered when user asks..."
# Official API Configuration
disable-model-invocation: false # Set true for manual-only skills
user-invocable: true            # Set false for hidden background context
allowed-tools: Read, Grep       # Restrict tool access if needed
argument-hint: "[arg1] [arg2]"  # Hint for slash command autocomplete
compatibility: opencode         # OpenCode standard compatibility
---

# <Skill-Name>

> Brief 1-2 sentence description of what this skill covers and when to use it.

## Quick Start

[3-5 bullet points: Installation, basic usage, first example]

## Core Concepts

[2-3 paragraphs explaining the foundational knowledge needed]

## Workflows

<decision_matrix>
<question>What task do you need?</question>

<option condition="Specific user intent">
  <navigate_to>references/comprehensive-guide.md</navigate_to>
  <description>Clear description of what this covers</description>
</option>

<option condition="Another user intent">
  <navigate_to>references/another-guide.md</navigate_to>
  <description>Clear description</description>
</option>
</decision_matrix>

## Common Patterns

[Table or list of 5-7 common patterns with links]

## Anti-Patterns

[3-5 anti-patterns with warnings]

## See Also

[Related skills, external docs]
```

### Step 1.3: Define Skill Metadata and Advanced Config

**Advanced Configuration Options** (Official Claude API):

| Field            | Description                     | Use Case                                               |
| ---------------- | ------------------------------- | ------------------------------------------------------ |
| `context: fork`  | Runs skill in isolated subagent | Deep research, dangerous operations                    |
| `agent: Explore` | Selects subagent type           | Use `Explore` for code search, `Plan` for architecture |
| `allowed-tools`  | Whitelist available tools       | Security containment (e.g., Read-only)                 |
| `hooks`          | Lifecycle event triggers        | Run scripts on skill activation                        |

**Dynamic Context Injection**:
Use `!`command`` syntax to inject shell output into your skill prompt:

```yaml
---
name: pr-review
---
Review this PR:
Diff: !`gh pr diff`
Comments: !`gh pr view --comments`
```

**Decision Matrix: Skill Type Selection**

| Primary User Need         | Diátaxis Quadrant | Example Skills                |
| ------------------------- | ----------------- | ----------------------------- |
| "How do I do X?"          | how-to            | effect-ts, terraform-cookbook |
| "Teach me X from scratch" | tutorial          | getting-started-guides        |
| "How does X work?"        | explanation       | individuality-sdk             |
| "What are the APIs?"      | reference         | api-specifications            |

### Step 1.4: Identify Semantic Domains

Before creating content, map the semantic domains your skill will cover:

**Example: Effect-TS Skill**:

```yaml
Core Concepts Domain:
  - effect-basics.md (Effect introduction)
  - error-handling.md (Either, Option patterns)
  - async-patterns.md (Effect.async, fibers)
  → Comprehensive: effect-basics-comprehensive.md (900-1300 lines)

Service Architecture Domain:
  - service-layer.md (Layer design)
  - dependency-injection.md (Context, layers)
  - testing-services.md (Test layers)
  → Comprehensive: service-architecture-comprehensive.md (800-1000 lines)

Testing Domain:
  - unit-testing.md (Arbitraries, Properties)
  - integration-testing.md (TestLayers, TestContainers)
  → Comprehensive: testing-comprehensive.md (800-1000 lines)
```

**Checkpoint before proceeding**:

- [ ] Directory structure created
- [ ] SKILL.md skeleton in place
- [ ] Semantic domains mapped
- [ ] Decision matrix outlined (even if links don't exist yet)

---

## Phase 2: Task — Define User Intent

### Objective

Clarify the specific user intents your skill will address and map them to content.

### Step 2.1: Identify User Personas

**Example Personas**:

| Persona                | Experience | Goal                                    | Example Query                               |
| ---------------------- | ---------- | --------------------------------------- | ------------------------------------------- |
| **Backend Engineer**   | 3-5 years  | Build production Effect services        | "How do I structure a service layer?"       |
| **Platform Developer** | 5-10 years | Integrate Effect into existing codebase | "Migration patterns for Express to Effect?" |
| **Junior Developer**   | 0-2 years  | Learn Effect basics                     | "What is an Effect and why use it?"         |

### Step 2.2: Map Intents to Content

**Decision Matrix: Intent → Content Mapping**

| User Intent                     | Content Type | File                                  | Size Target     |
| ------------------------------- | ------------ | ------------------------------------- | --------------- |
| "Learn Effect basics"           | Tutorial     | effect-foundation-comprehensive.md    | 1000-1300 lines |
| "Handle errors correctly"       | How-to       | error-handling-comprehensive.md       | 800-1100 lines  |
| "Test my services"              | How-to       | testing-comprehensive.md              | 800-1100 lines  |
| "Understand Layer architecture" | Explanation  | service-architecture-comprehensive.md | 900-1200 lines  |

### Step 2.3: Draft Golden Queries

**Golden Queries**: Test cases for decision matrix effectiveness

**Example Golden Queries for Effect-TS**:

| Query                               | Expected Path                                              | Rationale                 |
| ----------------------------------- | ---------------------------------------------------------- | ------------------------- |
| "How do I make an HTTP call?"       | effect-basics-comprehensive.md → Effect.request section    | Common pattern            |
| "How do I test a database service?" | testing-comprehensive.md → TestLayers section              | Specific testing scenario |
| "How do I migrate from Express?"    | service-architecture-comprehensive.md → Migration patterns | Advanced integration      |
| "Why use Effect over async/await?"  | effect-foundation-comprehensive.md → Rationale section     | Conceptual question       |
| "My service needs config, how?"     | service-architecture-comprehensive.md → Context section    | Dependency injection      |

**Validation**: For each query, trace the path through your decision matrix. If any query doesn't have a clear path, adjust the matrix.

### Step 2.4: Define Content Boundaries

**Avoid Overlap**: Ensure each comprehensive guide has a distinct purpose

| Domain               | Includes                           | Excludes (belongs to...)       |
| -------------------- | ---------------------------------- | ------------------------------ |
| Effect Basics        | Core Effect, async, error handling | Service layer, testing         |
| Service Architecture | Layer design, DI, Context          | Basic Effect concepts, testing |
| Testing              | Unit tests, integration tests      | Service architecture basics    |

**Decision Matrix: Boundary Conflicts**

| Conflict Symptom                     | Resolution                                  |
| ------------------------------------ | ------------------------------------------- |
| Same topic in 2 files                | Consolidate into single comprehensive guide |
| Topic feels squeezed between domains | Create new domain or adjust boundaries      |
| Guide exceeds 2000 lines             | Split at natural boundary into 2 guides     |
| Guide < 300 lines                    | Merge with related guide                    |

---

## Phase 3: Analysis — Design Content Architecture

### Objective

Plan the structure of each comprehensive guide before writing content.

### Step 3.1: Design Guide Templates

**Standard Comprehensive Guide Structure**:

```markdown
<atom>
  <type>comprehensive_guide</type>
  <diataxis_quadrant>how_to|tutorial|explanation|reference</diataxis_quadrant>
  <intent>specific_user_intent</intent>
</atom>

# Title: Comprehensive Guide

<nav>
  <tag name="overview" section="Overview" />
  <tag name="prerequisites" section="Prerequisites" />
  <tag name="core-concepts" section="Core Concepts" />
  <tag name="patterns" section="Common Patterns" />
  <tag name="examples" section="Real-World Examples" />
  <tag name="troubleshooting" section="Troubleshooting" />
  <tag name="see-also" section="See Also" />
</nav>

## Overview

[What this guide covers, who it's for, 3-5 sentences]

## Prerequisites

[Required knowledge, setup steps, 3-5 bullet points]

## Core Concepts

[Main content: explanations, patterns, decisions]

### Pattern 1: Name

- **When to use**: Condition
- **How it works**: Brief explanation
- **Example**: Code snippet

### Pattern 2: Name

[Same structure]

## Common Patterns

[Reusable patterns with examples]

## Real-World Examples

[2-3 complete examples from production code]

### Example 1: [Descriptive Title]

**Context**: What problem this solves
**Implementation**: Code or steps
**Outcome**: Result or benefit

## Troubleshooting

[Table: Issue | Symptom | Solution]

## See Also

[Related guides, external docs]
```

### Step 3.2: Plan Content Sizes

**Target Sizing by Type**:

| Content Type     | Min Lines | Target Lines | Max Lines |
| ---------------- | --------- | ------------ | --------- |
| **How-to Guide** | 300       | 800-1200     | 2000      |
| **Tutorial**     | 400       | 800-1200     | 1500      |
| **Explanation**  | 300       | 500-900      | 1100      |
| **Reference**    | 200       | 300-600      | 800       |

**Philosophy**: Semantic cohesion > arbitrary limits. If content needs 1100 lines to cover a semantic domain cohesively, that's preferable to splitting it into two 550-line fragments.

### Step 3.3: Add Navigation for Large Guides

**Adaptive Purity**: Files > 300 lines should have `<nav>` tags

```xml
<nav>
  <tag name="overview" section="Overview" />
  <tag name="prerequisites" section="Prerequisites" />
  <tag name="core-concepts" section="Core Concepts" />
  <tag name="pattern-1" section="Pattern 1: Name" />
  <tag name="pattern-2" section="Pattern 2: Name" />
  <tag name="examples" section="Real-World Examples" />
  <tag name="troubleshooting" section="Troubleshooting" />
</nav>
```

**Decision Matrix: Navigation Strategy**

| File Size      | Navigation                    | Rationale                              |
| -------------- | ----------------------------- | -------------------------------------- |
| < 300 lines    | No internal links (pure atom) | Small enough to scan                   |
| 300-600 lines  | Optional `<nav>`              | Medium size, navigation helpful        |
| 600-1200 lines | Recommended `<nav>`           | Large, needs section navigation        |
| > 2000 lines   | Split into 2 files            | Consider splitting at natural boundary |

### Step 3.4: Validate Decision Matrix Completeness

**Test your decision matrix** against golden queries:

```bash
# For each golden query, trace:
Query: "How do I test a database service?"

Path Check:
1. SKILL.md decision matrix → "Testing" option
2. Navigate to: testing-comprehensive.md
3. Check <nav> → "database-testing" section exists
4. Verify section covers: TestContainers, TestLayers, mocks
```

**IF** any query fails the trace, **THEN** update:

- Decision matrix option OR
- Guide structure OR
- Section content

---

## Phase 4: Result — Validate and Ship

### Objective

Verify the skill meets production-ready standards before shipping.

### Step 4.1: Run Boolean Gates

```bash
python3 .claude/skills/skill-creator/scripts/validate_skill.py .claude/skills/<skill-name>
```

**Expected**: All 10 gates PASS

**Common Failures and Fixes**:

| Gate                    | Failure Mode                       | Fix                       |
| ----------------------- | ---------------------------------- | ------------------------- |
| `has_skill_md`          | No SKILL.md in root                | Create SKILL.md           |
| `has_references_dir`    | No references/ directory           | Create directory          |
| `has_decision_matrix`   | No `<decision_matrix>` in SKILL.md | Add decision matrix       |
| `all_files_markdown`    | Non-.md files in references/       | Convert to .md            |
| `no_dead_links`         | Broken internal links              | Fix link paths            |
| `atom_metadata_present` | Files missing `<atom>` block       | Add metadata              |
| `file_size_minimum`     | Files < 100 lines                  | Consolidate or expand     |
| `max_file_size`         | Files > 2000 lines                 | Split at natural boundary |
| `skillscore_threshold`  | SkillScore < 80                    | Improve content quality   |
| `nav_tags_valid`        | Invalid `<nav>` syntax             | Fix XML format            |

### Step 4.2: Calculate SkillScore

```bash
python3 .claude/skills/skill-creator/scripts/skillscore_evaluator.py .claude/skills/<skill-name>/SKILL.md
```

**Score Bands**:

| Score  | Quality Band | Action                               |
| ------ | ------------ | ------------------------------------ |
| 90-100 | Excellent    | Ship ✅                              |
| 85-89  | Good         | Review improvements, optionally ship |
| 80-84  | Fair         | Improve before shipping              |
| < 80   | Poor         | Major improvements needed            |

**IF** SkillScore < 90, **THEN** proceed to Step 4.3.

### Step 4.3: Iterative Improvement

**Identify lowest-scoring components** and fix systematically:

**Common SkillScore Improvements**:

| Component             | Low Score Causes                       | Fixes                                   |
| --------------------- | -------------------------------------- | --------------------------------------- |
| **Navigation**        | Missing decision matrix, unclear paths | Add matrix entries, improve routing     |
| **Semantic Cohesion** | Fragmented content, arbitrary splits   | Consolidate into comprehensive guides   |
| **Content Quality**   | Weak examples, outdated info           | Add real-world examples, update content |
| **Completeness**      | Missing domains, gaps                  | Add missing sections or guides          |
| **Clarity**           | Verbose explanations, poor structure   | Apply C.L.E.A.R. principles             |

**Iteration Process**:

1. **Identify** lowest-scoring component (e.g., Navigation = 6/10)
2. **Apply** targeted fix (e.g., add 3 decision matrix entries)
3. **Re-validate** (boolean gates + SkillScore)
4. **Repeat** until SkillScore ≥ 90

**Max iterations**: 3. If not improving, reconsider architecture.

### Step 4.4: Final Verification Checklist

**Pre-Ship Checklist**:

- [ ] All 10 boolean gates PASS
- [ ] SkillScore ≥ 90/100
- [ ] Decision matrix covers all golden queries
- [ ] No files < 100 lines (except pure specs)
- [ ] No files > 2000 lines
- [ ] All comprehensive guides have `<nav>` tags
- [ ] No dead links or broken references
- [ ] Semantic domains clearly separated
- [ ] Examples are real-world and working
- [ ] Anti-patterns documented

**Post-Ship Monitoring**:

- Track user queries that don't match decision matrix
- Identify content gaps from user feedback
- Update examples as APIs evolve
- Re-run SkillScore evaluation quarterly

---

## Real-World Examples

### Example 1: Effect-TS Skill Creation

**Initial State**:

- 18 scattered files
- Avg 97 lines/file
- SkillScore: 72/100
- No decision matrix

**S.T.A.R. Process**:

**Phase 1: Setup**

- Created skill directory structure
- Drafted SKILL.md with decision matrix skeleton
- Identified 3 semantic domains: Basics, Service Architecture, Testing

**Phase 2: Task**

- Mapped user intents: "Learn Effect", "Build services", "Test Effect code"
- Created golden queries: 10 test cases
- Defined content boundaries for each domain

**Phase 3: Analysis**

- Designed 3 comprehensive guides (target: 1000-1500 lines each)
- Planned <nav> tags for each guide
- Structured decision matrix with 7 options

**Phase 4: Result**

- Consolidated 18 files → 7 files
- Boolean gates: 10/10 PASS
- SkillScore: 93/100
- Decision matrix covers all golden queries

**Final Structure**:

```
effect-ts/
├── SKILL.md (comprehensive decision matrix)
└── references/
    ├── effect-basics-comprehensive.md (850 lines)
    ├── service-architecture-comprehensive.md (720 lines)
    ├── testing-comprehensive.md (680 lines)
    ├── error-handling-patterns.md (320 lines)
    ├── async-patterns.md (280 lines)
    └── integration-patterns.md (250 lines)
```

### Example 2: Terraform-Cookbook Skill

**User Intent**: "How do I manage infrastructure with Terraform?"

**Decision Matrix Design**:

```markdown
<decision_matrix>
<question>What infrastructure task do you need?</question>

<option condition="Create new AWS resource">
  <navigate_to>references/aws-resources-comprehensive.md</navigate_to>
  <description>EC2, S3, RDS, VPC patterns with examples</description>
</option>

<option condition="Manage existing infrastructure">
  <navigate_to>references/terraform-state-comprehensive.md</navigate_to>
  <description>State management, imports, drift detection</description>
</option>

<option condition="Set up CI/CD pipeline">
  <navigate_to>references/terraform-ci-cd-comprehensive.md</navigate_to>
  <description>GitHub Actions, Atlantis, workflow patterns</description>
</option>

<option condition="Write reusable modules">
  <navigate_to>references/module-development-comprehensive.md</navigate_to>
  <description>Module structure, testing, publishing</description>
</option>

<option condition="Troubleshoot deployment issues">
  <navigate_to>references/troubleshooting-comprehensive.md</navigate_to>
  <description>Common errors, debugging techniques</description>
</option>
</decision_matrix>
```

**Result**: 5 comprehensive guides, avg 750 lines, SkillScore 94/100

---

## Anti-Patterns to Avoid

❌ **Skipping S.T.A.R. phases**: Jumping straight to content creation

- **Impact**: Poor structure, unclear navigation, low SkillScore
- **Fix**: Always complete Setup → Task → Analysis → Result in order

❌ **Writing before planning**: Creating content without semantic domain mapping

- **Impact**: Fragmented content, duplication, gaps
- **Fix**: Map semantic domains and user intents first (Phase 2)

❌ **Ignoring decision matrix**: Creating comprehensive guides without updating navigation

- **Impact**: Users can't find content, low utility
- **Fix**: Update decision matrix after each guide is created

❌ **Aggressive atomization**: Splitting content into tiny files (< 100 lines)

- **Impact**: Poor semantic cohesion, excessive navigation
- **Fix**: Consolidate into comprehensive guides (800-1500 lines)

❌ **Ship without validation**: Skipping boolean gates and SkillScore

- **Impact**: Low-quality skill, broken links, poor UX
- **Fix**: Always run validation before shipping (Phase 4)

✅ **Follow S.T.A.R. method**: Setup → Task → Analysis → Result
✅ **Plan before writing**: Map semantic domains and user intents
✅ **Update decision matrix**: Keep navigation in sync with content
✅ **Create comprehensive guides**: 800-1500 lines for semantic cohesion
✅ **Validate before shipping**: Boolean gates + SkillScore ≥ 90

---

## Validation Checklists

### Phase 1: Setup Checklist

- [ ] Skill directory created
- [ ] references/ subdirectory exists
- [ ] SKILL.md skeleton in place
- [ ] Metadata atom added to SKILL.md
- [ ] Semantic domains mapped (3-7 domains)
- [ ] Decision matrix skeleton created

### Phase 2: Task Checklist

- [ ] User personas identified
- [ ] Golden queries drafted (5-10 queries)
- [ ] Intent → Content mapping complete
- [ ] Content boundaries defined
- [ ] No overlap conflicts between domains

### Phase 3: Analysis Checklist

- [ ] Guide templates designed
- [ ] Target sizes planned (800-1500 lines)
- <nav> tags planned for large guides
- [ ] Decision matrix validated against golden queries
- [ ] All queries trace to specific sections

### Phase 4: Result Checklist

- [ ] All comprehensive guides created
- [ ] All guides have <atom> metadata
- [ ] All guides > 300 lines have <nav> tags
- [ ] Boolean gates: 10/10 PASS
- [ ] SkillScore ≥ 90/100
- [ ] No files < 100 lines (except specs)
- [ ] No files > 2000 lines
- [ ] Decision matrix functional
- [ ] Golden queries validated
- [ ] Ready to ship ✅

---

## Decision Matrices

### Matrix 1: Skill Type Selection

| Primary User Need    | Diátaxis Quadrant | Decision Matrix Focus      |
| -------------------- | ----------------- | -------------------------- |
| "How do I do X?"     | how-to            | Task-based options         |
| "Teach me X"         | tutorial          | Progressive learning paths |
| "How does X work?"   | explanation       | Concept-based navigation   |
| "What are the APIs?" | reference         | Component listings         |

### Matrix 2: Guide Size Planning

| Semantic Domain     | Estimated Patterns | Target Size     | Split If     |
| ------------------- | ------------------ | --------------- | ------------ |
| Single concept      | 3-5 patterns       | 300-500 lines   | N/A          |
| Multi-pattern task  | 5-10 patterns      | 800-1000 lines  | > 1800 lines |
| End-to-end workflow | 10-15 patterns     | 1200-1500 lines | > 2500 lines |
| API reference       | 20+ components     | 800-1200 lines  | > 2000 lines |

### Matrix 3: Validation Priority

| Issue Found             | Priority | Action                    | Block Ship? |
| ----------------------- | -------- | ------------------------- | ----------- |
| SkillScore < 80         | HIGH     | Improve content quality   | YES         |
| Boolean gate fail       | HIGH     | Fix specific gate         | YES         |
| Missing decision matrix | HIGH     | Add matrix                | YES         |
| File < 100 lines        | MEDIUM   | Consolidate or expand     | NO          |
| File > 1200 lines       | MEDIUM   | Split if natural boundary | NO          |
| Weak examples           | LOW      | Add real-world examples   | NO          |

---

## See Also

- [Workflow: Bottom-Up Reconstruction](workflow-bottom-up-reconstruction-comprehensive-guide.md) - Refactoring degraded skills
- [Progressive Disclosure: Tiers 1-3](progressive-disclosure-comprehensive-guide.md) - Size guidelines and architecture
- [Validation: Quality Gates](validation-quality-gates.md) - Boolean gate specifications
- [Navigation: Tags Specification](navigation-tags-specification.md) - JIT loading format
