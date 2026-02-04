<atom>
  <type>workflow</type>
  <diataxis_quadrant>how-to</diataxis_quadrant>
  <intent>refactor_degraded_skill_comprehensive</intent>
</atom>

# Bottom-Up Reconstruction: Complete Workflow for Skill Refactoring

<nav>
  <tag name="audit-phase" section="Phase 1: Audit" />
  <tag name="extract-phase" section="Phase 2: Extract" />
  <tag name="rebuild-phase" section="Phase 3: Rebuild" />
  <tag name="validate-phase" section="Phase 4: Validate" />
  <tag name="decision-matrices" section="Decision Matrices" />
</nav>

## Context

**What This Is**: End-to-end workflow for refactoring degraded skills (SkillScore < 85) using Bottom-Up Reconstruction methodology. Covers audit → extract → rebuild → validate cycle with decision matrices for each phase.

**When to Use**:

- Skill has quality score < 85
- Excessive atomization detected (> 15 files, avg < 150 lines)
- Content drift or outdated patterns
- Navigation complexity (users can't find relevant sections)

**Target Outcome**: SkillScore > 90, consolidated semantic guides, clear navigation paths

**Success Criteria**:

- SkillScore ≥ 90/100
- 0 boolean gate failures
- All files ≥ 100 lines (except pure technical specs)
- Clear decision matrix in SKILL.md

---

## Phase 1: Audit — Detect Problems

### Objective

Identify quality issues and calculate baseline SkillScore before reconstruction.

### Step 1.1: Run Automated Audit

```bash
# From repository root
python3 .claude/skills/skill-creator/scripts/audit_degraded_skill.py <skill-path>

# Example
python3 .claude/skills/skill-creator/scripts/audit_degraded_skill.py .claude/skills/effect-ts
```

**Expected Output**: Table showing file counts, line counts, and detected issues.

### Step 1.2: Manual Quality Assessment

**Checklist for Problem Detection**:

| Issue                       | Symptom                             | Detection Method | Severity |
| --------------------------- | ----------------------------------- | ---------------- | -------- |
| **Excessive atomization**   | > 15 files, avg < 150 lines         | Audit script     | HIGH     |
| **Micro-atomization**       | Files < 50 lines                    | File count       | HIGH     |
| **Arbitrary splits**        | Related content scattered           | Manual review    | MEDIUM   |
| **Missing decision matrix** | SKILL.md has no `<decision_matrix>` | SKILL.md check   | HIGH     |
| **Content drift**           | Outdated examples, broken links     | Manual review    | MEDIUM   |
| **Mixed Diátaxis modes**    | Tutorial mixed with reference       | Content analysis | MEDIUM   |

**Decision Matrix: Rebuild vs. Iterate**

| Condition                       | Action                 | Rationale                          |
| ------------------------------- | ---------------------- | ---------------------------------- |
| SkillScore < 80 OR > 15 files   | Full rebuild           | Too degraded for incremental fixes |
| SkillScore 80-85 AND < 10 files | Targeted consolidation | Fix specific issues                |
| SkillScore > 85                 | Minor cleanup          | No rebuild needed                  |
| Missing decision matrix         | Add matrix only        | Structural fix, not content issue  |

### Step 1.3: Calculate Baseline SkillScore

```bash
python3 .claude/skills/skill-creator/scripts/skillscore_evaluator.py <skill-path>/SKILL.md
```

**Record baseline** for post-rebuild comparison.

**IF** SkillScore < 80, **THEN** proceed to Phase 2 (Extract).
**ELIF** SkillScore 80-85, **THEN** assess if targeted consolidation suffices.
**ELSE** skip to Phase 4 (Validate) for minor cleanup.

---

## Phase 2: Extract — Preserve Patterns

### Objective

Extract all valuable content while identifying patterns for consolidation.

### Step 2.1: Inventory All Content

Create a manifest of existing files:

```markdown
## Content Inventory

| File               | Lines | Diátaxis Mode | Key Topics           | Keep? | Consolidate Into?               |
| ------------------ | ----- | ------------- | -------------------- | ----- | ------------------------------- |
| setup.md           | 120   | how-to        | Installation, config | Yes   | comprehensive-setup-guide.md    |
| auth-basics.md     | 85    | tutorial      | Auth concepts        | Yes   | auth-comprehensive-guide.md     |
| auth-advanced.md   | 95    | tutorial      | JWT, OAuth           | Yes   | auth-comprehensive-guide.md     |
| api-reference.md   | 300   | reference     | Endpoints            | Yes   | api-reference.md (standalone)   |
| troubleshooting.md | 45    | how-to        | Common issues        | No    | Migrate to comprehensive guides |
```

### Step 2.2: Identify Semantic Domains

Group related content by intent:

**Example Semantic Domains**:

```yaml
Authentication Domain:
  - auth-basics.md
  - auth-advanced.md
  - auth-troubleshooting.md
  → Consolidate into: auth-comprehensive-guide.md (target: 700-800 lines)

Testing Domain:
  - unit-testing.md
  - integration-testing.md
  - test-helpers.md
  → Consolidate into: testing-comprehensive-guide.md (target: 600-700 lines)
```

**Decision Matrix: Consolidation Criteria**

| Pattern                              | Action                           | Target Size        |
| ------------------------------------ | -------------------------------- | ------------------ |
| 3+ files on same topic               | Merge into comprehensive guide   | 800-1500 lines     |
| 2 related files < 300 lines combined | Merge                            | 300-600 lines      |
| Single file > 1000 lines             | Split if natural boundary exists | 500-700 lines each |
| Pure reference (API, config)         | Keep standalone                  | 300-600 lines      |

### Step 2.3: Extract Good Patterns

**Preserve These**:

- Code examples (especially real-world usage)
- Decision matrices and flowcharts
- Quality checklists
- Navigation tags (`<nav>` blocks)

**Discard These**:

- Outdated information
- Redundant explanations
- Dead links or broken references
- Filler content ("please", "thank you", verbose intros)

### Step 2.4: Mark Extraction Complete

**Checkpoint**: Before proceeding to Phase 3, verify:

- [ ] All valuable content identified
- [ ] Semantic domains mapped
- [ ] Consolidation plan documented
- [ ] No orphaned content (everything assigned a destination)

---

## Phase 3: Rebuild — Consolidate Content

### Objective

Reorganize content into comprehensive guides using semantic cohesion.

### Step 3.1: Create Comprehensive Guides

**Size Guidelines**:

| Content Type           | Target Size     | Rationale                             |
| ---------------------- | --------------- | ------------------------------------- |
| Multi-pattern how-to   | 800-1200 lines  | Cover related tasks in semantic unit  |
| End-to-end tutorial    | 1000-1500 lines | Full learning journey with exercises  |
| Conceptual explanation | 500-900 lines   | Deep dive with examples and context   |
| API reference          | 300-600 lines   | Factual material, no narrative needed |

**Philosophy**: "Semantic Density over Line Counts."

- **Goal**: Minimize context fragmentation.
- **Guideline**: Line counts are targets, not strict laws.
- **Exception**: Atomic reference lists (e.g., error codes) should remain atomic.

### Step 3.2: Structure Each Comprehensive Guide

**Template**:

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
</nav>

## Overview

[Brief context: what this covers, who it's for, 3-5 sentences max]

## Prerequisites

[Required knowledge, setup steps, 3-5 bullet points]

## Core Concepts

[Main content: explanations, patterns, decisions]

## Common Patterns

[Reusable patterns with examples]

## Real-World Examples

[2-3 complete examples from production code]

## Troubleshooting

[Common issues and solutions, table format]

## See Also

[Related guides with links]
```

### Step 3.3: Add Navigation Tags

**For guides > 300 lines**, include `<nav>` tags for JIT section loading:

```xml
<nav>
  <tag name="section-name" section="Section Title" />
</nav>
```

**Adaptive Purity Rule**:

- Files < 300 lines: NO internal links (pure atoms)
- Files > 300 lines: Internal navigation permitted and encouraged

### Step 3.4: Update SKILL.md Decision Matrix

**Template**:

```markdown
## Decision Matrix

<decision_matrix>
<question>What task do you need?</question>

<option condition="Specific user intent">
  <navigate_to>references/comprehensive-guide.md</navigate_to>
  <description>Clear description of what this covers</description>
</option>

<option condition="Another user intent">
  <navigate_to>references/another-comprehensive-guide.md</navigate_to>
  <description>Clear description</description>
</option>
</decision_matrix>
```

**Decision Matrix: SKILL.md Structure**

| Skill Size         | SKILL.md Structure                                       | Reference Links       |
| ------------------ | -------------------------------------------------------- | --------------------- |
| Small (≤ 3 files)  | Decision matrix + setup commands                         | Direct links          |
| Medium (4-7 files) | Decision matrix + workflows + common patterns            | Categorized by intent |
| Large (8+ files)   | Decision matrix + workflows + anti-patterns + references | Semantic grouping     |

### Step 3.5: Verify Semantic Cohesion

**Checklist**:

- [ ] Related concepts grouped together (not scattered)
- [ ] Natural section boundaries (no arbitrary breaks)
- [ ] Each guide covers a single semantic domain
- [ ] No content duplication across guides
- [ ] Navigation paths clear from decision matrix

**IF** content feels forced or split awkwardly, **THEN** reconsider consolidation strategy.

---

## Phase 4: Validate — Quality Gates

### Objective

Verify rebuilt skill meets production-ready standards.

### Step 4.1: Run Boolean Gates

```bash
python3 .claude/skills/skill-creator/scripts/validate_skill.py <skill-path>
```

**Expected**: All 10 gates PASS

**Common Failures and Fixes**:

| Gate                    | Failure Mode                       | Fix                                  |
| ----------------------- | ---------------------------------- | ------------------------------------ |
| `has_skill_md`          | No SKILL.md in root                | Create SKILL.md with decision matrix |
| `has_references_dir`    | No references/ directory           | Create references/ structure         |
| `has_decision_matrix`   | No `<decision_matrix>` in SKILL.md | Add decision matrix                  |
| `all_files_markdown`    | Non-.md files in references/       | Convert to .md or remove             |
| `no_dead_links`         | Broken internal links              | Update link paths                    |
| `atom_metadata_present` | Files missing `<atom>` block       | Add metadata to each file            |
| `file_size_minimum`     | Files < 100 lines                  | Consolidate with related content     |
| `max_file_size`         | Files > 2000 lines                 | Split at natural boundaries          |
| `skillscore_threshold`  | SkillScore < 80                    | Improve content quality              |
| `nav_tags_valid`        | Invalid `<nav>` syntax             | Fix XML format                       |

### Step 4.2: Calculate Final SkillScore

```bash
python3 .claude/skills/skill-creator/scripts/skillscore_evaluator.py <skill-path>/SKILL.md
```

**Success Criteria**:

- SkillScore ≥ 90/100 (Production Ready)
- SkillScore 85-89/100 (Good Quality)
- SkillScore 80-84/100 (Acceptable)
- SkillScore < 80/100 (Needs Improvement - BLOCKING)

**IF** SkillScore < 90, **THEN** proceed to Step 4.3 (Iterate).

### Step 4.3: Iteration Cycle (If Needed)

**Decision Matrix: Fix Strategy**

| Issue                 | Fix Type                    | Example                                 |
| --------------------- | --------------------------- | --------------------------------------- |
| Low semantic cohesion | Merge related files         | Combine auth patterns into single guide |
| Poor navigation       | Add decision matrix entries | Create paths for common intents         |
| Weak examples         | Add real-world code         | Replace placeholder with actual usage   |
| Missing metadata      | Add `<atom>` blocks         | Include Diátaxis quadrant and intent    |

**Iteration Process**:

1. **Identify lowest SkillScore component** (e.g., Navigation = 6/10)
2. **Apply targeted fix** (e.g., add decision matrix entries)
3. **Re-run validation** (boolean gates + SkillScore)
4. **Repeat** until SkillScore ≥ 90

**Max iterations**: 3. If not improving after 3 attempts, reconsider architecture.

### Step 4.4: Before/After Comparison

**Example Transformation**:

| Metric          | Before    | After      | Improvement |
| --------------- | --------- | ---------- | ----------- |
| Files           | 18        | 7          | -61%        |
| Avg lines/file  | 97        | 312        | +222%       |
| SkillScore      | 72/100    | 93/100     | +29%        |
| Boolean gates   | 7/10 PASS | 10/10 PASS | +43%        |
| Navigation time | 4.2 min   | 0.8 min    | -81%        |

---

## Decision Matrices

### Matrix 1: Rebuild Readiness

| Condition                                        | Action                                   |
| ------------------------------------------------ | ---------------------------------------- |
| SkillScore < 80 OR > 15 files OR avg < 150 lines | Full rebuild (all phases)                |
| SkillScore 80-85 AND < 10 files                  | Targeted consolidation (skip to Phase 3) |
| SkillScore > 85                                  | Minor cleanup (skip to Phase 4)          |
| Missing decision matrix only                     | Add matrix (skip content rebuild)        |

### Matrix 2: Consolidation Strategy

| Current State                            | Target State               | Method                       |
| ---------------------------------------- | -------------------------- | ---------------------------- |
| 3+ files on same topic, each < 300 lines | Single comprehensive guide | Merge all, add navigation    |
| 2 related files, combined < 300 lines    | Single focused guide       | Merge with minimal expansion |
| Single file > 1200 lines                 | Split into 2-3 guides      | Find natural boundaries      |
| Reference material (API, config)         | Keep standalone            | No consolidation needed      |

### Matrix 3: File Size Targets

| Diátaxis Quadrant | Min Size | Target Size | Max Size | Split If                   |
| ----------------- | -------- | ----------- | -------- | -------------------------- |
| how-to            | 300      | 800-1200    | 2000     | > 2000 with clear boundary |
| tutorial          | 400      | 800-1200    | 1500     | > 1500 with clear boundary |
| explanation       | 300      | 500-900     | 1100     | > 1100 with clear boundary |
| reference         | 200      | 300-600     | 800      | > 800 with clear sections  |

---

## Anti-Patterns to Avoid

❌ **Aggressive Atomization**: Splitting 800-line guide into 10 x 80-line fragments

- **Impact**: Lost semantic context, excessive navigation
- **Fix**: Consolidate into comprehensive guide

❌ **Arbitrary Splits**: Breaking mid-concept to hit line targets

- **Impact**: Incomplete explanations, disrupted learning
- **Fix**: Follow natural semantic boundaries

❌ **Separating Related Patterns**: Scattering related content across files

- **Impact**: Poor user experience, reduced comprehension
- **Fix**: Group by semantic domain

❌ **Dogmatic Monoliths**: Forcing distinct concepts into one file to hit 1500 lines

- **Impact**: Token bloat, hard to maintain
- **Fix**: Split if semantic cohesion is low

✅ **Semantic Cohesion**: Beefy comprehensive guides covering related patterns
✅ **Navigation Tags**: JIT section loading for large guides
✅ **Decision Matrix**: Clear routing based on user intent
✅ **Adaptive Purity**: Small files link-free, large files with navigation

---

## Validation Checklist

**Pre-Rebuild**:

- [ ] Audit script executed and results recorded
- [ ] Baseline SkillScore calculated
- [ ] Content inventory complete
- [ ] Semantic domains identified

**Post-Rebuild**:

- [ ] All 10 boolean gates PASS
- [ ] SkillScore ≥ 90/100
- [ ] No files < 100 lines (except pure specs)
- [ ] Decision matrix present and functional
- [ ] Navigation tags for files > 300 lines
- [ ] No dead links or broken references
- [ ] Before/after comparison shows improvement

---

## See Also

- [Validation: Quality Gates](validation-quality-gates.md) - Detailed boolean gate specifications
- [Progressive Disclosure: Tiers 1-3](progressive-disclosure-comprehensive-guide.md) - Size guidelines and architecture
- [Navigation: Tags Specification](navigation-tags-specification.md) - JIT loading format
- [Workflow: S.T.A.R. Creation](workflow-skill-creation-comprehensive-guide.md) - Creating new skills from scratch
