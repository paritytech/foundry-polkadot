# Progressive Disclosure for OpenCode Skills

> **TL;DR**: Don't overwhelm models with everything at once. Load metadata first (Tier 1), then instructions (Tier 2), then deep details (Tier 3) only when needed. This saves 70%+ tokens and improves accuracy.

## Why Progressive Disclosure Matters

Without progressive disclosure, a skill with 50+ topics would require loading **all** documentation into context immediately. This causes:

- **Context bloat**: 10k+ tokens before the actual task begins
- **Model confusion**: Instructions meant for database migrations interfere with CSS styling tasks
- **Token waste**: Paying for content never used in the current session
- **Accuracy loss**: Model can't distinguish relevant from irrelevant instructions

**Progressive disclosure solves this** by loading content **just-in-time** (JIT) based on task relevance.

---

## The 3-Tier Architecture

### Tier 1: Metadata (The "Menu")

**Always loaded** in system prompt (~100 tokens).

```yaml
---
name: skill-name
description: "Concise description with task-trigger keywords"
# Official API: Control visibility and timing
disable-model-invocation: false # Default: true. Set true to prevent auto-loading.
user-invocable: true            # Default: true. Set false to hide from menu.
compatibility: opencode
---
```

**Purpose**: Gives the model just enough information to decide _"I need this skill"_ without loading implementation details.

**Best Practices**:

- Keep description under 200 characters
- Include trigger keywords (e.g., "authentication", "database", "testing")
- Use `compatibility: opencode` for portable skills, `opencode-project` for project-specific

**Example**:

```yaml
---
name: effect-ts
description: "Production Effect-TS patterns: service architecture, error handling, testing, observability"
compatibility: opencode-project
---
```

---

### Tier 2: Core Instructions (The "Map")

**Loaded conditionally** when the model decides the skill is relevant (~500-1k tokens).

**File**: `SKILL.md`

**Structure**:

```markdown
<decision_matrix>
<question>What do you need?</question>

<option condition="Specific user intent">
  <navigate_to>references/comprehensive-guide.md</navigate_to>
  <description>What this reference covers</description>
</option>

<option condition="Different user intent">
  <navigate_to>references/other-guide.md</navigate_to>
  <description>What that covers</description>
</option>
</decision_matrix>

## Setup

# Commands for installation/verification

## Key Workflows

# Brief summaries of common patterns

## References

# Links to all Tier 3 files
```

**Purpose**: Teaches the model the skill's workflow and helps it navigate to Tier 3 resources.

**Decision Matrix Rules**:

- Use XML format: `<decision_matrix>`, `<option>`, `<navigate_to>`
- `<condition>` attributes describe **user intents**, not implementation details
- One option per distinct task domain (e.g., "Create skill", "Audit skill", "Validate skill")
- **Do not** split by file size or line counts
- **Do not** create multiple options for the same intent

**Anti-Pattern** ❌:

```xml
<option condition="Learn about progressive disclosure philosophy">
  <navigate_to>references/progressive-disclosure-overview.md</navigate_to>
</option>
<option condition="Learn about Tier 1 and Tier 2 structure">
  <navigate_to>references/progressive-disclosure-tiers-1-2.md</navigate_to>
</option>
<option condition="Learn about Tier 3 structure">
  <navigate_to>references/progressive-disclosure-tier-3.md</navigate_to>
</option>
```

**Best Practice** ✅:

```xml
<option condition="Design navigation structure for a skill (Tiers 1-3)">
  <navigate_to>references/progressive-disclosure-comprehensive-guide.md</navigate_to>
</option>
```

---

### Tier 3: Comprehensive References (The "Library")

**Loaded on-demand** when the model follows a `<navigate_to>` path.

**Contextual Granularity** (Optimizing for Semantic Purity):

The fundamental unit of agentic reasoning is the **Context Chunk**. Size is secondary to **Semantic Purity**—the degree to which a file represents a single, coherent concept.

