---
name: skill-creator
description: "Use when creating, refactoring, or validating OpenCode skills. Provides S.T.A.R. workflow, Bottom-Up Reconstruction, and Quality Gates. Semantic cohesion: Contextual granularity with dynamic context pruning."
license: MIT
compatibility: opencode
---

## Quick Decision Matrix

<decision_matrix>
<question>What do you need to accomplish?</question>

<option condition="Create a new skill from scratch">
  <navigate_to>creation-star-method.md</navigate_to>
  <description>Complete S.T.A.R. workflow (Setup → Task → Analysis → Result) for creating production-ready skills with SkillScore > 90</description>
</option>

<option condition="Refactor or rebuild a degraded skill (quality score < 85)">
  <navigate_to>refactoring-bottom-up-reconstruction.md</navigate_to>
  <description>End-to-end workflow: Audit → Extract → Rebuild → Validate. Consolidate fragmented content into comprehensive guides (800-1500 lines)</description>
</option>

<option condition="Design navigation structure for a skill">
  <navigate_to>progressive-disclosure-comprehensive-guide.md</navigate_to>
  <description>Progressive disclosure architecture: 3-tier system (metadata → instructions → references) with JIT loading for 70%+ token savings</description>
</option>

<option condition="Validate skill quality (SkillScore, boolean gates)">
  <navigate_to>validation-quality-gates.md</navigate_to>
  <description>Quality gates: Layer 1 (Boolean checks), Layer 1.5 (C.L.E.A.R. principles), Layer 2 (SkillScore metrics)</description>
</option>

<option condition="Implement navigation tags and JIT loading syntax">
  <navigate_to>navigation-tags-specification.md</navigate_to>
  <description>Specification for XML navigation tags, JIT loading patterns, and validation rules</description>
</option>

<option condition="Determine if content should be portable or project-specific">
  <navigate_to>references/portable-vs-project-specific-decision-framework.md</navigate_to>
  <description>Comprehensive framework with three-factor test, binary checklist, and 7 canonical examples to distinguish generic guidance from monorepo-specific content</description>
</option>
</decision_matrix>

---

## Setup

```bash
# Verify Python 3.8+ is installed
python --version

# Install dependencies
pip install pyyaml rich

# Create a new skill
python .claude/skills/skill-creator/scripts/init_skill.py my-skill

# Validate skill quality
python .claude/skills/skill-creator/scripts/validate_skill.py .claude/skills/my-skill

# Calculate SkillScore (Tier 1: Fast static analysis)
python .claude/skills/skill-creator/scripts/skillscore_evaluator.py .claude/skills/my-skill/SKILL.md
```

## Quantitative Impact

| Metric         | Anti-Pattern | Best Practice | Improvement       |
| -------------- | ------------ | ------------- | ----------------- |
| **SkillScore** | 55-70        | 88-95         | +20-35            |
| **Cohesion**   | 0.45-0.60    | 0.80-0.93     | +40-105%          |
| **Nav Time**   | 3-5 min      | 30-60 sec     | **75-83% faster** |
| **File Hops**  | 15+          | 3-5           | **70% reduction** |

**Key Drivers**:

- Semantic cohesion (+0.12 SkillScore): Contextual granularity vs fragments
- Progressive disclosure (+8-15%): Tiered architecture with JIT loading
- Quality gates (+15-25): Boolean gates + SkillScore validation
- Portability (+10-15): Generic placeholders vs hardcoded paths

### About SkillScore

SkillScore uses a **two-tiered quality assessment** system:

- **Tier 1 (Fast)**: Static analysis of document structure (current implementation)
  - Validates syntax, structure, and keyword density
  - Speed: ~10ms per file
  - Use: Development iteration and CI/CD

- **Tier 2 (Deep)**: AdvancedIF-style rubric verification (planned)
  - Validates actual agent behavior using LLM-as-a-Judge
  - Speed: ~2-5s per file
  - Use: Production validation and scientific rigor

**Important**: Tier 1 is a **development tool**, not a scientific metric. For production validation, use Tier 2 (see `docs/skillscore-tier2-specification.md`).

See also: `docs/skillscore-red-team-analysis.md` for critical evaluation.

---

## Core Principles

**S.T.A.R.**: Structure → Target → Author → Review

**Bottom-Up**: Audit → Rebuild → Validate

## 🚨 MANDATORY PROTOCOL (NON-NEGOTIABLE)

1. **STOP**. Do not create skills without a S.T.A.R. plan.
2. **MUST** use the Decision Matrix for navigation.
3. **CRITICAL**: All files MUST have semantic cohesion.
4. **NEVER** use excessive atomization (>15 files).

## Applying Prompt Engineering Best Practices

When creating skills, apply **C.L.E.A.R.** principles from the `prompt-engineering` skill:

| Principle      | Application to Skills                                                                | Impact                      |
| -------------- | ------------------------------------------------------------------------------------ | --------------------------- |
| **Concise**    | Maximize signal-to-noise. Remove filler phrases ("please", "thank you").             | 70% token reduction         |
| **Logical**    | Follow COSTAR structure: Context → Task → Constraints → Format                       | +55% clarity                |
| **Explicit**   | Use objective metrics instead of vague terms ("brief" → "3 sentences, 50 words max") | 91% hallucination reduction |
| **Adaptive**   | Handle edge cases with IF/THEN logic. Provide tier-based guidelines.                 | +51% edge case handling     |
| **Reflective** | Include validation checklists and self-audit before completion.                      | 67% error reduction         |

