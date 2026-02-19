# Quality Gates & SkillScore

<atom>
  <type>reference</type>
  <diataxis_quadrant>reference</diataxis_quadrant>
  <intent>validate_quality</intent>
</atom>

<internal_pattern>
<description>
Dual-layer validation framework for ensuring skill quality. Updated to reward semantic cohesion and beefy comprehensive documents over fragmented atoms.
</description>

## Layer 1: Boolean Gates (Hard Blocks)

These are non-negotiable structural requirements.

| Gate ID                 | Condition                               | Rationale                 |
| :---------------------- | :-------------------------------------- | :------------------------ |
| `has_skill_md`          | `SKILL.md` exists in root               | Entry point is mandatory  |
| `has_references_dir`    | `references/` directory exists          | Separation of concerns    |
| `has_decision_matrix`   | `SKILL.md` contains `<decision_matrix>` | Navigation logic required |
| `all_files_markdown`    | `references/*.md` (no other extensions) | Agents read MD best       |
| `no_dead_links`         | All `[link](path)` resolve              | Broken paths = failure    |
| `atom_metadata_present` | All files have `<atom>` block           | Semantic classification   |

**Note**: Line count checks and `<nav>` tag validation have been moved to Layer 1.5 (Warnings).

## Layer 1.5: Structural Warnings (Soft Checks)

These generate warnings but do not block usage.

| Check               | Warning Condition                           | Rationale                    |
| :------------------ | :------------------------------------------ | :--------------------------- |
| `file_size_minimum` | File < 100 lines (and not "reference" type) | Micro-atomization risk       |
| `max_file_size`     | File > 2000 lines                           | Context window flooding risk |
| `nav_tags_valid`    | Invalid XML in `<nav>` (if present)         | Broken custom navigation     |

## Layer 2: SkillScore Metrics (Rubric Evaluation)

We use **SkillScore** (0-100), a rubric-based metric derived from AGENTIF/AdvancedIF research, replacing the retrieval-based SkillScore.

| Category        | Weight  | Criteria                                      | Why it matters                                         |
| :-------------- | :------ | :-------------------------------------------- | :----------------------------------------------------- |
| **Navigation**  | **40%** | `<decision_matrix>`, Headers, Tags            | Agents need structural anchors to reduce search space. |
| **Determinism** | **30%** | Imperative verbs (`MUST`, `STOP`, `CRITICAL`) | Commanding language increases adherence by ~40%.       |
| **Safety**      | **15%** | Negative constraints (`DO NOT`, `NEVER`)      | Explicit boundaries are required for safe operation.   |
| **Cohesion**    | **15%** | Semantic density (50-2000 lines)              | Avoids fragmentation and context flooding.             |

### Scoring Targets

| Status                | SkillScore | Action                                   |
| :-------------------- | :--------- | :--------------------------------------- |
| **Production Ready**  | **≥ 90**   | Ship it.                                 |
| **Good Quality**      | **80-89**  | Acceptable for internal use.             |
| **Needs Improvement** | **< 80**   | **BLOCKING**. Must improve before merge. |

### How to Improve Score

| Low Score Area  | Fix                                                                        |
| :-------------- | :------------------------------------------------------------------------- |
| **Navigation**  | Add `<decision_matrix>` to SKILL.md. Add Table of Contents to large files. |
| **Determinism** | Change "You should..." to "You MUST...". Add explicit output templates.    |
| **Safety**      | Add a "Negative Constraints" section. Use warning blocks.                  |
| **Cohesion**    | Merge tiny files (<100 lines). Split monoliths (>2000 lines).              |

---

## Quality Gate Examples

**Example 1: Over-Atomized Skill** (POOR)

```yaml
Structure:
  - 25 reference files
  - Average 120 lines per file
  - Related content scattered across 8+ files
  
SkillScore Calculation:
  Question-Snippet Quality: 75/100 (confusing navigation)
  Initialization Score: 85/100 (setup OK)
  Semantic Cohesion: 0.52 (base) - 0.15 (excessive atomization) = 0.37
  Instructional Clarity: 0.82 (imperative voice OK)
  Reasoning Success: 85% (some queries fail due to fragmentation)
  Atomic Purity: 0.95 (atoms independent)
  
SkillScore = (75×0.15) + (85×0.15) + (37×0.25) + (82×0.15) + (85×0.20) + (95×0.10)
        = 11.25 + 12.75 + 9.25 + 12.3 + 17.0 + 9.5
        = 72.05/100 (POOR)
        
Recommendation: Consolidate into 3-5 comprehensive guides
Expected SkillScore after consolidation: 88-92/100
```