| Content Domain               | Recommended Threshold  | Characteristics                                                              | Agent Implication                                                          |
| :--------------------------- | :--------------------- | :--------------------------------------------------------------------------- | :------------------------------------------------------------------------- |
| **Technical Documentation**  | **High Granularity**   | Separates API endpoints, error codes, distinct procedures.                   | High precision for "How-to" agents. Prevents mixing configuration options. |
| **Concepts / Methodologies** | **Medium Granularity** | Captures complete arcs of cause-and-effect (e.g., entire S.T.A.R. workflow). | Essential for reasoning agents needing full context.                       |
| **Reference Lists**          | **Low Granularity**    | consolidated lists of similar items (e.g., all error codes).                 | Allows single-pass lookup.                                                 |

**Granularity Decision Framework**:

| Agent Persona     | Task Nature       | Recommended Strategy     | Rationale                                               |
| :---------------- | :---------------- | :----------------------- | :------------------------------------------------------ |
| **Support Bot**   | Fact Retrieval    | **Atomic Files**         | Speed is priority.                                      |
| **Analyst Agent** | Complex Reasoning | **Comprehensive Guides** | Needs high context to understand nuances and caveats.   |
| **Coder Agent**   | Implementation    | **Pattern-Based**        | Splits based on implementation patterns, not text size. |

**When to Split Tier 3**:

- **Semantic Divergence**: The file covers multiple **unrelated domains** (e.g., "Testing" AND "Deployment" in one file).
- **Navigation Friction**: Users (or agents) frequently need _only_ the last 10% of the file, but must load the first 90%.
- **Cognitive Load**: The complexity of the topic exceeds the reasoning capacity when combined with other topics.

**When to Consolidate**:

- **Excessive Atomization** (Anti-Pattern):
  - 15+ reference files with average <150 lines.
  - Related content scattered across 5+ files (e.g., `auth-setup.md`, `auth-config.md`, `auth-testing.md`).
  - Arbitrary splits mid-concept (e.g., "Part 1", "Part 2").
- **Loss of Context**: Agents fail to understand the "why" because the "how" is in a different file.

**Semantic Cohesion Rule**:

> **Content drives topology, not line counts.**
>
> If a topic naturally requires a comprehensive guide to cover cause, effect, and implementation, **keep it together**. Do not split to meet artificial targets. Semantic boundaries > Size targets.

**Example**: `effect-ts` skill has comprehensive guides for `service-layer.md` and `error-handling.md` because splitting them would break the reasoning chain.

---

## Navigation Tags for JIT Loading

**Status**: Recommended for very large files (>2000 lines), but optional. Standard Markdown headers are sufficient for most cases.

**XML Format (Legacy/Power User)**:

```xml
<nav>
  <tag name="section-name" section="Section Title" />
</nav>
```

**Standard Markdown**:
Agents can naturally navigate to headers like `## Section Title`. Ensure your headers are descriptive.

---

## Decision Matrix Best Practices

### Condition Attributes

Describe **user intents**, not implementation details:

❌ **Bad**: "Learn about YAML frontmatter"

✅ **Good**: "Create a new skill from scratch"

### Option Granularity

One option per **task domain**, not per sub-topic:

❌ **Bad**:

```xml
<option condition="Create skill metadata">...</option>
<option condition="Create SKILL.md">...</option>
<option condition="Create references">...</option>
```

✅ **Good**:

```xml
<option condition="Create a new skill from scratch">
  <navigate_to>workflows-creation-star-workflow.md</navigate_to>
</option>
```

### Navigation Format

Use `<navigate_to>` tags **exclusively**:

❌ **Bad**:

```markdown
See @references/some-file.md
Navigate to @references/other-file.md
```

✅ **Good**:

```xml
<navigate_to>references/some-file.md</navigate_to>
```

### Internal Decision Matrices

Tier 3 files covering complex topics can have **their own** decision matrices:

```markdown
# Comprehensive Guide: Progressive Disclosure

<decision_matrix>
<question>What do you need to design?</question>

<option condition="Tier 1 metadata">
  <navigate_to>#tier-1-metadata</navigate_to>
</option>

<option condition="Tier 2 SKILL.md structure">
  <navigate_to>#tier-2-core-instructions</navigate_to>
</option>

<option condition="Tier 3 comprehensive references">
  <navigate_to>#tier-3-comprehensive-references</navigate_to>
</option>
</decision_matrix>

## Tier 1: Metadata

<nav><tag name="tier-1" section="Tier 1: Metadata" /></nav>
...
```