**See Also**: `prompt-engineering` skill for complete framework details (Diátaxis, COSTAR, C.L.E.A.R., Chain-of-Thought, Decision Matrices, Few-Shot).

## Document Structure

### Tier 1: YAML Frontmatter (< 100 tokens)

```yaml
---
name: skill-name
description: "Concise description with keywords"
license: MIT
compatibility: opencode
---
```

### Tier 2: SKILL.md (100-800 lines)

**Purpose**: Decision matrix + setup commands

**Structure**:

- Quick Decision Matrix (XML format)
- Setup commands (bash)
- Key workflows and common patterns
- Reference links

### Tier 3: Comprehensive References (800-1500 lines)

**Purpose**: Deep dives with full semantic context

**Size Guidelines**:

| Tier       | Size Guidelines                             | Purpose                  |
| ---------- | ------------------------------------------- | ------------------------ |
| **Tier 3** | 800-1500 lines (when semantically cohesive) | Comprehensive deep dives |

**When to Split**:

- Exceeds 2000 lines AND natural section boundaries exist
- Covers multiple unrelated domains (e.g., testing + deployment)
- Creates navigation difficulties despite tags

**When to Consolidate**:

- Too many tiny files (> 15 files, avg < 150 lines)
- Arbitrary splits mid-concept
- Related content scattered across 5+ files

### Navigation Tags (for JIT Section Loading)

XML format for `<nav>` tags in reference files:

```xml
<nav>
  <tag name="section-name" section="Section Title" />
</nav>
```

YAML format for skill metadata:

```yaml
# Example in YAML frontmatter
tags:
  - authentication
  - error-handling
  - testing
```

## Key Workflows

### S.T.A.R. Workflow (Creation)

**S**etup → **T**ask → **A**nalysis → **R**esult

1. **Setup**: Structure the skill directory and SKILL.md skeleton
2. **Task**: Define user intents and semantic domains
3. **Analysis**: Design comprehensive guides (800-1500 lines) with navigation
4. **Result**: Validate with boolean gates and SkillScore > 90

See: [Workflow: Skill Creation Comprehensive Guide](references/workflow-skill-creation-comprehensive-guide.md)

### Bottom-Up Reconstruction (Refactoring)

For degraded skills needing rebuild:

1. **Audit**: Detect problems (excessive atomization, content drift)
2. **Extract**: Preserve valuable content and identify patterns
3. **Rebuild**: Consolidate into comprehensive guides
4. **Validate**: Quality gates (boolean gates + SkillScore)

See: [Workflow: Bottom-Up Reconstruction Comprehensive Guide](references/workflow-bottom-up-reconstruction-comprehensive-guide.md)

## Common Patterns

### Decision Matrix Structure

```markdown
<decision_matrix>
<question>What task do you need?</question>
<options>

<option condition="Specific user intent">
  <action><navigate_to>references/comprehensive-guide.md</navigate_to></action>
</option>
</options>
</decision_matrix>
```

### Atom Metadata

```xml
<atom>
  <type>pattern|workflow|reference</type>
  <diataxis_quadrant>how-to|tutorial|explanation|reference</diataxis_quadrant>
  <intent>specific_user_intent</intent>
</atom>
```

## Quality Targets

```yaml
Production Ready:
  SkillScore (Tier 1): > 90/100
  Boolean Gates: 10/10 PASS
  Reasoning Success: 100%

Good Quality:
  SkillScore (Tier 1): 80-90/100
  Boolean Gates: 9/10 PASS
  Reasoning Success: > 95%
```

## Two-Tiered Validation Architecture

We use a two-tiered approach to balance speed and scientific rigor:

| Tier       | Type                | Method                                              | Cost | Use Case                 |
| :--------- | :------------------ | :-------------------------------------------------- | :--- | :----------------------- |
| **Tier 1** | **Static Analysis** | `skillscore_evaluator.py` checks structure/keywords | Free | Dev loop (current)       |
| **Tier 2** | **Deep Verify**     | LLM-as-a-Judge checks semantic adherence            | $$$  | Release gating (planned) |

**Note**: High Tier 1 scores are necessary but not sufficient for Tier 2 success.

## Anti-Patterns

❌ **Excessive atomization**: > 15 files, avg < 150 lines
❌ **Arbitrary splits**: Breaking mid-concept to hit line targets
❌ **Missing decision matrix**: Users can't navigate
❌ **No setup section**: Users don't know how to use the skill

✅ **Semantic cohesion**: 800-1500 line comprehensive guides
✅ **Navigation tags**: Enable JIT section loading
✅ **Decision matrix**: Clear navigation paths
✅ **Setup section**: Prerequisites and commands

## References

- [Creation: S.T.A.R. Method](references/creation-star-method.md) - Complete workflow for creating production-ready skills from scratch
- [Refactoring: Bottom-Up Reconstruction](references/refactoring-bottom-up-reconstruction.md) - End-to-end workflow for refactoring degraded skills
- [Progressive Disclosure: Comprehensive Guide](references/progressive-disclosure-comprehensive-guide.md) - 3-tier architecture with JIT loading examples
- [Validation: Quality Gates](references/validation-quality-gates.md) - Boolean gates, SkillScore metrics, and TDD workflow
- [Navigation: Tags Specification](references/navigation-tags-specification.md) - JIT loading XML format specification