**Example 2: Comprehensive Cohesive Skill** (EXCELLENT)

```yaml
Structure:
  - 4 comprehensive reference files
  - Average 850 lines per file
  - Semantically unified domains
  - Navigation tags for JIT loading
  
SkillScore Calculation:
  Question-Snippet Quality: 92/100 (clear navigation)
  Initialization Score: 95/100 (excellent setup)
  Semantic Cohesion: 0.84 (base) + 0.12 (comprehensive) + 0.08 (nav tags) = 0.93
  Instructional Clarity: 0.94 (excellent imperative voice)
  Reasoning Success: 98% (golden queries pass)
  Atomic Purity: 0.97 (references independent)
  
SkillScore = (92×0.15) + (95×0.15) + (93×0.25) + (94×0.15) + (98×0.20) + (97×0.10)
        = 13.8 + 14.25 + 23.25 + 14.1 + 19.6 + 9.7
        = 94.7/100 (EXCELLENT)
        
Status: Ready for production
```

---

## When to Use Quality Gates

```yaml
 ✅ Complex skills:
   Validate 3-tier progressive disclosure structure
   Check semantic cohesion across domains
   Verify navigation tags for JIT loading

✅ Multi-reference skills:
  Ensure semantic independence and no orphaned files
  Validate comprehensive guide structure (files >= 300 lines)
  Check for excessive atomization and ZERO nav violations on small files

✅ Simple skills:
  Use simplified checks (no atoms = skip atom validation)
  Focus on decision matrix and setup quality
```

---

## Validation Commands

```bash
# Run boolean quality gates
python .claude/skills/skill-creator/scripts/validate_skill.py <skill_path> --boolean

# Calculate SkillScore with detailed metrics
python .claude/skills/skill-creator/scripts/skillscore_evaluator.py <skill_path>/SKILL.md

# Audit for excessive atomization
python .claude/skills/skill-creator/scripts/audit_degraded_skill.py <skill_path>

# Quick validation (structure only)
python .claude/skills/skill-creator/scripts/quick_validate.py <skill_path>
```

---

## Target Scores

```yaml
Production Ready:
  SkillScore: > 90/100
  Semantic Cohesion: > 0.8
  Boolean Gates: 10/10 PASS
  Reasoning Success: 100%

Good Quality:
  SkillScore: 80-90/100
  Semantic Cohesion: > 0.7
  Boolean Gates: 9/10 PASS
  Reasoning Success: > 95%

Needs Improvement:
  SkillScore: < 80/100
  Semantic Cohesion: < 0.7
  Boolean Gates: < 9/10 PASS
  Reasoning Success: < 95%
  
Common Issue: Excessive atomization
Fix: Consolidate fragmented content into comprehensive guides
Expected Improvement: +15-25 SkillScore points
```

---

## TDD Validation Workflow

<atom>
  <type>workflow</type>
  <diataxis_quadrant>how-to</diataxis_quadrant>
  <intent>validate_logic</intent>
</atom>

Red-Green-Refactor cycle for validating skill reasoning capabilities using Golden Queries.

### Phase 1: Red (Fail)

Create 18-20 "Golden Queries" (User questions). Run against current skill. Confirm failure or low quality.

### Phase 2: Green (Pass)

Implement missing atoms and navigation logic. Re-run validation until 100% pass rate.

### Phase 3: Refactor (Optimize)

Optimize token usage (Progressive Disclosure) and SkillScore rubric metrics without breaking Golden Queries.

---

## See Also

- [Workflow: Validate & Iterate](workflow-reconstruction-validate.md) - Fix violations
- [Progressive Disclosure: Comprehensive Guide](progressive-disclosure-comprehensive-guide.md) - Size guidelines and tier structure
- [Workflow: Audit & Extract](workflow-reconstruction-audit-extract.md) - Identify problems