**Benefit**: Enables fine-grained navigation within large reference files.

---

## Quality Metrics

### Token Efficiency

| Architecture                  | Initial Load  | Task-Specific Load | Total Token Savings |
| ----------------------------- | ------------- | ------------------ | ------------------- |
| **No progressive disclosure** | 10,000 tokens | 0 tokens           | 0%                  |
| **3-tier progressive**        | 100 tokens    | 1,500 tokens       | **84% reduction**   |

### SkillScore Impact

| Metric                | Anti-Pattern | Best Practice | Improvement   |
| --------------------- | ------------ | ------------- | ------------- |
| **Semantic Cohesion** | 0.45-0.60    | 0.80-0.93     | +40-105%      |
| **Navigation Time**   | 3-5 min      | 30-60 sec     | 75-83% faster |
| **File Hops**         | 15+          | 3-5           | 70% reduction |

**Key Drivers**:

- Semantic cohesion (+0.12 SkillScore): Comprehensive guides vs. fragments
- Progressive disclosure (+8-15%): Tiered architecture with JIT loading
- Decision matrices (+10-20%): Clear navigation paths

---

## Anti-Patterns

### ❌ Excessive Atomization

**Symptoms**:

- 15 reference files with average <150 lines
- Multiple files covering the same topic
- Arbitrary splits to hit line count targets

**Example**:

```
references/
  ├── progressive-disclosure-overview.md (42 lines)
  ├── progressive-disclosure-tiers-1-2.md (46 lines)
  └── progressive-disclosure-tier-3.md (133 lines)
```

**Fix**:

```bash
# Consolidate into one comprehensive guide
references/
  └── progressive-disclosure-comprehensive-guide.md (600+ lines)
```

**Detection** (automated in `audit_degraded_skill.py`):

```python
if reference_file_count > 15 and avg_file_size < 150:
    flag("EXCESSIVE ATOMIZATION")
    recommend("Consolidate into comprehensive guides (600-1000 lines)")
```

### ❌ Arbitrary Splits

**Symptoms**:

- Breaking concepts mid-flow
- "Part 1", "Part 2", "Part 3" without semantic boundaries
- Splits to meet size targets rather than content domains

**Example**:

```
references/
  ├── authentication-part-1.md (200 lines) - Setup
  ├── authentication-part-2.md (180 lines) - Implementation
  └── authentication-part-3.md (220 lines) - Testing
```

**Fix**:

```bash
# Consolidate into semantic domain
references/
  └── authentication-complete-guide.md (600 lines)
  # Sections: Setup, Implementation, Testing (with <nav> tags)
```

### ❌ Missing Decision Matrix

**Symptoms**:

- SKILL.md lacks `<decision_matrix>` section
- References listed but not navigable
- Model doesn't know which file to read for specific tasks

**Fix**: Always include decision matrix in SKILL.md (Tier 2).

### ❌ Deprecated Navigation Syntax

**Symptoms**:

```markdown
See @references/some-file.md
Navigate to @references/other-file.md
```

**Fix**: Use XML `<navigate_to>` tags exclusively.

---

## Validation Checklist

Use this checklist when creating or refactoring skills:

### Tier 1 (Metadata)

- [ ] YAML frontmatter present (`---` delimiters)
- [ ] `name` field: hyphen-case, matches folder name
- [ ] `description` field: <200 characters, includes trigger keywords
- [ ] `compatibility` field: `opencode` or `opencode-project`

### Tier 2 (SKILL.md)

- [ ] Decision matrix present with XML format
- [ ] `<option>` conditions describe user intents (not implementation)
- [ ] `<navigate_to>` tags used exclusively (no `@references/` in text)
- [ ] Setup section with installation/verification commands
- [ ] File size: 100-800 lines
- [ ] All references exist in `references/` directory

### Tier 3 (References)

- [ ] File sizes: 200-1000 lines (ideal: 600-1000)
- [ ] No excessive atomization (<15 files, avg >150 lines)
- [ ] No micro-atomization (no files <50 lines except utilities)
- [ ] Semantic cohesion: related content grouped, not scattered
- [ ] `<nav>` tags present for sections (if >5 sections)
- [ ] Internal decision matrices for complex topics (>3 subsections)

### Navigation

- [ ] No broken references (all `<navigate_to>` paths valid)
- [ ] No circular references
- [ ] Clear hierarchy (1-3 hops to any content)
- [ ] Deprecated syntax absent (no "See @", "Navigate to @")

---

## Migration Guide: Fixing Excessive Atomization

### Step 1: Audit

```bash
# Run degradation audit
python .claude/skills/skill-creator/scripts/audit_degraded_skill.py

# Look for:
# - EXCESSIVE ATOMIZATION warnings
# - File count > 15
# - Average file size < 150 lines
```

### Step 2: Identify Semantic Domains

Group related files by semantic topic:

```bash
# List all reference files with sizes
find references/ -name "*.md" -exec wc -l {} + | sort -n

# Identify patterns:
# - Are there 3-5 files covering the same topic?
# - Do files reference each other heavily?
# - Are there "Part 1", "Part 2" splits?
```

### Step 3: Consolidate

```bash
# Merge related files into one comprehensive guide
cat references/part-1.md references/part-2.md references/part-3.md \
  > references/comprehensive-guide.md

# Add <nav> tags for section navigation
# Add internal decision matrix if >3 major sections
```

### Step 4: Update Navigation

```xml
<!-- In SKILL.md, replace multiple options -->
<option condition="Learn about X">
  <navigate_to>references/x-part-1.md</navigate_to>
</option>
<option condition="Learn about X (continued)">
  <navigate_to>references/x-part-2.md</navigate_to>
</option>

<!-- With single option -->
<option condition="Learn about X">
  <navigate_to>references/x-comprehensive-guide.md</navigate_to>
</option>
```

### Step 5: Verify

```bash
# Re-run audit
python .claude/skills/skill-creator/scripts/audit_degraded_skill.py

# Validate quality gates
python .claude/skills/skill-creator/scripts/validate_skill.py

# Calculate SkillScore (should improve by 10-20 points)
python .claude/skills/skill-creator/scripts/skillscore_evaluator.py
```

---

## Examples from Portable Skills

### Example 1: E-Commerce Platform (Best Practice)

```
ecommerce-platform/
  references/
    ├── 01-overview.md (200 lines)
    ├── 02-product-catalog.md (850 lines) - Complete domain
    ├── 03-cart-service.md (750 lines) - Complete domain
    ├── 04-payment-gateway.md (900 lines) - Complete domain
    └── 05-order-management.md (650 lines) - Complete domain
```

**Why this works**:

- Each file covers a **complete semantic domain** (catalog, cart, payments)
- Sizes follow natural content needs (650-900 lines)
- No arbitrary splits (e.g., `payment-gateway-setup.md`, `payment-gateway-config.md`)
- Clear decision matrix in SKILL.md

### Example 2: Authentication Framework (Good)

```
auth-framework/
  references/
    ├── oauth-flows.md (450 lines)
    ├── jwt-structure.md (350 lines)
    └── mfa-strategies.md (500 lines)
```

**Why this works**:

- Each file covers one **complete concept**
- Sizes are smaller (<600 lines) because concepts are naturally concise
- No atomization - each concept is self-contained

### Example 3: Utils Library (Anti-Pattern Fixed)

**Before**:

```
references/
  ├── string-utils-overview.md (42 lines)
  ├── string-utils-formatting.md (55 lines)
  ├── string-utils-validation.md (65 lines)
  └── string-utils-regex.md (133 lines)
```

**After**:

```
references/
  └── string-utils-comprehensive-guide.md (300 lines)
```

**Result**:

- **Semantic Cohesion**: Users see all string utilities in one place
- **Navigation**: 1 file load instead of 4
- **Context**: Model sees validation alongside formatting, improving usage patterns

---

## Advanced: Hybrid Approaches

For very large topics (>2000 lines), use **internal progressive disclosure**:

```markdown
# Comprehensive Guide: Very Large Topic

<nav>
  <tag name="section-1" section="Section 1: Foundation" />
  <tag name="section-2" section="Section 2: Advanced" />
  <tag name="section-3" section="Section 3: Edge Cases" />
</nav>

<decision_matrix>
<question>What aspect of Very Large Topic do you need?</question>

<option condition="Foundation and basics">
  <navigate_to>#section-1</navigate_to>
</option>

<option condition="Advanced patterns">
  <navigate_to>#section-2</navigate_to>
</option>

<option condition="Edge case handling">
  <navigate_to>#section-3</navigate_to>
</option>
</decision_matrix>

## Section 1: Foundation {#section-1}

<nav><tag name="section-1" /></nav>
... (600 lines)

## Section 2: Advanced {#section-2}

<nav><tag name="section-2" /></nav>
... (700 lines)

## Section 3: Edge Cases {#section-3}

<nav><tag name="section-3" /></nav>
... (500 lines)
```

**Benefits**:

- Single file (portability)
- JIT section loading (token efficiency)
- Clear navigation (usability)

**When to use**:

- Topic exceeds 1500 lines
- Clear section boundaries exist
- Sections are frequently accessed independently

---

## FAQ

### Q: Should I split a 1200-line file into two 600-line files?

**A**: No. If the content is semantically cohesive, keep it together. The 600-1000 line guideline is a **target for ideal comprehensiveness**, not a hard maximum. Split only if:

- File exceeds 1500 lines AND
- Natural section boundaries exist AND
- Navigation becomes difficult despite `<nav>` tags

### Q: What if my reference is only 300 lines? Is it too small?

**A**: Not necessarily. If it covers a **complete semantic domain** (e.g., COSTAR framework), 300 lines is fine. The anti-pattern is **arbitrary splitting**, not small files per se.

**Warning signs**:

- "Part 1", "Part 2" without semantic boundaries
- Related content scattered across 5+ files
- Average file size <150 lines across >15 files

### Q: How many sections should a Tier 3 guide have?

**A**: As many as needed for semantic clarity. Use `<nav>` tags for >5 sections. Use internal decision matrices for >3 major sections.

### Q: Can I have nested decision matrices?

**A**: Yes. Tier 3 files can have their own decision matrices (see "Advanced: Hybrid Approaches"). SKILL.md has the top-level matrix; references have section-level matrices.

### Q: Do I need progressive disclosure for small skills?

**A**: Yes, even small skills benefit. A 3-file skill still saves tokens by loading only the relevant reference. The architecture scales down naturally.

---

## Summary

**Progressive disclosure is about JIT loading, not arbitrary splits**:

1. **Tier 1** (Metadata): Always loaded (~100 tokens)
2. **Tier 2** (SKILL.md): Loaded when skill is relevant (~500-1k tokens)
3. **Tier 3** (References): Loaded on-demand via `<navigate_to>` tags

**Key principles**:

- **Semantic cohesion > File count**: One 800-line guide > five 160-line fragments
- **Content drives size**: Don't split to hit targets; split when semantically necessary
- **Decision matrices everywhere**: SKILL.md (top-level) + references (section-level)
- **Navigation tags for JIT loading**: Use `<nav>` tags in large Tier 3 files

**Anti-patterns to avoid**:

- Excessive atomization (>15 files, avg <150 lines)
- Arbitrary splits (mid-concept breaks)
- Missing decision matrices
- Deprecated navigation syntax

**Quality targets**:

- SkillScore: >90/100
- Semantic cohesion: >0.80
- Navigation time: <60 seconds
- File hops: <5 to reach any content

---

## See Also

- [Creation: S.T.A.R. Method](creation-star-method.md) - How to create skills with progressive disclosure
- [Refactoring: Bottom-Up Reconstruction](refactoring-bottom-up-reconstruction.md) - Detect excessive atomization and consolidate fragmented content
- [Validation: Quality Gates](validation-quality-gates.md) - Boolean gates and SkillScore metrics
- [Navigation: Tags Specification](navigation-tags-specification.md) - `<nav>` tag format for JIT loading
